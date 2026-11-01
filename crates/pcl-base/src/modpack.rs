//! 整合包（Modpack）导入与导出引擎。
//! 支持 Modrinth `.mrpack` 标准格式整合包的解析、底层核心与加载器自动安装、并发下载模组资源及解压 `overrides`。
//! 严格遵循项目反写死与防 zip-slip 规范（§2 & §4）。

use crate::config::ConfigManager;
use crate::download::DownloadManager;
use crate::endpoints::{self, DEFAULT_LIBRARY_CONCURRENCY};
use crate::installer::{DownloadProgress, VersionInstaller};
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
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModpackIndex {
    #[serde(rename = "formatVersion")]
    pub format_version: u32,
    pub game: String,
    #[serde(rename = "versionId")]
    pub version_id: String,
    pub name: String,
    pub summary: Option<String>,
    pub files: Vec<ModpackFileEntry>,
    pub dependencies: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModpackFileEntry {
    pub path: String,
    pub hashes: HashMap<String, String>,
    pub env: Option<HashMap<String, String>>,
    pub downloads: Vec<String>,
    #[serde(rename = "fileSize")]
    pub file_size: u64,
}

pub struct ModpackManager;

impl ModpackManager {
    /// 将整合包内的相对路径字符串安全转换为位于 `base_dir` 下的 `PathBuf`（防止 `../` 逃逸，§4）
    pub fn sanitize_relative_path(base_dir: &Path, raw_rel: &str) -> Option<PathBuf> {
        let mut out = base_dir.to_path_buf();
        let mut has_segment = false;
        for seg in raw_rel.split(['/', '\\']) {
            if seg.is_empty() || seg == "." {
                continue;
            }
            if seg == ".." {
                return None;
            }
            out = out.join(seg);
            has_segment = true;
        }
        if has_segment {
            Some(out)
        } else {
            None
        }
    }

    /// 导入本地 `.mrpack` 整合包到指定的 `.minecraft` 目录
    pub async fn import_mrpack(
        app: AppHandle,
        game_dir_str: String,
        mrpack_path_str: String,
        custom_instance_name: Option<String>,
    ) -> Result<String, String> {
        let mrpack_path = Path::new(&mrpack_path_str);
        if !mrpack_path.is_file() {
            return Err("找不到指定的整合包文件".to_string());
        }

        // 1. 读取并解析 modrinth.index.json
        let index: ModpackIndex = {
            let file = File::open(mrpack_path).map_err(|e| format!("打开整合包失败: {}", e))?;
            let mut archive =
                ZipArchive::new(file).map_err(|e| format!("解析整合包压缩结构失败: {}", e))?;
            let mut index_entry = archive
                .by_name("modrinth.index.json")
                .map_err(|_| "该压缩包不是有效的 Modrinth (.mrpack) 整合包（缺失 modrinth.index.json）".to_string())?;
            let mut idx_str = String::new();
            index_entry
                .read_to_string(&mut idx_str)
                .map_err(|e| e.to_string())?;
            serde_json::from_str(&idx_str)
                .map_err(|e| format!("解析 modrinth.index.json 失败: {}", e))?
        };

        let instance_name = custom_instance_name
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| index.name.trim().to_string());

        let emit_progress =
            |step: &str, pct: f32, speed: &str, finished: bool, err: Option<String>| {
                let _ = app.emit(
                    "download_progress",
                    DownloadProgress {
                        version_id: instance_name.clone(),
                        vanilla_version: None,
                        step: step.to_string(),
                        percentage: pct,
                        speed_text: speed.to_string(),
                        is_finished: finished,
                        error: err,
                    },
                );
            };

        emit_progress(
            &format!("正在解析整合包 [{}] 环境依赖...", instance_name),
            5.0,
            "解析依赖",
            false,
            None,
        );

        let mc_version = index
            .dependencies
            .get("minecraft")
            .cloned()
            .ok_or_else(|| "整合包未声明目标 Minecraft 版本".to_string())?;

        // 识别加载器类型与版本
        let (loader_type, loader_version) = if let Some(v) = index.dependencies.get("fabric-loader")
        {
            (Some("fabric".to_string()), Some(v.clone()))
        } else if let Some(v) = index.dependencies.get("quilt-loader") {
            (Some("quilt".to_string()), Some(v.clone()))
        } else if let Some(v) = index.dependencies.get("neoforge") {
            (Some("neoforge".to_string()), Some(v.clone()))
        } else if let Some(v) = index.dependencies.get("forge") {
            (Some("forge".to_string()), Some(v.clone()))
        } else {
            (None, None)
        };

        // 2. 获取官方版本清单，找到对应 mc_version 的 meta_url 并安装底层游戏核心与加载器
        let remote_versions = DownloadManager::fetch_version_manifest().await?;
        let target_remote = remote_versions
            .into_iter()
            .find(|v| v.id == mc_version)
            .ok_or_else(|| format!("未在官方清单中找到 Minecraft {}", mc_version))?;

        VersionInstaller::install_version(
            app.clone(),
            game_dir_str.clone(),
            mc_version,
            Some(instance_name.clone()),
            target_remote.url,
            loader_type,
            loader_version,
        )
        .await?;

        let game_dir = Path::new(&game_dir_str);
        let version_dir = game_dir.join("versions").join(&instance_name);

        // 3. 高并发下载整合包清单中的所有客户端资源文件 (Mods / Resourcepacks / Shaders)
        let client_files: Vec<ModpackFileEntry> = index
            .files
            .into_iter()
            .filter(|f| {
                if let Some(ref env_map) = f.env {
                    if let Some(client_req) = env_map.get("client") {
                        return client_req != "unsupported";
                    }
                }
                true
            })
            .collect();

        let total_files = client_files.len();
        if total_files > 0 {
            let client = endpoints::shared_http_client();
            let prefer_mirror = ConfigManager::load().prefer_mirror;

            let semaphore = Arc::new(Semaphore::new(DEFAULT_LIBRARY_CONCURRENCY));
            let completed = Arc::new(AtomicUsize::new(0));
            let mut join_set = JoinSet::new();

            for file_entry in client_files {
                let Some(dest_path) = Self::sanitize_relative_path(&version_dir, &file_entry.path)
                else {
                    continue;
                };
                let sem = semaphore.clone();
                let comp = completed.clone();
                let app_clone = app.clone();
                let inst_name = instance_name.clone();

                let mut candidate_urls = Vec::new();
                for u in &file_entry.downloads {
                    for resolved in endpoints::resolve_modrinth_file_urls(u, prefer_mirror) {
                        if !candidate_urls.contains(&resolved) {
                            candidate_urls.push(resolved);
                        }
                    }
                }

                join_set.spawn(async move {
                    let _permit = sem.acquire_owned().await.ok();
                    let expected = if file_entry.file_size > 0 {
                        Some(file_entry.file_size)
                    } else {
                        None
                    };

                    let res = VersionInstaller::download_file_with_retry(
                        client,
                        &candidate_urls,
                        &dest_path,
                        expected,
                        |_, _| {},
                    )
                    .await;

                    let done = comp.fetch_add(1, Ordering::Relaxed) + 1;
                    let pct = 85.0 + (done as f32 / total_files as f32) * 12.0;
                    let _ = app_clone.emit(
                        "download_progress",
                        DownloadProgress {
                            version_id: inst_name,
                            vanilla_version: None,
                            step: format!("正在并发下载整合包模组与资源 ({}/{})...", done, total_files),
                            percentage: pct,
                            speed_text: format!("{}/{}", done, total_files),
                            is_finished: false,
                            error: None,
                        },
                    );
                    res.map_err(|e| format!("{} ({})", file_entry.path, e))
                });
            }

            let mut failed = Vec::new();
            while let Some(joined) = join_set.join_next().await {
                if let Ok(Err(e)) = joined {
                    failed.push(e);
                }
            }
            if !failed.is_empty() {
                let preview: Vec<&str> = failed.iter().take(3).map(|s| s.as_str()).collect();
                return Err(format!(
                    "整合包有 {} 个文件下载失败: {}",
                    failed.len(),
                    preview.join("; ")
                ));
            }
        }

        // 4. 解压 overrides/ 与 client-overrides/ 覆盖文件到版本隔离目录（遵循 §4 防 zip-slip）
        emit_progress("正在释放整合包配置文件 (overrides)...", 98.0, "解压配置", false, None);
        {
            let file = File::open(mrpack_path).map_err(|e| e.to_string())?;
            let mut archive = ZipArchive::new(file).map_err(|e| e.to_string())?;

            for i in 0..archive.len() {
                let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
                if entry.is_dir() {
                    continue;
                }
                let Some(enclosed) = entry.enclosed_name() else {
                    continue;
                };

                let mut comps = enclosed.components();
                let Some(Component::Normal(first)) = comps.next() else {
                    continue;
                };
                let prefix = first.to_string_lossy();
                if prefix != "overrides" && prefix != "client-overrides" {
                    continue;
                }

                let mut dest = version_dir.clone();
                let mut valid = false;
                for c in comps {
                    if let Component::Normal(part) = c {
                        dest = dest.join(part);
                        valid = true;
                    }
                }
                if !valid {
                    continue;
                }

                if let Some(parent) = dest.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                if let Ok(mut out_f) = File::create(&dest) {
                    let _ = io::copy(&mut entry, &mut out_f);
                }
            }
        }

        emit_progress(
            &format!("整合包 [{}] 导入完成！", instance_name),
            100.0,
            "完成",
            true,
            None,
        );
        Ok(instance_name)
    }

    /// 将本地指定版本实例导出为 `.mrpack` 整合包文件
    pub fn export_instance_to_mrpack(
        version_path_str: &str,
        game_dir_str: &str,
        version_id: &str,
        pack_name: &str,
        pack_version: &str,
        output_file_str: &str,
    ) -> Result<String, String> {
        let version_dir = Path::new(version_path_str);
        let game_dir = Path::new(game_dir_str);
        let out_path = Path::new(output_file_str);

        let file = File::create(out_path).map_err(|e| format!("创建导出文件失败: {}", e))?;
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        let mut dependencies = HashMap::new();
        dependencies.insert("minecraft".to_string(), version_id.to_string());

        let index = ModpackIndex {
            format_version: 1,
            game: "minecraft".to_string(),
            version_id: pack_version.to_string(),
            name: pack_name.to_string(),
            summary: Some("Exported by Plain Craft Launcher 2 (Vue3)".to_string()),
            files: Vec::new(),
            dependencies,
        };

        let index_json =
            serde_json::to_string_pretty(&index).map_err(|e| format!("序列化索引失败: {}", e))?;
        zip.start_file("modrinth.index.json", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(index_json.as_bytes())
            .map_err(|e| e.to_string())?;

        // 将 mods / config / resourcepacks / shaderpacks 写入 overrides/
        let folders_to_include = ["mods", "config", "resourcepacks", "shaderpacks"];
        for folder in folders_to_include {
            let v_sub = version_dir.join(folder);
            let g_sub = game_dir.join(folder);
            let source_dir = if v_sub.is_dir() {
                v_sub
            } else if g_sub.is_dir() {
                g_sub
            } else {
                continue;
            };

            for entry in WalkDir::new(&source_dir).into_iter().flatten() {
                let p = entry.path();
                if p.is_file() {
                    if let Ok(rel) = p.strip_prefix(&source_dir) {
                        let mut zip_entry_parts = vec!["overrides".to_string(), folder.to_string()];
                        for comp in rel.components() {
                            if let Component::Normal(part) = comp {
                                zip_entry_parts.push(part.to_string_lossy().to_string());
                            }
                        }
                        let zip_entry_name = zip_entry_parts.join("/");
                        if zip.start_file(&zip_entry_name, options).is_ok() {
                            if let Ok(mut src_f) = File::open(p) {
                                let _ = io::copy(&mut src_f, &mut zip);
                            }
                        }
                    }
                }
            }
        }

        zip.finish().map_err(|e| format!("完成压缩包写入失败: {}", e))?;
        Ok(out_path.to_string_lossy().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_relative_path_rejects_zip_slip() {
        let base = Path::new("/tmp/instance");
        assert!(ModpackManager::sanitize_relative_path(base, "../evil.jar").is_none());
        assert!(ModpackManager::sanitize_relative_path(base, "mods/../../etc/passwd").is_none());
        let valid = ModpackManager::sanitize_relative_path(base, "mods/sodium-fabric.jar").unwrap();
        assert_eq!(valid, base.join("mods").join("sodium-fabric.jar"));
    }
}
