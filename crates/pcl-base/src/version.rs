use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

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

/// 判断指定版本目录是否启用了独立版本隔离（目录内已存在 options.txt 或 saves）
#[inline]
pub fn is_version_isolated<P: AsRef<Path>>(version_dir: P) -> bool {
    let dir = version_dir.as_ref();
    dir.join("options.txt").exists() || dir.join("saves").exists()
}

/// 全仓唯一的游戏运行与数据目录决策点：
/// - 隔离版本 (has_indie == true) -> `versions/<name>`
/// - 非隔离版本 (has_indie == false) -> `game_dir` (.minecraft 根目录)
pub fn resolve_game_dir<P1: AsRef<Path>, P2: AsRef<Path>>(game_dir: P1, version_dir: P2) -> PathBuf {
    let g_dir = game_dir.as_ref();
    let v_dir = version_dir.as_ref();
    if is_version_isolated(v_dir) {
        v_dir.to_path_buf()
    } else {
        g_dir.to_path_buf()
    }
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
        let has_indie = is_version_isolated(dir);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_game_dir_isolated_vs_non_isolated() {
        let base_temp = std::env::temp_dir().join(format!("pcl_game_dir_test_{}", std::process::id()));
        let game_dir = base_temp.join(".minecraft");
        let v1_dir = game_dir.join("versions").join("1.21.4-Vanilla");
        let v2_dir = game_dir.join("versions").join("1.21.4-Fabric");
        let v3_dir = game_dir.join("versions").join("1.20.4-Forge");

        let _ = fs::create_dir_all(&v1_dir);
        let _ = fs::create_dir_all(&v2_dir);
        let _ = fs::create_dir_all(&v3_dir);

        // v1: 全新版本，无 options.txt 也无 saves -> 非隔离，指向 game_dir
        assert!(!is_version_isolated(&v1_dir));
        assert_eq!(resolve_game_dir(&game_dir, &v1_dir), game_dir);

        // v2: 存在 saves -> 隔离版本，指向 v2_dir
        fs::create_dir_all(v2_dir.join("saves")).unwrap();
        assert!(is_version_isolated(&v2_dir));
        assert_eq!(resolve_game_dir(&game_dir, &v2_dir), v2_dir);

        // v3: 存在 options.txt -> 隔离版本，指向 v3_dir
        fs::write(v3_dir.join("options.txt"), "fov:70.0").unwrap();
        assert!(is_version_isolated(&v3_dir));
        assert_eq!(resolve_game_dir(&game_dir, &v3_dir), v3_dir);

        let _ = fs::remove_dir_all(&base_temp);
    }
}
