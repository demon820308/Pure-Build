use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub id: String,
    pub name: String,
    pub version_type: String, // release, snapshot, modded
    pub loader: Option<String>, // Fabric, Forge, NeoForge, OptiFine
    pub release_time: Option<String>,
    pub main_class: Option<String>,
    pub has_indie: bool, // 是否启用了独立版本隔离
    pub path: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct VersionJson {
    id: Option<String>,
    #[serde(rename = "type")]
    json_type: Option<String>,
    #[serde(rename = "releaseTime")]
    release_time: Option<String>,
    #[serde(rename = "mainClass")]
    main_class: Option<String>,
    #[serde(rename = "inheritsFrom")]
    inherits_from: Option<String>,
}

pub struct VersionScanner;

impl VersionScanner {
    /// 扫描指定 .minecraft 根目录下的所有版本
    pub fn scan_versions<P: AsRef<Path>>(game_dir: P) -> Vec<VersionInfo> {
        let versions_dir = game_dir.as_ref().join("versions");
        let mut list = Vec::new();

        if !versions_dir.is_dir() {
            return list;
        }

        if let Ok(entries) = fs::read_dir(versions_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let folder_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    let json_path = path.join(format!("{}.json", folder_name));

                    if json_path.is_file() {
                        if let Some(version) = Self::parse_version(&path, &json_path, folder_name) {
                            list.push(version);
                        }
                    }
                }
            }
        }

        // 按名称排序
        list.sort_by(|a, b| b.id.cmp(&a.id));
        list
    }

    fn parse_version(dir: &Path, json_path: &Path, folder_name: &str) -> Option<VersionInfo> {
        let content = fs::read_to_string(json_path).ok()?;
        let json: VersionJson = serde_json::from_str(&content).ok()?;

        let mut loader = None;
        let main_class = json.main_class.as_deref().unwrap_or("");

        // 识别加载器类型
        if main_class.contains("fabricmc") || folder_name.contains("Fabric") {
            loader = Some("Fabric".to_string());
        } else if main_class.contains("forge") || folder_name.contains("Forge") {
            loader = Some("Forge".to_string());
        } else if main_class.contains("neoforged") || folder_name.contains("NeoForge") {
            loader = Some("NeoForge".to_string());
        } else if main_class.contains("optifine") || folder_name.contains("OptiFine") {
            loader = Some("OptiFine".to_string());
        }

        let is_modded = loader.is_some() || json.inherits_from.is_some();
        let version_type = if is_modded {
            "modded".to_string()
        } else {
            json.json_type.unwrap_or_else(|| "release".to_string())
        };

        // 判断是否有独立隔离目录（PCL / HMCL 常见特征：versions/<name>/options.txt 或 versions/<name>/saves）
        let has_indie = dir.join("options.txt").exists() || dir.join("saves").exists();

        Some(VersionInfo {
            id: folder_name.to_string(),
            name: folder_name.to_string(),
            version_type,
            loader,
            release_time: json.release_time,
            main_class: json.main_class,
            has_indie,
            path: dir.to_string_lossy().to_string(),
        })
    }
}
