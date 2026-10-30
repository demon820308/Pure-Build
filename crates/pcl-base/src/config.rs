use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub game_dir: String,
    pub max_memory_mb: u32,
    pub player_name: String,
    pub java_path: Option<String>,
    pub window_width: u32,
    pub window_height: u32,
    pub theme_hue: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            game_dir: r"D:\Program Files (x86)\.minecraft".to_string(),
            max_memory_mb: 4096,
            player_name: "Steve".to_string(),
            java_path: None,
            window_width: 854,
            window_height: 480,
            theme_hue: 205,
        }
    }
}

pub struct ConfigManager;

impl ConfigManager {
    fn config_path() -> PathBuf {
        PathBuf::from("pcl_config.json")
    }

    /// 读取持久化配置
    pub fn load() -> AppConfig {
        let path = Self::config_path();
        if path.is_file() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(cfg) = serde_json::from_str::<AppConfig>(&content) {
                    return cfg;
                }
            }
        }
        AppConfig::default()
    }

    /// 保存配置到本地 JSON
    pub fn save(cfg: &AppConfig) -> Result<(), String> {
        let content = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
        fs::write(Self::config_path(), content).map_err(|e| e.to_string())
    }
}
