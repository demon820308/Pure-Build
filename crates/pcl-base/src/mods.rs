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
    pub is_enabled: bool,
    pub icon_base64: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FabricModJson {
    id: Option<String>,
    name: Option<String>,
    version: Option<String>,
    description: Option<String>,
}

pub struct ModScanner;

impl ModScanner {
    /// 扫描指定版本隔离目录或全局下的 mods 文件夹
    pub fn scan_mods<P: AsRef<Path>>(version_dir: P, game_dir: P) -> Vec<ModInfo> {
        let v_mods = version_dir.as_ref().join("mods");
        let g_mods = game_dir.as_ref().join("mods");

        // 优先使用版本隔离目录下的 mods 文件夹，不存在则回退至全局 mods 文件夹
        let target_dir = if v_mods.is_dir() {
            v_mods
        } else if g_mods.is_dir() {
            g_mods
        } else {
            return Vec::new();
        };

        let mut mods = Vec::new();

        if let Ok(entries) = fs::read_dir(&target_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    let is_jar = file_name.ends_with(".jar");
                    let is_disabled = file_name.ends_with(".jar.disabled") || file_name.ends_with(".disabled");

                    if is_jar || is_disabled {
                        let info = Self::parse_mod_jar(&path, file_name, !is_disabled);
                        mods.push(info);
                    }
                }
            }
        }

        // 按名称排序
        mods.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        mods
    }

    /// 切换 Mod 启停状态（重命名为 .disabled 或去除 .disabled）
    pub fn toggle_mod<P: AsRef<Path>>(mods_dir: P, file_name: &str) -> Result<String, String> {
        let base = mods_dir.as_ref();
        let current_path = base.join(file_name);

        if !current_path.is_file() {
            return Err("目标 Mod 文件不存在".to_string());
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
            is_enabled,
            icon_base64: None,
        };

        // 尝试通过 zip 流式读取元数据
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
                            return mod_info;
                        }
                    }
                }

                // 2. 尝试解析 Forge: META-INF/mods.toml (简单正则抽取)
                if let Ok(mut forge_file) = archive.by_name("META-INF/mods.toml") {
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
                                    mod_info.version = val.trim().trim_matches('"').to_string();
                                }
                            }
                        }
                        return mod_info;
                    }
                }
            }
        }

        mod_info
    }
}
