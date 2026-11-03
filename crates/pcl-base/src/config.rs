use crate::auth::UserProfile;
use crate::endpoints::{
    CONFIG_FILE_NAME, CREDENTIALS_FILE_NAME, DEFAULT_BG_BLUR, DEFAULT_BG_OPACITY,
    DEFAULT_HIDE_YGGDRASIL_LOGIN, DEFAULT_MAX_MEMORY_MB, DEFAULT_MUSIC_AUTO_PLAY,
    DEFAULT_MUSIC_COMPRESSOR, DEFAULT_MUSIC_CROSSFADE, DEFAULT_MUSIC_EQ_PRESET,
    DEFAULT_MUSIC_PAUSE_ON_LAUNCH, DEFAULT_MUSIC_PLAY_MODE, DEFAULT_MUSIC_VOLUME,
    DEFAULT_PLAYER_NAME, DEFAULT_THEME_HUE, DEFAULT_UI_STYLE, DEFAULT_WINDOW_HEIGHT,
    DEFAULT_WINDOW_WIDTH,
};
use crate::platform;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

fn default_prefer_mirror() -> bool {
    true
}

fn default_bg_blur() -> u32 {
    DEFAULT_BG_BLUR
}

fn default_bg_opacity() -> u32 {
    DEFAULT_BG_OPACITY
}

fn default_ui_style() -> String {
    DEFAULT_UI_STYLE.to_string()
}

fn default_hide_yggdrasil_login() -> bool {
    DEFAULT_HIDE_YGGDRASIL_LOGIN
}

fn default_music_volume() -> u32 {
    DEFAULT_MUSIC_VOLUME
}

fn default_music_play_mode() -> String {
    DEFAULT_MUSIC_PLAY_MODE.to_string()
}

fn default_music_auto_play() -> bool {
    DEFAULT_MUSIC_AUTO_PLAY
}

fn default_music_pause_on_launch() -> bool {
    DEFAULT_MUSIC_PAUSE_ON_LAUNCH
}

fn default_music_eq_preset() -> String {
    DEFAULT_MUSIC_EQ_PRESET.to_string()
}

fn default_music_eq_custom() -> Vec<f32> {
    vec![0.0, 0.0, 0.0, 0.0, 0.0]
}

fn default_music_compressor() -> bool {
    DEFAULT_MUSIC_COMPRESSOR
}

fn default_music_crossfade() -> bool {
    DEFAULT_MUSIC_CROSSFADE
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub game_dir: String,
    pub max_memory_mb: u32,
    pub player_name: String,
    pub java_path: Option<String>,
    pub window_width: u32,
    pub window_height: u32,
    pub theme_hue: u32,
    #[serde(default = "default_prefer_mirror")]
    pub prefer_mirror: bool,
    #[serde(default)]
    pub custom_bg_url: Option<String>,
    #[serde(default = "default_bg_blur")]
    pub bg_blur: u32,
    #[serde(default = "default_bg_opacity")]
    pub bg_opacity: u32,
    #[serde(default)]
    pub custom_homepage_notice: Option<String>,
    #[serde(default = "default_ui_style")]
    pub ui_style: String,
    #[serde(default = "default_hide_yggdrasil_login")]
    pub hide_yggdrasil_login: bool,
    #[serde(default)]
    pub music_dirs: Vec<String>,
    #[serde(default = "default_music_volume")]
    pub music_volume: u32,
    #[serde(default = "default_music_play_mode")]
    pub music_play_mode: String,
    #[serde(default = "default_music_auto_play")]
    pub music_auto_play: bool,
    #[serde(default = "default_music_pause_on_launch")]
    pub music_pause_on_launch: bool,
    #[serde(default = "default_music_eq_preset")]
    pub music_eq_preset: String,
    #[serde(default = "default_music_eq_custom")]
    pub music_eq_custom: Vec<f32>,
    #[serde(default = "default_music_compressor")]
    pub music_compressor: bool,
    #[serde(default = "default_music_crossfade")]
    pub music_crossfade: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            game_dir: platform::default_game_dir().to_string_lossy().to_string(),
            max_memory_mb: DEFAULT_MAX_MEMORY_MB,
            player_name: DEFAULT_PLAYER_NAME.to_string(),
            java_path: None,
            window_width: DEFAULT_WINDOW_WIDTH,
            window_height: DEFAULT_WINDOW_HEIGHT,
            theme_hue: DEFAULT_THEME_HUE,
            prefer_mirror: default_prefer_mirror(),
            custom_bg_url: None,
            bg_blur: DEFAULT_BG_BLUR,
            bg_opacity: DEFAULT_BG_OPACITY,
            custom_homepage_notice: None,
            ui_style: default_ui_style(),
            hide_yggdrasil_login: default_hide_yggdrasil_login(),
            music_dirs: Vec::new(),
            music_volume: default_music_volume(),
            music_play_mode: default_music_play_mode(),
            music_auto_play: default_music_auto_play(),
            music_pause_on_launch: default_music_pause_on_launch(),
            music_eq_preset: default_music_eq_preset(),
            music_eq_custom: default_music_eq_custom(),
            music_compressor: default_music_compressor(),
            music_crossfade: default_music_crossfade(),
        }
    }
}

pub struct ConfigManager;

impl ConfigManager {
    pub fn config_path() -> PathBuf {
        platform::app_config_dir().join(CONFIG_FILE_NAME)
    }

    pub fn credentials_path() -> PathBuf {
        platform::app_config_dir().join(CREDENTIALS_FILE_NAME)
    }

    /// 读取持久化配置（若文件不存在或 game_dir 为空则使用平台推导默认值）
    pub fn load() -> AppConfig {
        let path = Self::config_path();
        if path.is_file() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(mut cfg) = serde_json::from_str::<AppConfig>(&content) {
                    if cfg.game_dir.trim().is_empty() {
                        cfg.game_dir = platform::default_game_dir().to_string_lossy().to_string();
                    }
                    return cfg;
                }
            }
        }
        AppConfig::default()
    }

    /// 保存配置到 app_config_dir 下的 JSON 文件
    pub fn save(cfg: &AppConfig) -> Result<(), String> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("创建配置目录失败: {}", e))?;
        }
        let content = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
        fs::write(path, content).map_err(|e| format!("写入配置文件失败: {}", e))
    }

    /// 从 app_config_dir 下的凭据文件读取用户登录 Profile（替代前端 localStorage）
    pub fn load_profile() -> Option<UserProfile> {
        let path = Self::credentials_path();
        if path.is_file() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(profile) = serde_json::from_str::<UserProfile>(&content) {
                    return Some(profile);
                }
            }
        }
        None
    }

    /// 保存或清除用户登录凭据至 app_config_dir 下的凭据文件
    pub fn save_profile(profile: Option<&UserProfile>) -> Result<(), String> {
        let path = Self::credentials_path();
        match profile {
            Some(p) => {
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent).map_err(|e| format!("创建凭据目录失败: {}", e))?;
                }
                let content = serde_json::to_string_pretty(p).map_err(|e| e.to_string())?;
                fs::write(&path, content).map_err(|e| format!("写入凭据文件失败: {}", e))?;
                platform::restrict_credentials_permissions(&path);
                Ok(())
            }
            None => {
                if path.exists() {
                    fs::remove_file(&path).map_err(|e| format!("清除凭据文件失败: {}", e))?;
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_config_audio_tuning_defaults_and_serde() {
        let default_cfg = AppConfig::default();
        assert_eq!(default_cfg.music_eq_preset, "flat");
        assert_eq!(default_cfg.music_eq_custom, vec![0.0, 0.0, 0.0, 0.0, 0.0]);
        assert!(default_cfg.music_compressor);
        assert!(default_cfg.music_crossfade);

        // 测试旧版本缺失字段的反序列化回退兼容
        let old_json = r#"{
            "game_dir": "/mock/game",
            "max_memory_mb": 4096,
            "player_name": "Alex",
            "window_width": 854,
            "window_height": 480,
            "theme_hue": 205
        }"#;
        let parsed: AppConfig = serde_json::from_str(old_json).expect("旧配置解析失败");
        assert_eq!(parsed.music_eq_preset, "flat");
        assert_eq!(parsed.music_eq_custom, vec![0.0, 0.0, 0.0, 0.0, 0.0]);
        assert!(parsed.music_compressor);
        assert!(parsed.music_crossfade);

        // 测试自定义音效序列化回环
        let mut custom_cfg = default_cfg;
        custom_cfg.music_eq_preset = "ambient".to_string();
        custom_cfg.music_eq_custom = vec![3.0, 1.5, -2.0, 2.5, 4.0];
        custom_cfg.music_compressor = false;
        let serialized = serde_json::to_string(&custom_cfg).unwrap();
        let deserialized: AppConfig = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.music_eq_preset, "ambient");
        assert_eq!(deserialized.music_eq_custom, vec![3.0, 1.5, -2.0, 2.5, 4.0]);
        assert!(!deserialized.music_compressor);
    }

    #[test]
    fn test_app_config_minimal_json_fallback_defaults() {
        let minimal_json = r#"{
            "game_dir": "/custom/.minecraft",
            "max_memory_mb": 2048,
            "player_name": "Tester",
            "window_width": 1024,
            "window_height": 768,
            "theme_hue": 180
        }"#;
        let cfg: AppConfig = serde_json::from_str(minimal_json).expect("解析最小核心 JSON 应当使用默认值");
        assert_eq!(cfg.game_dir, "/custom/.minecraft");
        assert_eq!(cfg.max_memory_mb, 2048);
        assert_eq!(cfg.player_name, "Tester");
        assert_eq!(cfg.window_width, 1024);
        assert_eq!(cfg.window_height, 768);
        assert_eq!(cfg.music_volume, 45);
        assert!(cfg.music_compressor);
        assert!(cfg.music_crossfade);
        assert_eq!(cfg.music_eq_preset, "flat");
        assert_eq!(cfg.music_eq_custom, vec![0.0, 0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_user_profile_serde_roundtrip() {
        let profile = UserProfile {
            name: "Steve".to_string(),
            uuid: "00000000-0000-0000-0000-000000000001".to_string(),
            access_token: "secret_token_123".to_string(),
            avatar_url: "https://textures.minecraft.net/texture/abc".to_string(),
            auth_type: "msa".to_string(),
            auth_server: None,
        };
        let json = serde_json::to_string(&profile).unwrap();
        let loaded: UserProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.name, "Steve");
        assert_eq!(loaded.uuid, "00000000-0000-0000-0000-000000000001");
        assert_eq!(loaded.access_token, "secret_token_123");
        assert_eq!(loaded.auth_type, "msa");
        assert_eq!(loaded.auth_server, None);
    }
}
