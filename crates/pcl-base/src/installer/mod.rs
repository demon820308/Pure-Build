pub mod download;
pub mod forge;
pub mod libraries;
pub mod merge;
pub mod optifine;
pub mod server;

use crate::config::ConfigManager;
use crate::endpoints::{
    self, DEFAULT_ASSET_CONCURRENCY, DEFAULT_LIBRARY_CONCURRENCY,
};
use crate::platform;
use crate::reporter::ReporterHandle;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub use download::BatchDownloadTask;
pub use merge::LoaderProfileJson;

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
    #[allow(dead_code)]
    sha1: Option<String>,
    #[allow(dead_code)]
    size: Option<u64>,
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
    #[allow(dead_code)]
    server: Option<DownloadArtifact>,
}

#[derive(Debug, Deserialize)]
struct DownloadArtifact {
    url: String,
    size: u64,
    sha1: Option<String>,
}

pub struct VersionInstaller;

impl VersionInstaller {
    #[inline]
    pub fn is_file_valid(path: &Path, expected_size: Option<u64>, expected_sha1: Option<&str>) -> bool {
        download::is_file_valid(path, expected_size, expected_sha1)
    }

    #[inline]
    pub async fn download_file_with_retry<F>(
        client: &reqwest::Client,
        urls: &[String],
        target_path: &Path,
        expected_size: Option<u64>,
        expected_sha1: Option<&str>,
        on_progress: F,
    ) -> Result<(), String>
    where
        F: FnMut(u64, u64),
    {
        download::download_file_with_retry(client, urls, target_path, expected_size, expected_sha1, on_progress).await
    }

    #[inline]
    pub async fn fetch_bytes_with_fallback(
        client: &reqwest::Client,
        primary_url: &str,
        backup_url: Option<&str>,
    ) -> Result<Vec<u8>, String> {
        download::fetch_bytes_with_fallback(client, primary_url, backup_url).await
    }

    #[inline]
    pub async fn fetch_text_with_fallback(
        client: &reqwest::Client,
        primary_url: &str,
        backup_url: &str,
    ) -> Result<String, String> {
        download::fetch_text_with_fallback(client, primary_url, backup_url).await
    }

    #[inline]
    pub async fn execute_batch_downloads(
        reporter: &ReporterHandle,
        version_id: &str,
        vanilla_version: Option<&str>,
        stage_label: &str,
        start_pct: f32,
        end_pct: f32,
        concurrency: usize,
        client: &reqwest::Client,
        tasks: Vec<BatchDownloadTask>,
    ) -> Result<(), String> {
        download::execute_batch_downloads(
            reporter,
            version_id,
            vanilla_version,
            stage_label,
            start_pct,
            end_pct,
            concurrency,
            client,
            tasks,
        )
        .await
    }

    #[inline]
    pub fn maven_to_path_and_url(maven_coord: &str) -> (PathBuf, String) {
        libraries::maven_to_path_and_url(maven_coord)
    }

    #[inline]
    pub fn collect_library_tasks(
        lib_values: &[serde_json::Value],
        libraries_dir: &Path,
        natives_dir: Option<&Path>,
        prefer_mirror: bool,
        target_os: &str,
        target_arch: &str,
        default_maven_base: Option<&str>,
    ) -> Vec<BatchDownloadTask> {
        libraries::collect_library_tasks(
            lib_values,
            libraries_dir,
            natives_dir,
            prefer_mirror,
            target_os,
            target_arch,
            default_maven_base,
        )
    }

    #[inline]
    pub fn merge_loader_profile_into_version_json(
        original_json_val: &mut serde_json::Value,
        profile: LoaderProfileJson,
        final_target_name: &str,
    ) {
        merge::merge_loader_profile_into_version_json(original_json_val, profile, final_target_name)
    }

    #[inline]
    pub fn extract_forge_or_neoforge_installer(
        installer_bytes: &[u8],
        libraries_dir: &Path,
    ) -> Result<(LoaderProfileJson, Vec<serde_json::Value>), String> {
        forge::extract_forge_or_neoforge_installer(installer_bytes, libraries_dir)
    }

    #[inline]
    pub fn extract_optifine_installer(
        installer_bytes: &[u8],
        vanilla_version: &str,
        loader_ver: &str,
        client_jar_path: &Path,
        libraries_dir: &Path,
        final_target_name: &str,
        original_json_val: &serde_json::Value,
    ) -> Result<(LoaderProfileJson, Vec<serde_json::Value>), String> {
        optifine::extract_optifine_installer(
            installer_bytes,
            vanilla_version,
            loader_ver,
            client_jar_path,
            libraries_dir,
            final_target_name,
            original_json_val,
        )
    }

    #[inline]
    pub async fn install_server(
        reporter: ReporterHandle,
        target_dir: String,
        version_id: String,
        meta_url: String,
    ) -> Result<String, String> {
        server::install_server(reporter, target_dir, version_id, meta_url).await
    }

    /// 完整安装指定版本的原版核心、并发支持库、Natives、Assets 资源及可选 Loader (Fabric / Quilt / Forge / NeoForge / OptiFine)
    pub async fn install_version(
        reporter: ReporterHandle,
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
                reporter.emit_download(DownloadProgress {
                    version_id: final_target_name.clone(),
                    vanilla_version: Some(vanilla_version.clone()),
                    step: step.to_string(),
                    percentage: pct,
                    speed_text: speed.to_string(),
                    is_finished: finished,
                    error: err,
                });
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
            download::fetch_text_with_fallback(client, &primary_meta_url, &backup_meta_url)
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

        // 2. 下载或本地复制 client.jar (10% ~ 40%)
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

                if !download::is_file_valid(&jar_file_path, Some(client_art.size), client_art.sha1.as_deref())
                    && final_target_name != vanilla_version
                {
                    let vanilla_jar = game_dir
                        .join("versions")
                        .join(&vanilla_version)
                        .join(format!("{}.jar", vanilla_version));
                    if download::is_file_valid(&vanilla_jar, Some(client_art.size), client_art.sha1.as_deref()) {
                        let _ = fs::copy(&vanilla_jar, &jar_file_path);
                    }
                }

                let (primary_client_url, backup_client_url) =
                    endpoints::resolve_mojang_download_urls(&client_art.url, prefer_mirror);
                let client_urls = vec![primary_client_url, backup_client_url];
                let mut last_emit_pct = 0.0f32;

                download::download_file_with_retry(
                    client,
                    &client_urls,
                    &jar_file_path,
                    Some(client_art.size),
                    client_art.sha1.as_deref(),
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
            let lib_tasks = libraries::collect_library_tasks(
                orig_libs,
                &libraries_dir,
                Some(&natives_dir),
                prefer_mirror,
                target_os,
                target_arch,
                None,
            );
            download::execute_batch_downloads(
                &reporter,
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

        // 4. 下载并补全 Assets 资源索引与音频语言包对象 (65% ~ 85%)
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
                download::fetch_text_with_fallback(client, &primary_idx_url, &backup_idx_url)
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
                if download::is_file_valid(&target_obj_path, Some(entry.size), Some(&entry.hash)) {
                    continue;
                }
                let (primary_url, backup_url) =
                    endpoints::resolve_asset_object_urls(&entry.hash, prefer_mirror);
                asset_tasks.push(BatchDownloadTask {
                    target_path: target_obj_path,
                    urls: vec![primary_url, backup_url],
                    expected_size: Some(entry.size),
                    expected_sha1: Some(entry.hash.clone()),
                    extract_natives: None,
                });
            }

            if !asset_tasks.is_empty() {
                download::execute_batch_downloads(
                    &reporter,
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
            Some("fabric") | Some("quilt") => {
                let l_type = loader_type.as_deref().unwrap();
                merge::install_fabric_or_quilt_loader(
                    &reporter,
                    client,
                    &vanilla_version,
                    &final_target_name,
                    l_type,
                    loader_version.as_deref(),
                    &libraries_dir,
                    &json_file_path,
                    prefer_mirror,
                    target_os,
                    target_arch,
                    &mut original_json_val,
                )
                .await?;
            }
            Some("forge") | Some("neoforge") => {
                let l_type = loader_type.as_deref().unwrap();
                forge::install_forge_or_neoforge_loader(
                    &reporter,
                    client,
                    &vanilla_version,
                    &final_target_name,
                    l_type,
                    loader_version.as_deref(),
                    &libraries_dir,
                    &json_file_path,
                    prefer_mirror,
                    target_os,
                    target_arch,
                    &mut original_json_val,
                )
                .await?;
            }
            Some("optifine") => {
                optifine::install_optifine_loader(
                    &reporter,
                    client,
                    &vanilla_version,
                    &final_target_name,
                    loader_version.as_deref(),
                    &version_dir,
                    &libraries_dir,
                    &json_file_path,
                    prefer_mirror,
                    target_os,
                    target_arch,
                    &mut original_json_val,
                )
                .await?;
            }
            _ => {}
        }

        // 6. 安装完成
        emit_progress("安装完成！已成功加入版本列表", 100.0, "完成", true, None);
        Ok(())
    }
}
