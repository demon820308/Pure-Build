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
    primary_url: String,
    backup_url: Option<String>,
    expected_size: Option<u64>,
    extract_natives: Option<(PathBuf, Option<Vec<String>>)>,
}

pub struct VersionInstaller;

impl VersionInstaller {
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

    async fn fetch_bytes(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, String> {
        let resp = client.get(url).send().await.map_err(|e| e.to_string())?;
        if resp.status().is_success() {
            let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
            Ok(bytes.to_vec())
        } else {
            Err(format!("HTTP {}", resp.status()))
        }
    }

    async fn fetch_bytes_with_fallback(
        client: &reqwest::Client,
        primary_url: &str,
        backup_url: Option<&str>,
    ) -> Result<Vec<u8>, String> {
        match Self::fetch_bytes(client, primary_url).await {
            Ok(b) => Ok(b),
            Err(e) => {
                if let Some(backup) = backup_url {
                    Self::fetch_bytes(client, backup)
                        .await
                        .map_err(|e2| format!("下载失败: {} / {}", e, e2))
                } else {
                    Err(format!("下载失败: {}", e))
                }
            }
        }
    }

    async fn fetch_text_with_fallback(
        client: &reqwest::Client,
        primary_url: &str,
        backup_url: &str,
    ) -> Result<String, String> {
        if let Ok(resp) = client.get(primary_url).send().await {
            if resp.status().is_success() {
                if let Ok(txt) = resp.text().await {
                    return Ok(txt);
                }
            }
        }
        let resp = client
            .get(backup_url)
            .send()
            .await
            .map_err(|e| format!("请求失败: {}", e))?;
        resp.text().await.map_err(|e| format!("读取响应失败: {}", e))
    }

    /// 通用高并发批量下载池（使用 Semaphore 控制并发，实时派发进度）
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
    ) {
        let total = tasks.len();
        if total == 0 {
            return;
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
        let mut join_set = JoinSet::new();
        let step_interval = (total / 40).max(1);

        for task in tasks {
            let sem = semaphore.clone();
            let cli = client.clone();
            let comp = completed.clone();
            let app_clone = app.clone();
            let ver_id = version_id.to_string();
            let van_ver = van_ver_owned.clone();
            let label = stage_label.to_string();

            join_set.spawn(async move {
                let _permit = sem.acquire_owned().await.ok();

                let already_valid = if task.target_path.is_file() {
                    if let Ok(meta) = fs::metadata(&task.target_path) {
                        if let Some(exp_size) = task.expected_size {
                            meta.len() == exp_size
                        } else {
                            meta.len() > 0
                        }
                    } else {
                        false
                    }
                } else {
                    false
                };

                if !already_valid {
                    if let Some(parent) = task.target_path.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    if let Ok(bytes) = Self::fetch_bytes_with_fallback(
                        &cli,
                        &task.primary_url,
                        task.backup_url.as_deref(),
                    )
                    .await
                    {
                        if let Ok(mut f) = File::create(&task.target_path) {
                            let _ = f.write_all(&bytes);
                        }
                    }
                }

                // 若该任务标记为需解压 Natives，落盘后立即安全解压
                if let Some((natives_dir, excludes)) = task.extract_natives {
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
                        let (primary_url, backup_url) =
                            endpoints::resolve_mojang_download_urls(url_str, prefer_mirror);
                        tasks.push(BatchDownloadTask {
                            target_path: libraries_dir.join(rel_path),
                            primary_url,
                            backup_url: Some(backup_url),
                            expected_size: artifact.size,
                            extract_natives: extract_opt,
                        });
                    }
                }
            } else if parsed_lib.downloads.is_none() {
                let (rel_path, url_subpath) = Self::maven_to_path_and_url(&parsed_lib.name);
                let repo_base = parsed_lib.url.as_deref().or(default_maven_base);
                let (primary_url, backup_url) =
                    endpoints::resolve_maven_library_urls(repo_base, &url_subpath, prefer_mirror);
                tasks.push(BatchDownloadTask {
                    target_path: libraries_dir.join(rel_path),
                    primary_url,
                    backup_url: Some(backup_url),
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
                    let (primary_url, backup_url) =
                        endpoints::resolve_mojang_download_urls(url_str, prefer_mirror);
                    let extract_cls = natives_dir.map(|nd| {
                        (
                            nd.to_path_buf(),
                            parsed_lib.extract.as_ref().and_then(|e| e.exclude.clone()),
                        )
                    });
                    tasks.push(BatchDownloadTask {
                        target_path: libraries_dir.join(rel_path),
                        primary_url,
                        backup_url: Some(backup_url),
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

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| e.to_string())?;

        // 1. 获取并保存版本元数据 JSON
        emit_progress("正在获取原版元数据 JSON...", 5.0, "拉取清单", false, None);

        let (primary_meta_url, backup_meta_url) =
            endpoints::resolve_mojang_download_urls(&meta_url, prefer_mirror);

        let json_text =
            Self::fetch_text_with_fallback(&client, &primary_meta_url, &backup_meta_url)
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

        // 2. 下载 client.jar (10% ~ 40%)
        emit_progress(
            "正在准备下载游戏核心 (client.jar)...",
            10.0,
            "解析地址",
            false,
            None,
        );

        if let Some(ref downloads) = ver_detail.downloads {
            if let Some(ref client_art) = downloads.client {
                let (primary_client_url, backup_client_url) =
                    endpoints::resolve_mojang_download_urls(&client_art.url, prefer_mirror);

                let mut jar_resp = match client.get(&primary_client_url).send().await {
                    Ok(r) if r.status().is_success() => r,
                    _ => client
                        .get(&backup_client_url)
                        .send()
                        .await
                        .map_err(|e| format!("下载 client.jar 请求失败: {}", e))?,
                };

                let jar_file_path = version_dir.join(format!("{}.jar", final_target_name));
                let mut file = File::create(&jar_file_path).map_err(|e| e.to_string())?;

                let total_size = client_art.size.max(1) as f32;
                let mut downloaded: f32 = 0.0;

                while let Some(chunk) = jar_resp.chunk().await.map_err(|e| e.to_string())? {
                    file.write_all(&chunk).map_err(|e| e.to_string())?;
                    downloaded += chunk.len() as f32;

                    let pct = 10.0 + (downloaded / total_size) * 30.0;
                    let mb_text = format!(
                        "{:.1}MB / {:.1}MB",
                        downloaded / (1024.0 * 1024.0),
                        total_size / (1024.0 * 1024.0)
                    );
                    emit_progress("正在高速下载核心 JAR 文件...", pct, &mb_text, false, None);
                }
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
                &client,
                lib_tasks,
            )
            .await;
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

            if let Ok(idx_text) =
                Self::fetch_text_with_fallback(&client, &primary_idx_url, &backup_idx_url).await
            {
                let idx_path = indexes_dir.join(format!("{}.json", asset_meta.id));
                let _ = fs::write(&idx_path, &idx_text);

                if let Ok(index_file) = serde_json::from_str::<AssetIndexFile>(&idx_text) {
                    let mut asset_tasks = Vec::new();
                    let mut seen_hashes = std::collections::HashSet::new();

                    for entry in index_file.objects.values() {
                        if entry.hash.len() < 2 || !seen_hashes.insert(entry.hash.clone()) {
                            continue;
                        }
                        let prefix = &entry.hash[0..2];
                        let target_obj_path = objects_dir.join(prefix).join(&entry.hash);
                        if target_obj_path.is_file() {
                            if let Ok(m) = fs::metadata(&target_obj_path) {
                                if m.len() == entry.size {
                                    continue;
                                }
                            }
                        }
                        let (primary_url, backup_url) =
                            endpoints::resolve_asset_object_urls(&entry.hash, prefer_mirror);
                        asset_tasks.push(BatchDownloadTask {
                            target_path: target_obj_path,
                            primary_url,
                            backup_url: Some(backup_url),
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
                            &client,
                            asset_tasks,
                        )
                        .await;
                    }
                }
            }
        }

        // 5. 可选：自动合成加载器 (Fabric / Quilt / Forge / NeoForge) (85% ~ 98%)
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

                let mut loaders_opt: Option<Vec<FabricLoaderEntry>> = None;
                for url in &loader_urls {
                    if let Ok(resp) = client.get(url).send().await {
                        if let Ok(parsed) = resp.json::<Vec<FabricLoaderEntry>>().await {
                            loaders_opt = Some(parsed);
                            break;
                        }
                    }
                }

                if let Some(loaders) = loaders_opt {
                    let chosen_entry = if let Some(ref req_ver) = loader_version {
                        loaders.iter().find(|l| &l.loader.version == req_ver)
                    } else {
                        loaders
                            .iter()
                            .find(|l| l.loader.stable)
                            .or_else(|| loaders.first())
                    };

                    if let Some(entry) = chosen_entry {
                        let loader_ver = &entry.loader.version;
                        let profile_urls = endpoints::fabric_profile_json_urls(
                            &vanilla_version,
                            loader_ver,
                            prefer_mirror,
                        );

                        let mut profile_opt: Option<LoaderProfileJson> = None;
                        for p_url in &profile_urls {
                            if let Ok(resp) = client.get(p_url).send().await {
                                if let Ok(prof) = resp.json::<LoaderProfileJson>().await {
                                    profile_opt = Some(prof);
                                    break;
                                }
                            }
                        }

                        if let Some(fabric_profile) = profile_opt {
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
                                &client,
                                fab_tasks,
                            )
                            .await;

                            Self::merge_loader_profile_into_version_json(
                                &mut original_json_val,
                                fabric_profile,
                                &final_target_name,
                            );
                            if let Ok(merged_str) =
                                serde_json::to_string_pretty(&original_json_val)
                            {
                                let _ = fs::write(&json_file_path, merged_str);
                            }
                        }
                    }
                }
            }
            Some("quilt") => {
                if let Some(ref loader_ver) = loader_version {
                    let profile_urls = endpoints::quilt_profile_json_urls(
                        &vanilla_version,
                        loader_ver,
                        prefer_mirror,
                    );
                    for p_url in &profile_urls {
                        if let Ok(resp) = client.get(p_url).send().await {
                            if let Ok(quilt_profile) = resp.json::<LoaderProfileJson>().await {
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
                                    &client,
                                    quilt_tasks,
                                )
                                .await;

                                Self::merge_loader_profile_into_version_json(
                                    &mut original_json_val,
                                    quilt_profile,
                                    &final_target_name,
                                );
                                if let Ok(merged_str) =
                                    serde_json::to_string_pretty(&original_json_val)
                                {
                                    let _ = fs::write(&json_file_path, merged_str);
                                }
                                break;
                            }
                        }
                    }
                }
            }
            Some("forge") | Some("neoforge") => {
                let is_neoforge = loader_type.as_deref() == Some("neoforge");
                let label_name = if is_neoforge { "NeoForge" } else { "Forge" };
                if let Some(ref loader_ver) = loader_version {
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

                    if let Ok(inst_bytes) = Self::fetch_bytes_with_fallback(
                        &client,
                        &primary_inst_url,
                        Some(&backup_inst_url),
                    )
                    .await
                    {
                        emit_progress(
                            &format!("正在解包并合成 {} {} 元数据...", label_name, loader_ver),
                            90.0,
                            "解包合成",
                            false,
                            None,
                        );
                        if let Ok((loader_profile, extra_libs)) =
                            Self::extract_forge_or_neoforge_installer(&inst_bytes, &libraries_dir)
                        {
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
                                &client,
                                forge_tasks,
                            )
                            .await;

                            Self::merge_loader_profile_into_version_json(
                                &mut original_json_val,
                                loader_profile,
                                &final_target_name,
                            );
                            if let Ok(merged_str) =
                                serde_json::to_string_pretty(&original_json_val)
                            {
                                let _ = fs::write(&json_file_path, merged_str);
                            }
                        }
                    }
                }
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

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| e.to_string())?;

        let (primary_meta_url, backup_meta_url) =
            endpoints::resolve_mojang_download_urls(&meta_url, prefer_mirror);

        let meta_text =
            Self::fetch_text_with_fallback(&client, &primary_meta_url, &backup_meta_url)
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

        emit_progress("正在下载官方服务端核心...", 30.0, "下载中", false, None);

        let (primary_server_url, backup_server_url) =
            endpoints::resolve_mojang_download_urls(&server_art.url, prefer_mirror);

        let target_path = PathBuf::from(&target_dir);
        if !target_path.exists() {
            fs::create_dir_all(&target_path).map_err(|e| e.to_string())?;
        }

        let jar_name = format!("{}-server.jar", version_id);
        let server_jar_path = target_path.join(&jar_name);
        let bytes = Self::fetch_bytes_with_fallback(
            &client,
            &primary_server_url,
            Some(&backup_server_url),
        )
        .await?;

        fs::write(&server_jar_path, &bytes)
            .map_err(|e| format!("写入服务端 jar 失败: {}", e))?;

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
