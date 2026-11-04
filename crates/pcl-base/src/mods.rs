use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModInfo {
    pub file_name: String,
    pub name: String,
    pub mod_id: String,
    pub version: String,
    pub description: Option<String>,
    pub authors: Vec<String>,
    pub is_enabled: bool,
    pub icon_base64: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FabricModJson {
    id: Option<String>,
    name: Option<String>,
    version: Option<String>,
    description: Option<String>,
    authors: Option<Vec<serde_json::Value>>,
}

pub struct ModScanner;

impl ModScanner {
    /// 扫描指定版本依据隔离策略解析出的游戏根目录下的 mods 文件夹
    pub fn scan_mods<P: AsRef<Path>>(version_dir: P, game_dir: P) -> Vec<ModInfo> {
        let target_dir = crate::version::resolve_game_dir(game_dir, version_dir).join("mods");
        if !target_dir.is_dir() {
            return Vec::new();
        }

        let mut mods = Vec::new();

        if let Ok(entries) = fs::read_dir(&target_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    let is_jar = file_name.ends_with(".jar");
                    let is_disabled =
                        file_name.ends_with(".jar.disabled") || file_name.ends_with(".disabled");

                    if is_jar || is_disabled {
                        let info = Self::parse_mod_jar(&path, file_name, !is_disabled);
                        mods.push(info);
                    }
                }
            }
        }

        mods.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        mods
    }

    /// 切换 Mod 启停状态（重命名为 .disabled 或去除 .disabled）
    pub fn toggle_mod<P: AsRef<Path>>(mods_dir: P, file_name: &str) -> Result<String, String> {
        let base = mods_dir.as_ref();
        let current_path = base.join(file_name);

        if !current_path.exists() {
            return Err("目标文件不存在".to_string());
        }

        let new_name = if file_name.ends_with(".disabled") {
            file_name.trim_end_matches(".disabled").to_string()
        } else {
            format!("{}.disabled", file_name)
        };

        let new_path = base.join(&new_name);
        fs::rename(&current_path, &new_path).map_err(|e| format!("重命名失败: {}", e))?;

        Ok(new_name)
    }

    /// 删除指定目录中的 Mod 或资源包文件
    pub fn delete_item<P: AsRef<Path>>(target_dir: P, file_name: &str) -> Result<(), String> {
        let safe_name = Path::new(file_name)
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or("非法的文件名")?;
        let path = target_dir.as_ref().join(safe_name);
        if path.is_dir() {
            fs::remove_dir_all(&path).map_err(|e| format!("删除文件夹失败: {}", e))
        } else if path.is_file() {
            fs::remove_file(&path).map_err(|e| format!("删除文件失败: {}", e))
        } else {
            Err("目标文件不存在".to_string())
        }
    }

    fn parse_mod_jar(path: &Path, file_name: &str, is_enabled: bool) -> ModInfo {
        let default_name = file_name
            .trim_end_matches(".disabled")
            .trim_end_matches(".jar")
            .to_string();

        let mut mod_info = ModInfo {
            file_name: file_name.to_string(),
            name: default_name.clone(),
            mod_id: default_name,
            version: "1.0.0".to_string(),
            description: None,
            authors: Vec::new(),
            is_enabled,
            icon_base64: None,
        };

        if let Ok(file) = File::open(path) {
            if let Ok(mut archive) = ZipArchive::new(file) {
                // 1. 尝试解析 Fabric: fabric.mod.json
                if let Ok(mut fabric_file) = archive.by_name("fabric.mod.json") {
                    let mut content = String::new();
                    if fabric_file.read_to_string(&mut content).is_ok() {
                        if let Ok(json) = serde_json::from_str::<FabricModJson>(&content) {
                            if let Some(n) = json.name {
                                mod_info.name = n;
                            }
                            if let Some(id) = json.id {
                                mod_info.mod_id = id;
                            }
                            if let Some(v) = json.version {
                                mod_info.version = v;
                            }
                            mod_info.description = json.description;
                            if let Some(raw_authors) = json.authors {
                                for a in raw_authors {
                                    if let Some(s) = a.as_str() {
                                        mod_info.authors.push(s.to_string());
                                    } else if let Some(n) = a.get("name").and_then(|v| v.as_str()) {
                                        mod_info.authors.push(n.to_string());
                                    }
                                }
                            }
                            return mod_info;
                        }
                    }
                }

                // 2. 尝试解析 Forge / NeoForge: META-INF/mods.toml 或 META-INF/neoforge.mods.toml
                for toml_name in ["META-INF/neoforge.mods.toml", "META-INF/mods.toml"] {
                    if let Ok(mut forge_file) = archive.by_name(toml_name) {
                        let mut content = String::new();
                        if forge_file.read_to_string(&mut content).is_ok() {
                            for line in content.lines() {
                                let line = line.trim();
                                if line.starts_with("displayName") {
                                    if let Some(val) = line.split('=').nth(1) {
                                        mod_info.name = val.trim().trim_matches('"').to_string();
                                    }
                                } else if line.starts_with("modId") {
                                    if let Some(val) = line.split('=').nth(1) {
                                        mod_info.mod_id = val.trim().trim_matches('"').to_string();
                                    }
                                } else if line.starts_with("version") {
                                    if let Some(val) = line.split('=').nth(1) {
                                        let v = val.trim().trim_matches('"');
                                        if v != "${file.jarVersion}" {
                                            mod_info.version = v.to_string();
                                        }
                                    }
                                } else if line.starts_with("authors") {
                                    if let Some(val) = line.split('=').nth(1) {
                                        let a = val.trim().trim_matches('"');
                                        if !a.is_empty() {
                                            mod_info.authors = vec![a.to_string()];
                                        }
                                    }
                                }
                            }
                            return mod_info;
                        }
                    }
                }
            }
        }

        mod_info
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::PathBuf;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    struct TempTestDir(PathBuf);
    impl TempTestDir {
        fn new(name: &str) -> Self {
            let p = std::env::temp_dir().join(format!(
                "pcl_mods_test_{}_{}",
                name,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            fs::create_dir_all(&p).unwrap();
            Self(p)
        }
    }
    impl Drop for TempTestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn test_mods_scan_toggle_and_delete_in_tempdir() {
        let temp = TempTestDir::new("lifecycle");
        let mods_dir = temp.0.join("mods");
        fs::create_dir_all(&mods_dir).unwrap();

        // 创建带 fabric.mod.json 的真实 jar 文件
        let jar_path = mods_dir.join("sodium-0.5.8.jar");
        let file = File::create(&jar_path).unwrap();
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        zip.start_file("fabric.mod.json", options).unwrap();
        let fabric_json = r#"{
            "schemaVersion": 1,
            "id": "sodium",
            "version": "0.5.8",
            "name": "Sodium",
            "description": "Modern rendering engine for Minecraft",
            "authors": ["jellysquid3_"]
        }"#;
        zip.write_all(fabric_json.as_bytes()).unwrap();
        zip.finish().unwrap();

        // 1. 扫描启用状态
        let scanned = ModScanner::scan_mods(&temp.0, &temp.0);
        assert_eq!(scanned.len(), 1);
        let m = &scanned[0];
        assert_eq!(m.mod_id, "sodium");
        assert_eq!(m.name, "Sodium");
        assert_eq!(m.version, "0.5.8");
        assert!(m.is_enabled);
        assert_eq!(m.file_name, "sodium-0.5.8.jar");

        // 2. 切换为禁用
        let disabled_name = ModScanner::toggle_mod(&mods_dir, "sodium-0.5.8.jar").unwrap();
        assert_eq!(disabled_name, "sodium-0.5.8.jar.disabled");
        assert!(!mods_dir.join("sodium-0.5.8.jar").exists());
        assert!(mods_dir.join("sodium-0.5.8.jar.disabled").exists());

        let scanned2 = ModScanner::scan_mods(&temp.0, &temp.0);
        assert_eq!(scanned2.len(), 1);
        assert!(!scanned2[0].is_enabled);
        assert_eq!(scanned2[0].mod_id, "sodium");

        // 3. 再次切换恢复启用
        let reenabled = ModScanner::toggle_mod(&mods_dir, "sodium-0.5.8.jar.disabled").unwrap();
        assert_eq!(reenabled, "sodium-0.5.8.jar");
        assert!(mods_dir.join("sodium-0.5.8.jar").exists());

        // 4. 删除文件
        ModScanner::delete_item(&mods_dir, "sodium-0.5.8.jar").unwrap();
        assert!(!mods_dir.join("sodium-0.5.8.jar").exists());

        let scanned3 = ModScanner::scan_mods(&temp.0, &temp.0);
        assert_eq!(scanned3.len(), 0);
    }
}
