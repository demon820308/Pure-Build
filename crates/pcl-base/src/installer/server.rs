use crate::config::ConfigManager;
use crate::endpoints;
use crate::installer::download::{download_file_with_retry, fetch_text_with_fallback};
use crate::installer::DownloadProgress;
use crate::platform;
use crate::reporter::ReporterHandle;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
struct ServerVersionDetailJson {
    downloads: Option<ServerVersionDownloads>,
}

#[derive(Debug, Deserialize)]
struct ServerVersionDownloads {
    server: Option<ServerDownloadArtifact>,
}

#[derive(Debug, Deserialize)]
struct ServerDownloadArtifact {
    url: String,
    size: u64,
    sha1: Option<String>,
}

/// 仅下载指定版本的服务端 jar 到目标路径（复用于开服安装与 Horizon 首次准备）
pub async fn download_server_jar(
    reporter: &ReporterHandle,
    progress_id: &str,
    version_id: &str,
    meta_url: &str,
    server_jar_path: &Path,
) -> Result<(), String> {
    let emit_progress = |step: &str, pct: f32, speed: &str, finished: bool, err: Option<String>| {
        reporter.emit_download(DownloadProgress {
            version_id: progress_id.to_string(),
            vanilla_version: Some(version_id.to_string()),
            step: step.to_string(),
            percentage: pct,
            speed_text: speed.to_string(),
            is_finished: finished,
            error: err,
        });
    };

    emit_progress("正在获取服务端元数据...", 10.0, "", false, None);

    let cfg = ConfigManager::load();
    let prefer_mirror = cfg.prefer_mirror;
    let client = endpoints::shared_http_client();

    let (primary_meta_url, backup_meta_url) =
        endpoints::resolve_mojang_download_urls(meta_url, prefer_mirror);

    let meta_text = fetch_text_with_fallback(client, &primary_meta_url, &backup_meta_url)
        .await
        .map_err(|e| format!("获取版本元数据失败: {}", e))?;

    let detail: ServerVersionDetailJson = serde_json::from_str(&meta_text)
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

    if let Some(parent) = server_jar_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }

    let server_urls = vec![primary_server_url, backup_server_url];
    let mut last_emit_pct = 0.0f32;

    download_file_with_retry(
        client,
        &server_urls,
        server_jar_path,
        Some(server_art.size),
        server_art.sha1.as_deref(),
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
    .map_err(|e| format!("下载服务端核心失败: {}", e))
}

/// 下载并配置 Minecraft 官方服务端（调用 platform::server_script 生成跨平台开服脚本并返回脚本名）
pub async fn install_server(
    reporter: ReporterHandle,
    target_dir: String,
    version_id: String,
    meta_url: String,
) -> Result<String, String> {
    let progress_id = format!("{}-server", version_id);
    let target_path = PathBuf::from(&target_dir);
    let jar_name = format!("{}-server.jar", version_id);
    let server_jar_path = target_path.join(&jar_name);

    download_server_jar(&reporter, &progress_id, &version_id, &meta_url, &server_jar_path).await?;

    reporter.emit_download(DownloadProgress {
        version_id: progress_id.clone(),
        vanilla_version: Some(version_id.clone()),
        step: "正在生成开服脚本与配置文件...".to_string(),
        percentage: 90.0,
        speed_text: "".to_string(),
        is_finished: false,
        error: None,
    });

    let cfg = ConfigManager::load();
    let script_name = platform::server_script_with_memory(
        &target_path,
        &version_id,
        &jar_name,
        cfg.max_memory_mb,
    )?;

    let eula_content = "#By changing the setting below to TRUE you are indicating your agreement to our EULA (https://aka.ms/MinecraftEULA).\neula=true\n";
    let _ = fs::write(target_path.join("eula.txt"), eula_content);

    reporter.emit_download(DownloadProgress {
        version_id: progress_id,
        vanilla_version: Some(version_id),
        step: "服务端与启动脚本已准备就绪！".to_string(),
        percentage: 100.0,
        speed_text: "完成".to_string(),
        is_finished: true,
        error: None,
    });
    Ok(script_name)
}
