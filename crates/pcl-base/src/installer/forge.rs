use crate::endpoints::{self, DEFAULT_LIBRARY_CONCURRENCY};
use crate::installer::download;
use crate::installer::libraries;
use crate::installer::merge::{self, LoaderProfileJson};
use crate::installer::DownloadProgress;
use crate::reporter::ReporterHandle;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Component, Path};
use zip::ZipArchive;

/// 解析 Forge / NeoForge Installer JAR，提取内嵌 maven 库并返回其 version.json 与额外依赖库
pub fn extract_forge_or_neoforge_installer(
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
            // 安全解压 installer 内置的 maven 构件到 libraries 目录（防 zip-slip）
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
            .map_err(|e| format!("解析 Forge version.json 失败: {}", e))?
    } else if !install_profile_str.is_empty() {
        let v: serde_json::Value = serde_json::from_str(&install_profile_str)
            .map_err(|e| format!("解析 Forge install_profile.json 失败: {}", e))?;
        if let Some(vi) = v.get("versionInfo") {
            serde_json::from_value(vi.clone())
                .map_err(|e| format!("解析 Forge versionInfo 结构失败: {}", e))?
        } else {
            return Err("Installer 中未找到 version.json 或 versionInfo 字段".to_string());
        }
    } else {
        return Err("Installer JAR 损坏，缺少 version.json".to_string());
    };

    let mut extra_libs = Vec::new();
    if !install_profile_str.is_empty() {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&install_profile_str) {
            if let Some(libs) = v.get("libraries").and_then(|l| l.as_array()) {
                extra_libs = libs.clone();
            }
        }
    }

    Ok((version_profile, extra_libs))
}

pub async fn install_forge_or_neoforge_loader(
    reporter: &ReporterHandle,
    client: &reqwest::Client,
    vanilla_version: &str,
    final_target_name: &str,
    loader_type: &str,
    loader_version: Option<&str>,
    libraries_dir: &Path,
    json_file_path: &Path,
    prefer_mirror: bool,
    target_os: &str,
    target_arch: &str,
    original_json_val: &mut serde_json::Value,
) -> Result<(), String> {
    let is_neoforge = loader_type == "neoforge";
    let label_name = if is_neoforge { "NeoForge" } else { "Forge" };
    let loader_ver = loader_version
        .ok_or_else(|| format!("未指定 {} 版本", label_name))?;

    reporter.emit_download(DownloadProgress {
        version_id: final_target_name.to_string(),
        vanilla_version: Some(vanilla_version.to_string()),
        step: format!("正在下载 {} {} 安装包...", label_name, loader_ver),
        percentage: 86.0,
        speed_text: "下载安装包".to_string(),
        is_finished: false,
        error: None,
    });

    let (primary_inst_url, backup_inst_url) = if is_neoforge {
        endpoints::neoforge_installer_urls(loader_ver, prefer_mirror)
    } else {
        endpoints::forge_installer_urls(vanilla_version, loader_ver, prefer_mirror)
    };

    // shortcut: Forge/NeoForge installer jar 属于分发安装包，未在版本清单中提供官方 SHA-1，维持大小与 ZIP 魔数校验
    let inst_bytes = download::fetch_bytes_with_fallback(
        client,
        &primary_inst_url,
        Some(&backup_inst_url),
    )
    .await
    .map_err(|e| format!("下载 {} {} 安装包失败: {}", label_name, loader_ver, e))?;

    reporter.emit_download(DownloadProgress {
        version_id: final_target_name.to_string(),
        vanilla_version: Some(vanilla_version.to_string()),
        step: format!("正在解包并分析 {} {}...", label_name, loader_ver),
        percentage: 89.0,
        speed_text: "解包分析".to_string(),
        is_finished: false,
        error: None,
    });

    let (loader_profile, extra_install_libs) =
        extract_forge_or_neoforge_installer(&inst_bytes, libraries_dir)?;

    let default_repo = if is_neoforge {
        endpoints::NEOFORGE_MAVEN_OFFICIAL
    } else {
        endpoints::FORGE_MAVEN_OFFICIAL
    };

    let mut all_forge_libs = loader_profile.libraries.clone();
    for ext in extra_install_libs {
        all_forge_libs.push(ext);
    }

    let forge_tasks = libraries::collect_library_tasks(
        &all_forge_libs,
        libraries_dir,
        None,
        prefer_mirror,
        target_os,
        target_arch,
        Some(default_repo),
    );
    download::execute_batch_downloads(
        reporter,
        final_target_name,
        Some(vanilla_version),
        &format!("正在并发下载 {} {} 依赖库", label_name, loader_ver),
        91.0,
        98.0,
        DEFAULT_LIBRARY_CONCURRENCY,
        client,
        forge_tasks,
    )
    .await?;

    merge::merge_loader_profile_into_version_json(
        original_json_val,
        loader_profile,
        final_target_name,
    );
    let merged_str = serde_json::to_string_pretty(original_json_val)
        .map_err(|e| format!("序列化合并后的版本 JSON 失败: {}", e))?;
    fs::write(json_file_path, merged_str)
        .map_err(|e| format!("写入合并后的版本 JSON 失败: {}", e))?;

    Ok(())
}
