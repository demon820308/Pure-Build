use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceItem {
    pub file_name: String,
    pub name: String,
    pub size_mb: f32,
    pub is_enabled: bool,
    pub is_directory: bool,
}

pub struct ResourceScanner;

impl ResourceScanner {
    /// 扫描资源包 (resourcepacks) 或光影包 (shaderpacks)
    pub fn scan_folder<P: AsRef<Path>>(
        version_dir: P,
        game_dir: P,
        folder_name: &str,
    ) -> Vec<ResourceItem> {
        let v_folder = version_dir.as_ref().join(folder_name);
        let g_folder = game_dir.as_ref().join(folder_name);

        let target_dir = if v_folder.is_dir() {
            v_folder
        } else if g_folder.is_dir() {
            g_folder
        } else {
            return Vec::new();
        };

        let mut items = Vec::new();

        if let Ok(entries) = fs::read_dir(&target_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                // 忽略非 zip/非文件夹和 txt 等描述文件
                if file_name.ends_with(".txt") {
                    continue;
                }

                let is_dir = path.is_dir();
                let is_zip = file_name.ends_with(".zip");
                let is_disabled = file_name.ends_with(".disabled");

                if is_dir || is_zip || is_disabled {
                    let size_mb = if is_dir {
                        0.0
                    } else {
                        fs::metadata(&path).map(|m| m.len() as f32 / (1024.0 * 1024.0)).unwrap_or(0.0)
                    };

                    let clean_name = file_name
                        .trim_end_matches(".disabled")
                        .trim_end_matches(".zip")
                        .to_string();

                    items.push(ResourceItem {
                        file_name: file_name.to_string(),
                        name: clean_name,
                        size_mb,
                        is_enabled: !is_disabled,
                        is_directory: is_dir,
                    });
                }
            }
        }

        items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        items
    }

    /// 打开指定本地文件夹在 Windows 资源管理器中显示
    pub fn open_folder<P: AsRef<Path>>(path: P) -> Result<(), String> {
        let p = path.as_ref();
        if !p.exists() {
            fs::create_dir_all(p).map_err(|e| format!("创建目录失败: {}", e))?;
        }

        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("explorer")
                .arg(p.to_string_lossy().to_string())
                .spawn()
                .map_err(|e| format!("无法打开资源管理器: {}", e))?;
        }
        Ok(())
    }
}
