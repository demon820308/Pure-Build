use crate::config::ConfigManager;
use crate::endpoints::{
    self, DEFAULT_ASSET_CONCURRENCY, DEFAULT_LIBRARY_CONCURRENCY,
};
use crate::launch::{LaunchLibrary, MinecraftLauncher};
use crate::platform;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use zip::ZipArchive;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub version_id: String,
    #[serde(default)]
    pub vanilla_version: Option<String>,
    pub step: String,
    pub percentage: f32,
    pub speed_text: String,
    pub is_finished: bool,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VersionDetailJson {
    downloads: Option<VersionDownloads>,
    #[serde(rename = "assetIndex")]
    asset_index: Option<AssetIndexMeta>,
}

#[derive(Debug, Deserialize)]
struct AssetIndexMeta {
    id: String,
    url: String,
}

#[derive(Debug, Deserialize)]
struct AssetIndexFile {
    objects: HashMap<String, AssetObjectEntry>,
}

#[derive(Debug, Deserialize)]
struct AssetObjectEntry {
    hash: String,
    size: u64,
}

#[derive(Debug, Deserialize)]
struct VersionDownloads {
    client: Option<DownloadArtifact>,
    server: Option<DownloadArtifact>,
}

#[derive(Debug, Deserialize)]
struct DownloadArtifact {
    url: String,
    size: u64,
}

#[derive(Debug, Deserialize)]
struct FabricLoaderEntry {
    loader: FabricLoaderInfo,
}

#[derive(Debug, Deserialize)]
struct FabricLoaderInfo {
    version: String,
    stable: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct LoaderProfileJson {
    id: String,
    #[serde(rename = "inheritsFrom")]
    inherits_from: Option<String>,
    #[serde(rename = "mainClass")]
    main_class: String,
    arguments: Option<serde_json::Value>,
    #[serde(rename = "minecraftArguments")]
    minecraft_arguments: Option<String>,
    libraries: Vec<serde_json::Value>,
}

#[derive(Debug, Clone)]
struct BatchDownloadTask {
    target_path: PathBuf,
    urls: Vec<String>,
    expected_size: Option<u64>,
    extract_natives: Option<(PathBuf, Option<Vec<String>>)>,
}

pub struct VersionInstaller;

impl VersionInstaller {
    /// 获取针对特定本地目标路径的异步互斥锁（对齐 PCL2 `NetManagerClass.AllFiles` 全局正在下载去重机制，防止多次点击或并发任务写同一文件冲突）
    fn acquire_file_lock(path: &Path) -> Arc<tokio::sync::Mutex<()>> {
        static FILE_LOCKS: std::sync::OnceLock<
            std::sync::Mutex<HashMap<PathBuf, Arc<tokio::sync::Mutex<()>>>>,
        > = std::sync::OnceLock::new();
        let map_mutex = FILE_LOCKS.get_or_init(|| std::sync::Mutex::new(HashMap::new()));
        let mut map = map_mutex.lock().unwrap();
        map.entry(path.to_path_buf())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    }

    /// 校验本地文件是否完整有效（对齐 PCL2 `FileChecker.Check`）：
    /// 1. 校验文件存在且大小匹配 `expected_size`（未指定时要求 > 0）；
    /// 2. 若扩展名为 `.jar` 或 `.zip`，额外校验 ZIP 魔数 `PK\x03\x04` 或 `PK\x05\x06`，防止网关 429/502 HTML 报错页或截断文件伪装成有效构件。
    pub fn is_file_valid(path: &Path, expected_size: Option<u64>) -> bool {
        let Ok(meta) = fs::metadata(path) else {
            return false;
        };
        if !meta.is_file() {
            return false;
        }
        let len = meta.len();
        if let Some(exp) = expected_size {
            if exp > 0 && len != exp {
                return false;
            }
        } else if len == 0 {
            return false;
        }

        let is_archive = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("jar") || ext.eq_ignore_ascii_case("zip"))
            .unwrap_or(false);

        if is_archive {
            if len < 4 {
                return false;
            }
            let Ok(mut f) = File::open(path) else {
                return false;
            };
            let mut magic = [0u8; 4];
            if f.read_exact(&mut magic).is_err() {
                return false;
            }
            if &magic != b"PK\x03\x04" && &magic != b"PK\x05\x06" {
                return false;
            }
        }

        true
    }

    /// 流式下载单个文件并原子落盘（对齐 PCL2 `NetFile` 下载状态机）：
    /// - 同路径全局互斥防冲突，已通过 `is_file_valid` 的文件直接秒过；
    /// - 针对 BMCLAPI 自动执行建连节流；
    /// - 支持多源轮询与两轮 `Retried` 完整重试，遇到 429/503 自动冷却等待；
    /// - 先写入 `.tmp` 临时文件，完成并通过 `is_file_valid` 校验后原子替换目标文件。
    pub async fn download_file_with_retry<F>(
        client: &reqwest::Client,
        urls: &[String],
        target_path: &Path,
        expected_size: Option<u64>,
        mut on_progress: F,
    ) -> Result<(), String>
    where
        F: FnMut(u64, u64),
    {
        let file_lock = Self::acquire_file_lock(target_path);
        let _guard = file_lock.lock().await;

        if Self::is_file_valid(target_path, expected_size) {
            let sz = expected_size
                .or_else(|| fs::metadata(target_path).ok().map(|m| m.len()))
                .unwrap_or(1);
            on_progress(sz, sz);
            return Ok(());
        }

        if let Some(parent) = target_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let file_name = target_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("download");
        let mut last_err = String::from("无可用下载源");

        // 对齐 PCL2 NetFile：两轮完整尝试（第一轮常规多源切换，第二轮 Retried 兜底重试）
        for pass in 0..2usize {
            if pass > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(800)).await;
            }
            for url in urls {
                if url.trim().is_empty() {
                    continue;
                }
                endpoints::pace_mirror_request(url).await;

                let resp = match client.get(url).send().await {
                    Ok(r) => r,
                    Err(e) => {
                        last_err = format!("{} 请求失败: {}", file_name, e);
                        continue;
                    }
                };

                let status = resp.status();
                if status.as_u16() == 429
                    || status.as_u16() == 503
                    || (status.as_u16() == 403 && endpoints::is_mirror_url(url))
                {
                    last_err = format!("{} 镜像源限流 (HTTP {})", file_name, status.as_u16());
                    tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
                    continue;
                }
                if !status.is_success() {
                    last_err = format!("{} HTTP {} ({})", file_name, status, url);
                    continue;
                }

                let content_len = resp.content_length();
                if let (Some(exp), Some(cl)) = (expected_size, content_len) {
                    if exp > 0 && cl > 0 && cl != exp {
                        last_err = format!(
                            "{} 远端 ContentLength ({}) 与预期大小 ({}) 不一致",
                            file_name, cl, exp
                        );
                        continue;
                    }
                }

                let total_bytes = expected_size.or(content_len).unwrap_or(0);
                let tmp_path = target_path.with_file_name(format!(
                    ".{}.{}.part.tmp",
                    file_name,
                    std::process::id()
                ));

                let mut stream_ok = true;
                let mut downloaded: u64 = 0;

                match File::create(&tmp_path) {
                    Ok(mut tmp_file) => {
                        let mut resp_stream = resp;
                        loop {
                            match resp_stream.chunk().await {
                                Ok(Some(chunk)) => {
                                    if let Err(e) = tmp_file.write_all(&chunk) {
                                        last_err = format!("写入临时文件失败: {}", e);
                                        stream_ok = false;
                                        break;
                                    }
                                    downloaded += chunk.len() as u64;
                                    on_progress(downloaded, total_bytes.max(downloaded));
                                }
                                Ok(None) => break,
                                Err(e) => {
                                    last_err = format!("{} 读取数据流中断: {}", file_name, e);
                                    stream_ok = false;
                                    break;
                                }
                            }
                        }
                        let _ = tmp_file.flush();
                    }
                    Err(e) => {
                        last_err = format!("创建临时文件失败: {}", e);
                        stream_ok = false;
                    }
                }

                if stream_ok {
                    let check_size = expected_size.or(content_len.filter(|&c| c > 0));
                    if Self::is_file_valid(&tmp_path, check_size) {
                        if target_path.exists() {
                            let _ = fs::remove_file(target_path);
                        }
                        if let Err(e) = fs::rename(&tmp_path, target_path) {
                            // 跨卷或重命名受阻时回退 copy + remove
                            if fs::copy(&tmp_path, target_path).is_ok() {
                                let _ = fs::remove_file(&tmp_path);
                                return Ok(());
                            }
                            last_err = format!("替换目标文件失败: {}", e);
                        } else {
                            return Ok(());
                        }
                    } else {
                        last_err = format!("{} 下载后文件校验未通过 ({})", file_name, url);
                    }
                }

                let _ = fs::remove_file(&tmp_path);
            }
        }

        Err(last_err)
    }

    /// 将 Maven 坐标转换为 (本地相对 PathBuf, 远端 URL 子路径)
    fn maven_to_path_and_url(name: &str) -> (PathBuf, String) {
        let rel_path = MinecraftLauncher::maven_to_path(name);
        let parts: Vec<&str> = name.split(':').collect();
        if parts.len() < 3 {
            return (rel_path, name.to_string());
        }
        let group_url = parts[0].split('.').collect::<Vec<&str>>().join("/");
        let artifact = parts[1];
        let version = parts[2];
        let classifier = if parts.len() > 3 {
            format!("-{}", parts[3])
        } else {
            String::new()
        };
        let file_name = if classifier.is_empty() {
            format!("{}-{}.jar", artifact, version)
        } else {
            format!("{}-{}{}.jar", artifact, version, classifier)
        };
        let url_subpath = format!("{}/{}/{}/{}", group_url, artifact, version, file_name);
        (rel_path, url_subpath)
    }

    async fn fetch_bytes_with_fallback(
        client: &reqwest::Client,
        primary_url: &str,
        backup_url: Option<&str>,
    ) -> Result<Vec<u8>, String> {
        let mut urls = vec![primary_url.to_string()];
        if let Some(b) = backup_url {
            if !b.is_empty() && b != primary_url {
                urls.push(b.to_string());
            }
        }
        let mut last_err = String::from("下载失败");
        for attempt in 0..3usize {
            if attempt > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(600 * attempt as u64)).await;
            }
            for url in &urls {
                endpoints::pace_mirror_request(url).await;
                match client.get(url).send().await {
                    Ok(resp) => {
                        let status = resp.status();
                        if status.as_u16() == 429 || status.as_u16() == 503 {
                            last_err = format!("HTTP {} ({})", status, url);
                            tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
                            continue;
                        }
                        if !status.is_success() {
                            last_err = format!("HTTP {} ({})", status, url);
                            continue;
                        }
                        match resp.bytes().await {
                            Ok(b) if !b.is_empty() => return Ok(b.to_vec()),
                            Ok(_) => last_err = format!("响应内容为空 ({})", url),
                            Err(e) => last_err = format!("读取字节流失败: {}", e),
                        }
                    }
                    Err(e) => last_err = format!("请求异常: {}", e),
                }
            }
        }
        Err(last_err)
    }

    async fn fetch_text_with_fallback(
        client: &reqwest::Client,
        primary_url: &str,
        backup_url: &str,
    ) -> Result<String, String> {
        let mut urls = vec![primary_url.to_string()];
        if !backup_url.is_empty() && backup_url != primary_url {
            urls.push(backup_url.to_string());
        }
        let mut last_err = String::from("请求文本失败");
        for attempt in 0..3usize {
            if attempt > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(500 * attempt as u64)).await;
            }
            for url in &urls {
                endpoints::pace_mirror_request(url).await;
                match client.get(url).send().await {
                    Ok(resp) => {
                        let status = resp.status();
                        if status.as_u16() == 429 || status.as_u16() == 503 {
                            last_err = format!("HTTP {} ({})", status, url);
                            tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
                            continue;
                        }
                        if !status.is_success() {
                            last_err = format!("HTTP {} ({})", status, url);
                            continue;
                        }
                        match resp.text().await {
                            Ok(txt) => {
                                let trimmed = txt.trim();
                                // 对齐 PCL2 RequireJson 检查：确保返回的是闭合 JSON，防止网关报错 HTML 混入
                                if (trimmed.starts_with('{') && trimmed.ends_with('}'))
                                    || (trimmed.starts_with('[') && trimmed.ends_with(']'))
                                {
                                    return Ok(txt);
                                }
                                last_err = format!("返回内容不是完整 JSON ({})", url);
                            }
                            Err(e) => last_err = format!("读取响应文本失败: {}", e),
                        }
                    }
                    Err(e) => last_err = format!("请求失败: {}", e),
                }
            }
        }
        Err(last_err)
    }

    /// 通用高并发批量下载池（使用 Semaphore 控制并发 + BMCLAPI 节流 + 临时文件原子校验 + 失败汇总报告）
    async fn execute_batch_downloads(
        app: &AppHandle,
        version_id: &str,
        vanilla_version: Option<&str>,
        stage_label: &str,
        start_pct: f32,
        end_pct: f32,
        concurrency: usize,
        client: &reqwest::Client,
        tasks: Vec<BatchDownloadTask>,
    ) -> Result<(), String> {
        let total = tasks.len();
        if total == 0 {
            return Ok(());
        }

        let van_ver_owned = vanilla_version.map(|s| s.to_string());
        let _ = app.emit(
            "download_progress",
            DownloadProgress {
                version_id: version_id.to_string(),
                vanilla_version: van_ver_owned.clone(),
                step: format!("{} (0/{})", stage_label, total),
                percentage: start_pct,
                speed_text: format!("0/{}", total),
                is_finished: false,
                error: None,
            },
        );

        let semaphore = Arc::new(Semaphore::new(concurrency.max(1)));
        let completed = Arc::new(AtomicUsize::new(0));
        let failures: Arc<std::sync::Mutex<Vec<String>>> =
            Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut join_set = JoinSet::new();
        let step_interval = (total / 40).max(1);

        for task in tasks {
            let sem = semaphore.clone();
            let cli = client.clone();
            let comp = completed.clone();
            let fail_list = failures.clone();
            let app_clone = app.clone();
            let ver_id = version_id.to_string();
            let van_ver = van_ver_owned.clone();
            let label = stage_label.to_string();

            join_set.spawn(async move {
                let _permit = sem.acquire_owned().await.ok();

                if let Err(err_msg) = Self::download_file_with_retry(
                    &cli,
                    &task.urls,
                    &task.target_path,
                    task.expected_size,
                    |_, _| {},
                )
                .await
                {
                    if let Ok(mut list) = fail_list.lock() {
                        if list.len() < 10 {
                            list.push(err_msg);
                        } else if list.len() == 10 {
                            list.push("...".to_string());
                        }
                    }
                } else if let Some((natives_dir, excludes)) = task.extract_natives {
                    // 若该任务标记为需解压 Natives，校验落盘成功后立即安全解压
                    if task.target_path.is_file() {
                        let _ = MinecraftLauncher::extract_native_jar(
                            &task.target_path,
                            &natives_dir,
                            excludes.as_deref(),
                        );
                    }
                }

                let done = comp.fetch_add(1, Ordering::Relaxed) + 1;
                if done == total || done % step_interval == 0 {
                    let ratio = done as f32 / total as f32;
                    let pct = start_pct + ratio * (end_pct - start_pct);
                    let _ = app_clone.emit(
                        "download_progress",
                        DownloadProgress {
                            version_id: ver_id,
                            vanilla_version: van_ver,
                            step: format!("{} ({}/{})", label, done, total),
                            percentage: pct,
                            speed_text: format!("{}/{}", done, total),
                            is_finished: false,
                            error: None,
                        },
                    );
                }
            });
        }

        while join_set.join_next().await.is_some() {}

        let recorded_failures = failures.lock().unwrap().clone();
        if !recorded_failures.is_empty() {
            return Err(format!(
                "{} 阶段有文件下载失败（已完成的文件已缓存，可直接点击重试补全）：\n{}",
                stage_label,
                recorded_failures.join("\n")
            ));
        }

        Ok(())
    }

    /// 从版本 libraries JSON 数组中收集适用于当前平台的所有支持库与 Natives 下载任务
    fn collect_library_tasks(
        lib_values: &[serde_json::Value],
        libraries_dir: &Path,
        natives_dir: Option<&Path>,
        prefer_mirror: bool,
        target_os: &str,
        target_arch: &str,
        default_maven_base: Option<&str>,
    ) -> Vec<BatchDownloadTask> {
        let active_features = HashMap::new();
        let mut tasks = Vec::new();

        for lib_val in lib_values {
            let Ok(parsed_lib) = serde_json::from_value::<LaunchLibrary>(lib_val.clone()) else {
                continue;
            };

            // §4：不适用于当前平台的库不下载
            if !MinecraftLauncher::check_rules_for_target(
                &parsed_lib.rules,
                target_os,
                target_arch,
                None,
                &active_features,
            ) {
                continue;
            }

            let extract_opt = if MinecraftLauncher::is_modern_native_library(&parsed_lib) {
                natives_dir.map(|nd| {
                    (
                        nd.to_path_buf(),
                        parsed_lib.extract.as_ref().and_then(|e| e.exclude.clone()),
                    )
                })
            } else {
                None
            };

            if let Some(ref artifact) = parsed_lib
                .downloads
                .as_ref()
                .and_then(|d| d.artifact.clone())
            {
                if let (Some(path_str), Some(url_str)) = (&artifact.path, &artifact.url) {
                    if !url_str.trim().is_empty() {
                        let rel_path = MinecraftLauncher::artifact_path_to_pathbuf(path_str);
                        let urls = endpoints::resolve_mojang_library_urls(url_str, prefer_mirror);
                        tasks.push(BatchDownloadTask {
                            target_path: libraries_dir.join(rel_path),
                            urls,
                            expected_size: artifact.size,
                            extract_natives: extract_opt,
                        });
                    }
                }
            } else if parsed_lib.downloads.is_none() {
                let (rel_path, url_subpath) = Self::maven_to_path_and_url(&parsed_lib.name);
                let repo_base = parsed_lib.url.as_deref().or(default_maven_base);
                let urls = endpoints::resolve_maven_library_urls_list(
                    repo_base,
                    &url_subpath,
                    prefer_mirror,
                );
                tasks.push(BatchDownloadTask {
                    target_path: libraries_dir.join(rel_path),
                    urls,
                    expected_size: None,
                    extract_natives: extract_opt,
                });
            }

            // 查表收集 classifiers 中的平台专属 natives 罐（§3.1.3）
            if let Some((_cls_key, native_art)) =
                MinecraftLauncher::select_native_classifier(&parsed_lib, target_os, target_arch)
            {
                if let (Some(path_str), Some(url_str)) = (&native_art.path, &native_art.url) {
                    let rel_path = MinecraftLauncher::artifact_path_to_pathbuf(path_str);
                    let urls = endpoints::resolve_mojang_library_urls(url_str, prefer_mirror);
                    let extract_cls = natives_dir.map(|nd| {
                        (
                            nd.to_path_buf(),
                            parsed_lib.extract.as_ref().and_then(|e| e.exclude.clone()),
                        )
                    });
                    tasks.push(BatchDownloadTask {
                        target_path: libraries_dir.join(rel_path),
                        urls,
                        expected_size: native_art.size,
                        extract_natives: extract_cls,
                    });
                }
            }
        }

        tasks
    }

    /// 将 Loader 的 Profile JSON（Fabric / Quilt / Forge / NeoForge）合并进目标版本的 JSON 对象
    fn merge_loader_profile_into_version_json(
        original_json_val: &mut serde_json::Value,
        profile: LoaderProfileJson,
        final_target_name: &str,
    ) {
        original_json_val["mainClass"] = serde_json::Value::String(profile.main_class);
        original_json_val["id"] = serde_json::Value::String(final_target_name.to_string());

        if let Some(orig_libs) = original_json_val
            .get_mut("libraries")
            .and_then(|l| l.as_array_mut())
        {
            for l_lib in profile.libraries {
                orig_libs.push(l_lib);
            }
        }

        if let Some(loader_args) = profile.arguments {
            if let Some(loader_jvm) = loader_args.get("jvm").and_then(|j| j.as_array()) {
                if let Some(orig_jvm) = original_json_val
                    .get_mut("arguments")
                    .and_then(|a| a.get_mut("jvm"))
                    .and_then(|j| j.as_array_mut())
                {
                    for j_arg in loader_jvm {
                        orig_jvm.push(j_arg.clone());
                    }
                }
            }
            if let Some(loader_game) = loader_args.get("game").and_then(|g| g.as_array()) {
                if let Some(orig_game) = original_json_val
                    .get_mut("arguments")
                    .and_then(|a| a.get_mut("game"))
                    .and_then(|g| g.as_array_mut())
                {
                    for g_arg in loader_game {
                        orig_game.push(g_arg.clone());
                    }
                }
            }
        } else if let Some(mc_args) = profile.minecraft_arguments {
            original_json_val["minecraftArguments"] = serde_json::Value::String(mc_args);
        }
    }

    /// 解析 Forge / NeoForge Installer JAR，提取内嵌 maven 库并返回其 version.json 与额外依赖库
    fn extract_forge_or_neoforge_installer(
        installer_bytes: &[u8],
        libraries_dir: &Path,
    ) -> Result<(LoaderProfileJson, Vec<serde_json::Value>), String> {
        let reader = io::Cursor::new(installer_bytes);
        let mut archive =
            ZipArchive::new(reader).map_err(|e| format!("解析 Installer JAR 失败: {}", e))?;

        let mut version_json_str = String::new();
        let mut install_profile_str = String::new();

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
            let Some(enclosed) = entry.enclosed_name() else {
                continue;
            };
            let norm = enclosed.to_string_lossy().to_string();

            if norm == "version.json" {
                let _ = entry.read_to_string(&mut version_json_str);
            } else if norm == "install_profile.json" {
                let _ = entry.read_to_string(&mut install_profile_str);
            } else if norm.starts_with("maven/") && !entry.is_dir() {
                // 安全解压 installer 内置的 maven 构件到 libraries 目录（遵循 §4 防 zip-slip）
                let mut dest = libraries_dir.to_path_buf();
                for comp in enclosed.components().skip(1) {
                    if let Component::Normal(part) = comp {
                        dest = dest.join(part);
                    }
                }
                if let Some(parent) = dest.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                if let Ok(mut out_f) = File::create(&dest) {
                    let _ = io::copy(&mut entry, &mut out_f);
                }
            }
        }

        // 旧版 Forge 可能把 versionInfo 嵌在 install_profile.json 的 "versionInfo" 字段内
        let version_profile: LoaderProfileJson = if !version_json_str.is_empty() {
            serde_json::from_str(&version_json_str)
                .map_err(|e| format!("解析 Installer version.json 失败: {}", e))?
        } else if !install_profile_str.is_empty() {
            let raw_prof: serde_json::Value = serde_json::from_str(&install_profile_str)
                .map_err(|e| format!("解析 install_profile.json 失败: {}", e))?;
            if let Some(ver_info) = raw_prof.get("versionInfo") {
                serde_json::from_value(ver_info.clone())
                    .map_err(|e| format!("解析 versionInfo 失败: {}", e))?
            } else {
                return Err("Installer JAR 中未找到有效的版本定义".to_string());
            }
        } else {
            return Err("Installer JAR 中缺少 version.json 与 install_profile.json".to_string());
        };

        let mut extra_libs = Vec::new();
        if !install_profile_str.is_empty() {
            if let Ok(raw_prof) = serde_json::from_str::<serde_json::Value>(&install_profile_str) {
                if let Some(libs) = raw_prof.get("libraries").and_then(|l| l.as_array()) {
                    extra_libs.extend(libs.iter().cloned());
                }
            }
        }

        Ok((version_profile, extra_libs))
    }

    /// 解析并安装 OptiFine Installer JAR，提取内置 launchwrapper-of、执行字节码补丁合并并构造 LoaderProfileJson
    fn extract_optifine_installer(
        installer_bytes: &[u8],
        vanilla_version: &str,
        loader_ver: &str,
        client_jar_path: &Path,
        libraries_dir: &Path,
        final_target_name: &str,
        original_json_val: &serde_json::Value,
    ) -> Result<(LoaderProfileJson, Vec<serde_json::Value>), String> {
        let reader = io::Cursor::new(installer_bytes);
        let mut archive =
            ZipArchive::new(reader).map_err(|e| format!("解析 OptiFine JAR 失败: {}", e))?;

        let mut lw_ver_opt: Option<String> = None;
        let mut has_patches = false;

        for i in 0..archive.len() {
            if let Ok(mut entry) = archive.by_index(i) {
                let Some(enclosed) = entry.enclosed_name() else {
                    continue;
                };
                let norm = enclosed.to_string_lossy().replace('\\', "/");
                if norm == "launchwrapper-of.txt" || norm == "/launchwrapper-of.txt" {
                    let mut s = String::new();
                    if entry.read_to_string(&mut s).is_ok() {
                        let trimmed = s.trim().to_string();
                        if !trimmed.is_empty() {
                            lw_ver_opt = Some(trimmed);
                        }
                    }
                } else if norm.starts_with("patch/") {
                    has_patches = true;
                }
            }
        }

        // 若包含内置 launchwrapper-of-<ver>.jar，安全解压到 libraries/optifine/launchwrapper-of/<ver>/
        if let Some(ref lw_ver) = lw_ver_opt {
            let lw_jar_name = format!("launchwrapper-of-{}.jar", lw_ver);
            for i in 0..archive.len() {
                if let Ok(mut entry) = archive.by_index(i) {
                    let Some(enclosed) = entry.enclosed_name() else {
                        continue;
                    };
                    let norm = enclosed.to_string_lossy().replace('\\', "/");
                    if norm == lw_jar_name && !entry.is_dir() {
                        let lw_dest = libraries_dir
                            .join("optifine")
                            .join("launchwrapper-of")
                            .join(lw_ver)
                            .join(&lw_jar_name);
                        if let Some(parent) = lw_dest.parent() {
                            let _ = fs::create_dir_all(parent);
                        }
                        if let Ok(mut out_f) = File::create(&lw_dest) {
                            let _ = io::copy(&mut entry, &mut out_f);
                        }
                        break;
                    }
                }
            }
        }

        let of_full_ver = format!("{}_{}", vanilla_version, loader_ver);
        let of_lib_dir = libraries_dir
            .join("optifine")
            .join("OptiFine")
            .join(&of_full_ver);
        let _ = fs::create_dir_all(&of_lib_dir);
        let of_lib_jar = of_lib_dir.join(format!("OptiFine-{}.jar", of_full_ver));

        let mut patched_ok = false;
        if has_patches && client_jar_path.is_file() {
            let temp_installer = of_lib_dir.join(format!("OptiFine-{}-installer.jar", of_full_ver));
            if fs::write(&temp_installer, installer_bytes).is_ok() {
                let java_bin = ConfigManager::load()
                    .java_path
                    .filter(|p| !p.is_empty() && Path::new(p).is_file())
                    .or_else(|| {
                        crate::java::JavaScanner::scan_all()
                            .into_iter()
                            .next()
                            .map(|j| j.path)
                    });

                if let Some(java_exe) = java_bin {
                    if let Ok(out) = std::process::Command::new(&java_exe)
                        .arg("-cp")
                        .arg(&temp_installer)
                        .arg("optifine.Patcher")
                        .arg(client_jar_path)
                        .arg(&temp_installer)
                        .arg(&of_lib_jar)
                        .output()
                    {
                        if out.status.success() && of_lib_jar.is_file() {
                            patched_ok = true;
                        }
                    }
                }
                let _ = fs::remove_file(&temp_installer);
            }
        }

        if !patched_ok {
            fs::write(&of_lib_jar, installer_bytes)
                .map_err(|e| format!("写入 OptiFine 库文件失败: {}", e))?;
        }

        let mut extra_libs = Vec::new();
        let lw_lib_val = if let Some(lw_ver) = lw_ver_opt {
            serde_json::json!({
                "name": format!("optifine:launchwrapper-of:{}", lw_ver)
            })
        } else {
            let legacy_lw = serde_json::json!({
                "name": "net.minecraft:launchwrapper:1.12"
            });
            extra_libs.push(legacy_lw.clone());
            legacy_lw
        };

        let of_lib_val = serde_json::json!({
            "name": format!("optifine:OptiFine:{}", of_full_ver)
        });

        let legacy_mc_args = original_json_val
            .get("minecraftArguments")
            .and_then(|v| v.as_str())
            .map(|s| format!("{} --tweakClass optifine.OptiFineTweaker", s));

        let arguments = if legacy_mc_args.is_none() {
            Some(serde_json::json!({
                "game": ["--tweakClass", "optifine.OptiFineTweaker"]
            }))
        } else {
            None
        };

        let profile = LoaderProfileJson {
            id: final_target_name.to_string(),
            inherits_from: Some(vanilla_version.to_string()),
            main_class: "net.minecraft.launchwrapper.Launch".to_string(),
            libraries: vec![lw_lib_val, of_lib_val],
            arguments,
            minecraft_arguments: legacy_mc_args,
        };

        Ok((profile, extra_libs))
    }

    /// 完整安装指定版本的原版核心、并发支持库、Natives、Assets 资源及可选 Loader (Fabric / Quilt / Forge / NeoForge)
    pub async fn install_version(
        app: AppHandle,
        game_dir_str: String,
        vanilla_version: String,
        target_name: Option<String>,
        meta_url: String,
        loader_type: Option<String>,
        loader_version: Option<String>,
    ) -> Result<(), String> {
        let final_target_name = target_name.unwrap_or_else(|| vanilla_version.clone());
        let game_dir = Path::new(&game_dir_str);
        let version_dir = game_dir.join("versions").join(&final_target_name);

        if !version_dir.exists() {
            fs::create_dir_all(&version_dir).map_err(|e| format!("创建版本目录失败: {}", e))?;
        }

        let emit_progress =
            |step: &str, pct: f32, speed: &str, finished: bool, err: Option<String>| {
                let _ = app.emit(
                    "download_progress",
                    DownloadProgress {
                        version_id: final_target_name.clone(),
                        vanilla_version: Some(vanilla_version.clone()),
                        step: step.to_string(),
                        percentage: pct,
                        speed_text: speed.to_string(),
                        is_finished: finished,
                        error: err,
                    },
                );
            };

        let prefer_mirror = ConfigManager::load().prefer_mirror;
        let target_os = platform::mc_os_name();
        let target_arch = platform::mc_os_arch();

        let client = endpoints::shared_http_client();

        // 1. 获取并保存版本元数据 JSON
        emit_progress("正在获取原版元数据 JSON...", 5.0, "拉取清单", false, None);

        let (primary_meta_url, backup_meta_url) =
            endpoints::resolve_mojang_download_urls(&meta_url, prefer_mirror);

        let json_text =
            Self::fetch_text_with_fallback(client, &primary_meta_url, &backup_meta_url)
                .await
                .map_err(|e| format!("无法下载版本 JSON: {}", e))?;

        let json_file_path = version_dir.join(format!("{}.json", final_target_name));
        let mut original_json_val: serde_json::Value = serde_json::from_str(&json_text)
            .map_err(|e| format!("解析版本 JSON 结构失败: {}", e))?;

        original_json_val["id"] = serde_json::Value::String(final_target_name.clone());
        fs::write(
            &json_file_path,
            serde_json::to_string_pretty(&original_json_val).unwrap_or(json_text.clone()),
        )
        .map_err(|e| format!("写入 JSON 失败: {}", e))?;

        let ver_detail: VersionDetailJson = serde_json::from_str(&json_text)
            .map_err(|e| format!("解析版本 JSON 结构失败: {}", e))?;

        // 2. 下载或本地复用 client.jar (10% ~ 40%)
        emit_progress(
            "正在准备下载游戏核心 (client.jar)...",
            10.0,
            "解析地址",
            false,
            None,
        );

        if let Some(ref downloads) = ver_detail.downloads {
            if let Some(ref client_art) = downloads.client {
                let jar_file_path = version_dir.join(format!("{}.jar", final_target_name));

                // 对齐 PCL2 CheckExistingFiles：若同目录下原版版本已有通过校验的 client.jar，直接本地复制免重复下载
                if !Self::is_file_valid(&jar_file_path, Some(client_art.size))
                    && final_target_name != vanilla_version
                {
                    let vanilla_jar = game_dir
                        .join("versions")
                        .join(&vanilla_version)
                        .join(format!("{}.jar", vanilla_version));
                    if Self::is_file_valid(&vanilla_jar, Some(client_art.size)) {
                        let _ = fs::copy(&vanilla_jar, &jar_file_path);
                    }
                }

                let (primary_client_url, backup_client_url) =
                    endpoints::resolve_mojang_download_urls(&client_art.url, prefer_mirror);
                let client_urls = vec![primary_client_url, backup_client_url];
                let mut last_emit_pct = 0.0f32;

                Self::download_file_with_retry(
                    client,
                    &client_urls,
                    &jar_file_path,
                    Some(client_art.size),
                    |downloaded, total| {
                        let total_f = total.max(1) as f32;
                        let dl_f = downloaded as f32;
                        let pct = 10.0 + (dl_f / total_f) * 30.0;
                        if (pct - last_emit_pct).abs() >= 0.8 || downloaded >= total {
                            last_emit_pct = pct;
                            let mb_text = format!(
                                "{:.1}MB / {:.1}MB",
                                dl_f / (1024.0 * 1024.0),
                                total_f / (1024.0 * 1024.0)
                            );
                            emit_progress(
                                "正在高速下载核心 JAR 文件...",
                                pct,
                                &mb_text,
                                false,
                                None,
                            );
                        }
                    },
                )
                .await
                .map_err(|e| format!("下载核心 client.jar 失败: {}", e))?;
            }
        }

        // 3. 高并发下载原版运行支持库与 Natives (40% ~ 65%)
        let libraries_dir = game_dir.join("libraries");
        if !libraries_dir.exists() {
            let _ = fs::create_dir_all(&libraries_dir);
        }
        let natives_dir = version_dir.join(format!("{}-natives", final_target_name));
        if !natives_dir.exists() {
            let _ = fs::create_dir_all(&natives_dir);
        }

        if let Some(orig_libs) = original_json_val.get("libraries").and_then(|l| l.as_array()) {
            let lib_tasks = Self::collect_library_tasks(
                orig_libs,
                &libraries_dir,
                Some(&natives_dir),
                prefer_mirror,
                target_os,
                target_arch,
                None,
            );
            Self::execute_batch_downloads(
                &app,
                &final_target_name,
                Some(&vanilla_version),
                "正在并发下载支持库与 Natives",
                40.0,
                65.0,
                DEFAULT_LIBRARY_CONCURRENCY,
                client,
                lib_tasks,
            )
            .await?;
        }

        // 4. 下载并补全 Assets 资源索引与音频/语言包对象 (65% ~ 85%)
        if let Some(ref asset_meta) = ver_detail.asset_index {
            emit_progress(
                &format!("正在拉取资源索引 ({}.json)...", asset_meta.id),
                66.0,
                "资源索引",
                false,
                None,
            );
            let indexes_dir = game_dir.join("assets").join("indexes");
            let objects_dir = game_dir.join("assets").join("objects");
            let _ = fs::create_dir_all(&indexes_dir);
            let _ = fs::create_dir_all(&objects_dir);

            let (primary_idx_url, backup_idx_url) =
                endpoints::resolve_mojang_download_urls(&asset_meta.url, prefer_mirror);

            let idx_text =
                Self::fetch_text_with_fallback(client, &primary_idx_url, &backup_idx_url)
                    .await
                    .map_err(|e| format!("拉取资源索引 ({}.json) 失败: {}", asset_meta.id, e))?;

            let idx_path = indexes_dir.join(format!("{}.json", asset_meta.id));
            let _ = fs::write(&idx_path, &idx_text);

            let index_file: AssetIndexFile = serde_json::from_str(&idx_text)
                .map_err(|e| format!("解析资源索引 ({}.json) 失败: {}", asset_meta.id, e))?;

            let mut asset_tasks = Vec::new();
            let mut seen_hashes = std::collections::HashSet::new();

            for entry in index_file.objects.values() {
                if entry.hash.len() < 2 || !seen_hashes.insert(entry.hash.clone()) {
                    continue;
                }
                let prefix = &entry.hash[0..2];
                let target_obj_path = objects_dir.join(prefix).join(&entry.hash);
                if Self::is_file_valid(&target_obj_path, Some(entry.size)) {
                    continue;
                }
                let (primary_url, backup_url) =
                    endpoints::resolve_asset_object_urls(&entry.hash, prefer_mirror);
                asset_tasks.push(BatchDownloadTask {
                    target_path: target_obj_path,
                    urls: vec![primary_url, backup_url],
                    expected_size: Some(entry.size),
                    extract_natives: None,
                });
            }

            if !asset_tasks.is_empty() {
                Self::execute_batch_downloads(
                    &app,
                    &final_target_name,
                    Some(&vanilla_version),
                    "正在并发补全游戏音效与语言包资源",
                    68.0,
                    85.0,
                    DEFAULT_ASSET_CONCURRENCY,
                    client,
                    asset_tasks,
                )
                .await?;
            }
        }

        // 5. 可选：自动合成加载器 (Fabric / Quilt / Forge / NeoForge / OptiFine) (85% ~ 98%)
        match loader_type.as_deref() {
            Some("fabric") => {
                emit_progress(
                    "正在查询匹配的 Fabric Loader...",
                    86.0,
                    "Fabric 元数据",
                    false,
                    None,
                );
                let loader_urls =
                    endpoints::fabric_loader_list_urls(&vanilla_version, prefer_mirror);
                let loaders: Vec<FabricLoaderEntry> =
                    crate::download::DownloadManager::fetch_json_with_retry(&loader_urls)
                        .await
                        .map_err(|e| format!("查询 Fabric Loader 清单失败: {}", e))?;

                let chosen_entry = if let Some(ref req_ver) = loader_version {
                    loaders.iter().find(|l| &l.loader.version == req_ver)
                } else {
                    loaders
                        .iter()
                        .find(|l| l.loader.stable)
                        .or_else(|| loaders.first())
                }
                .ok_or_else(|| format!("未找到匹配的 Fabric Loader 版本 ({:?})", loader_version))?;

                let loader_ver = &chosen_entry.loader.version;
                let profile_urls = endpoints::fabric_profile_json_urls(
                    &vanilla_version,
                    loader_ver,
                    prefer_mirror,
                );

                let fabric_profile: LoaderProfileJson =
                    crate::download::DownloadManager::fetch_json_with_retry(&profile_urls)
                        .await
                        .map_err(|e| format!("拉取 Fabric Profile JSON 失败: {}", e))?;

                let fab_tasks = Self::collect_library_tasks(
                    &fabric_profile.libraries,
                    &libraries_dir,
                    None,
                    prefer_mirror,
                    target_os,
                    target_arch,
                    Some(endpoints::FABRIC_MAVEN_OFFICIAL),
                );
                Self::execute_batch_downloads(
                    &app,
                    &final_target_name,
                    Some(&vanilla_version),
                    &format!("正在并发下载 Fabric {} 依赖库", loader_ver),
                    88.0,
                    98.0,
                    DEFAULT_LIBRARY_CONCURRENCY,
                    client,
                    fab_tasks,
                )
                .await?;

                Self::merge_loader_profile_into_version_json(
                    &mut original_json_val,
                    fabric_profile,
                    &final_target_name,
                );
                let merged_str = serde_json::to_string_pretty(&original_json_val)
                    .map_err(|e| format!("序列化合并后的版本 JSON 失败: {}", e))?;
                fs::write(&json_file_path, merged_str)
                    .map_err(|e| format!("写入合并后的版本 JSON 失败: {}", e))?;
            }
            Some("quilt") => {
                let loader_ver = loader_version
                    .as_deref()
                    .ok_or_else(|| "未指定 Quilt Loader 版本".to_string())?;
                let profile_urls = endpoints::quilt_profile_json_urls(
                    &vanilla_version,
                    loader_ver,
                    prefer_mirror,
                );
                let quilt_profile: LoaderProfileJson =
                    crate::download::DownloadManager::fetch_json_with_retry(&profile_urls)
                        .await
                        .map_err(|e| format!("拉取 Quilt Profile JSON 失败: {}", e))?;

                let quilt_tasks = Self::collect_library_tasks(
                    &quilt_profile.libraries,
                    &libraries_dir,
                    None,
                    prefer_mirror,
                    target_os,
                    target_arch,
                    Some(endpoints::QUILT_MAVEN_OFFICIAL),
                );
                Self::execute_batch_downloads(
                    &app,
                    &final_target_name,
                    Some(&vanilla_version),
                    &format!("正在并发下载 Quilt {} 依赖库", loader_ver),
                    88.0,
                    98.0,
                    DEFAULT_LIBRARY_CONCURRENCY,
                    client,
                    quilt_tasks,
                )
                .await?;

                Self::merge_loader_profile_into_version_json(
                    &mut original_json_val,
                    quilt_profile,
                    &final_target_name,
                );
                let merged_str = serde_json::to_string_pretty(&original_json_val)
                    .map_err(|e| format!("序列化合并后的版本 JSON 失败: {}", e))?;
                fs::write(&json_file_path, merged_str)
                    .map_err(|e| format!("写入合并后的版本 JSON 失败: {}", e))?;
            }
            Some("forge") | Some("neoforge") => {
                let is_neoforge = loader_type.as_deref() == Some("neoforge");
                let label_name = if is_neoforge { "NeoForge" } else { "Forge" };
                let loader_ver = loader_version
                    .as_deref()
                    .ok_or_else(|| format!("未指定 {} 版本", label_name))?;

                emit_progress(
                    &format!("正在下载 {} {} 安装包...", label_name, loader_ver),
                    86.0,
                    "下载安装器",
                    false,
                    None,
                );

                let (primary_inst_url, backup_inst_url) = if is_neoforge {
                    endpoints::neoforge_installer_urls(loader_ver, prefer_mirror)
                } else {
                    endpoints::forge_installer_urls(&vanilla_version, loader_ver, prefer_mirror)
                };

                let inst_bytes = Self::fetch_bytes_with_fallback(
                    client,
                    &primary_inst_url,
                    Some(&backup_inst_url),
                )
                .await
                .map_err(|e| format!("下载 {} {} 安装包失败: {}", label_name, loader_ver, e))?;

                emit_progress(
                    &format!("正在解包并合成 {} {} 元数据...", label_name, loader_ver),
                    90.0,
                    "解包合成",
                    false,
                    None,
                );
                let (loader_profile, extra_libs) =
                    Self::extract_forge_or_neoforge_installer(&inst_bytes, &libraries_dir)?;

                let mut all_forge_libs = loader_profile.libraries.clone();
                all_forge_libs.extend(extra_libs);

                let default_repo = if is_neoforge {
                    endpoints::NEOFORGE_MAVEN_OFFICIAL
                } else {
                    endpoints::FORGE_MAVEN_OFFICIAL
                };

                let forge_tasks = Self::collect_library_tasks(
                    &all_forge_libs,
                    &libraries_dir,
                    None,
                    prefer_mirror,
                    target_os,
                    target_arch,
                    Some(default_repo),
                );
                Self::execute_batch_downloads(
                    &app,
                    &final_target_name,
                    Some(&vanilla_version),
                    &format!("正在并发下载 {} {} 依赖库", label_name, loader_ver),
                    91.0,
                    98.0,
                    DEFAULT_LIBRARY_CONCURRENCY,
                    client,
                    forge_tasks,
                )
                .await?;

                Self::merge_loader_profile_into_version_json(
                    &mut original_json_val,
                    loader_profile,
                    &final_target_name,
                );
                let merged_str = serde_json::to_string_pretty(&original_json_val)
                    .map_err(|e| format!("序列化合并后的版本 JSON 失败: {}", e))?;
                fs::write(&json_file_path, merged_str)
                    .map_err(|e| format!("写入合并后的版本 JSON 失败: {}", e))?;
            }
            Some("optifine") => {
                let loader_ver = loader_version
                    .as_deref()
                    .ok_or_else(|| "未指定 OptiFine 版本".to_string())?;
                let (of_type, of_patch) = loader_ver
                    .rsplit_once('_')
                    .ok_or_else(|| format!("非法的 OptiFine 版本格式: {}", loader_ver))?;

                emit_progress(
                    &format!("正在下载 OptiFine {} 安装包...", loader_ver),
                    86.0,
                    "下载安装器",
                    false,
                    None,
                );
                let inst_url =
                    endpoints::optifine_installer_url(&vanilla_version, of_type, of_patch);
                let inst_bytes = Self::fetch_bytes_with_fallback(client, &inst_url, None)
                    .await
                    .map_err(|e| format!("下载 OptiFine {} 安装包失败: {}", loader_ver, e))?;

                emit_progress(
                    &format!("正在解包并合成 OptiFine {}...", loader_ver),
                    92.0,
                    "解包合成",
                    false,
                    None,
                );
                let client_jar_path = version_dir.join(format!("{}.jar", final_target_name));
                let (loader_profile, extra_libs) = Self::extract_optifine_installer(
                    &inst_bytes,
                    &vanilla_version,
                    loader_ver,
                    &client_jar_path,
                    &libraries_dir,
                    &final_target_name,
                    &original_json_val,
                )?;

                if !extra_libs.is_empty() {
                    let of_tasks = Self::collect_library_tasks(
                        &extra_libs,
                        &libraries_dir,
                        None,
                        prefer_mirror,
                        target_os,
                        target_arch,
                        None,
                    );
                    Self::execute_batch_downloads(
                        &app,
                        &final_target_name,
                        Some(&vanilla_version),
                        &format!("正在并发下载 OptiFine {} 依赖库", loader_ver),
                        94.0,
                        98.0,
                        DEFAULT_LIBRARY_CONCURRENCY,
                        client,
                        of_tasks,
                    )
                    .await?;
                }

                Self::merge_loader_profile_into_version_json(
                    &mut original_json_val,
                    loader_profile,
                    &final_target_name,
                );
                let merged_str = serde_json::to_string_pretty(&original_json_val)
                    .map_err(|e| format!("序列化合并后的版本 JSON 失败: {}", e))?;
                fs::write(&json_file_path, merged_str)
                    .map_err(|e| format!("写入合并后的版本 JSON 失败: {}", e))?;
            }
            _ => {}
        }

        // 6. 安装完成
        emit_progress("安装完成！已成功加入版本列表", 100.0, "完成", true, None);
        Ok(())
    }

    /// 下载并配置 Minecraft 官方服务端（遵循 §3.5：调用 platform::server_script 生成跨平台脚本并返回脚本名）
    pub async fn install_server(
        app: AppHandle,
        target_dir: String,
        version_id: String,
        meta_url: String,
    ) -> Result<String, String> {
        let emit_progress =
            |step: &str, pct: f32, speed: &str, finished: bool, err: Option<String>| {
                let _ = app.emit(
                    "download_progress",
                    DownloadProgress {
                        version_id: format!("{}-server", version_id),
                        vanilla_version: Some(version_id.clone()),
                        step: step.to_string(),
                        percentage: pct,
                        speed_text: speed.to_string(),
                        is_finished: finished,
                        error: err,
                    },
                );
            };

        emit_progress("正在获取服务端元数据...", 10.0, "", false, None);

        let cfg = ConfigManager::load();
        let prefer_mirror = cfg.prefer_mirror;

        let client = endpoints::shared_http_client();

        let (primary_meta_url, backup_meta_url) =
            endpoints::resolve_mojang_download_urls(&meta_url, prefer_mirror);

        let meta_text =
            Self::fetch_text_with_fallback(client, &primary_meta_url, &backup_meta_url)
                .await
                .map_err(|e| format!("获取版本元数据失败: {}", e))?;

        let detail: VersionDetailJson = serde_json::from_str(&meta_text)
            .map_err(|e| format!("解析版本元数据失败: {}", e))?;

        let server_art = match detail.downloads.and_then(|d| d.server) {
            Some(s) => s,
            None => {
                let err = format!("Mojang 官方没有为 Minecraft {} 提供服务端文件", version_id);
                emit_progress(&err, 0.0, "", true, Some(err.clone()));
                return Err(err);
            }
        };

        emit_progress("正在下载官方服务端核心...", 25.0, "下载中", false, None);

        let (primary_server_url, backup_server_url) =
            endpoints::resolve_mojang_download_urls(&server_art.url, prefer_mirror);

        let target_path = PathBuf::from(&target_dir);
        if !target_path.exists() {
            fs::create_dir_all(&target_path).map_err(|e| e.to_string())?;
        }

        let jar_name = format!("{}-server.jar", version_id);
        let server_jar_path = target_path.join(&jar_name);
        let server_urls = vec![primary_server_url, backup_server_url];
        let mut last_emit_pct = 0.0f32;

        Self::download_file_with_retry(
            client,
            &server_urls,
            &server_jar_path,
            Some(server_art.size),
            |downloaded, total| {
                let total_f = total.max(1) as f32;
                let dl_f = downloaded as f32;
                let pct = 25.0 + (dl_f / total_f) * 63.0;
                if (pct - last_emit_pct).abs() >= 1.0 || downloaded >= total {
                    last_emit_pct = pct;
                    let mb_text = format!(
                        "{:.1}MB / {:.1}MB",
                        dl_f / (1024.0 * 1024.0),
                        total_f / (1024.0 * 1024.0)
                    );
                    emit_progress("正在下载官方服务端核心...", pct, &mb_text, false, None);
                }
            },
        )
        .await
        .map_err(|e| format!("下载服务端核心失败: {}", e))?;

        emit_progress("正在生成开服脚本与配置文件...", 90.0, "", false, None);

        let script_name = platform::server_script_with_memory(
            &target_path,
            &version_id,
            &jar_name,
            cfg.max_memory_mb,
        )?;

        let eula_content = "#By changing the setting below to TRUE you are indicating your agreement to our EULA (https://aka.ms/MinecraftEULA).\neula=true\n";
        let _ = fs::write(target_path.join("eula.txt"), eula_content);

        emit_progress("服务端与启动脚本已准备就绪！", 100.0, "完成", true, None);
        Ok(script_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    #[test]
    fn test_extract_optifine_installer_and_merge() {
        let mut buf = Vec::new();
        {
            let mut zw = ZipWriter::new(io::Cursor::new(&mut buf));
            let opts = SimpleFileOptions::default();
            zw.start_file("launchwrapper-of.txt", opts).unwrap();
            zw.write_all(b"2.3\n").unwrap();
            zw.start_file("launchwrapper-of-2.3.jar", opts).unwrap();
            zw.write_all(b"dummy-lw-jar").unwrap();
            zw.finish().unwrap();
        }

        let temp_dir = std::env::temp_dir().join(format!("pcl_of_test_{}", std::process::id()));
        let libs_dir = temp_dir.join("libraries");
        let client_jar = temp_dir.join("1.20.4.jar");
        let mut orig_json = serde_json::json!({
            "id": "1.20.4",
            "mainClass": "net.minecraft.client.main.Main",
            "libraries": [],
            "arguments": {
                "game": [],
                "jvm": []
            }
        });

        let (profile, extra_libs) = VersionInstaller::extract_optifine_installer(
            &buf,
            "1.20.4",
            "HD_U_I7",
            &client_jar,
            &libs_dir,
            "1.20.4-OptiFine_HD_U_I7",
            &orig_json,
        )
        .expect("extract optifine installer should succeed");

        assert!(extra_libs.is_empty());
        assert_eq!(profile.main_class, "net.minecraft.launchwrapper.Launch");
        assert!(libs_dir
            .join("optifine")
            .join("launchwrapper-of")
            .join("2.3")
            .join("launchwrapper-of-2.3.jar")
            .is_file());
        assert!(libs_dir
            .join("optifine")
            .join("OptiFine")
            .join("1.20.4_HD_U_I7")
            .join("OptiFine-1.20.4_HD_U_I7.jar")
            .is_file());

        VersionInstaller::merge_loader_profile_into_version_json(
            &mut orig_json,
            profile,
            "1.20.4-OptiFine_HD_U_I7",
        );

        assert_eq!(orig_json["id"], "1.20.4-OptiFine_HD_U_I7");
        assert_eq!(
            orig_json["mainClass"],
            "net.minecraft.launchwrapper.Launch"
        );
        let game_args = orig_json["arguments"]["game"].as_array().unwrap();
        assert!(game_args.contains(&serde_json::json!("--tweakClass")));
        assert!(game_args.contains(&serde_json::json!("optifine.OptiFineTweaker")));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_is_file_valid_rejects_html_jar_and_size_mismatch() {
        let temp_dir = std::env::temp_dir().join(format!("pcl_valid_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);

        let fake_jar = temp_dir.join("corrupted.jar");
        let html_err = b"<html><body>429 Too Many Requests</body></html>";
        fs::write(&fake_jar, html_err).unwrap();

        // 即使字节数碰巧一致，非 PK\x03\x04 开头的 .jar 也必须被拒绝
        assert!(!VersionInstaller::is_file_valid(
            &fake_jar,
            Some(html_err.len() as u64)
        ));

        // 合法 ZIP/JAR 魔数头且尺寸吻合时通过
        let valid_jar = temp_dir.join("valid.jar");
        let mut zip_bytes = Vec::new();
        {
            let mut zw = ZipWriter::new(io::Cursor::new(&mut zip_bytes));
            zw.start_file("META-INF/MANIFEST.MF", SimpleFileOptions::default())
                .unwrap();
            zw.write_all(b"Manifest-Version: 1.0\n").unwrap();
            zw.finish().unwrap();
        }
        fs::write(&valid_jar, &zip_bytes).unwrap();
        assert!(VersionInstaller::is_file_valid(
            &valid_jar,
            Some(zip_bytes.len() as u64)
        ));
        assert!(!VersionInstaller::is_file_valid(
            &valid_jar,
            Some(zip_bytes.len() as u64 + 10)
        ));

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
