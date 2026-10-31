use crate::endpoints::SUPPORTED_AUDIO_EXTENSIONS;
use crate::platform;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceItem {
    pub file_name: String,
    pub name: String,
    pub size_mb: f32,
    pub is_enabled: bool,
    pub is_directory: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveInfo {
    pub folder_name: String,
    pub size_mb: f32,
    pub path: String,
    pub has_level_dat: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicTrack {
    pub name: String,
    pub file_name: String,
    pub path: String,
    pub folder_path: String,
    pub size_mb: f32,
}

pub struct ResourceScanner;

impl ResourceScanner {
    /// 解析指定版本或全局下的目标资源子目录（如 mods / shaderpacks / resourcepacks / saves）
    pub fn resolve_resource_folder<P: AsRef<Path>>(
        version_dir: P,
        game_dir: P,
        folder_name: &str,
    ) -> PathBuf {
        let v_folder = version_dir.as_ref().join(folder_name);
        let g_folder = game_dir.as_ref().join(folder_name);

        if v_folder.is_dir() {
            v_folder
        } else if g_folder.is_dir() {
            g_folder
        } else {
            v_folder
        }
    }

    /// 扫描资源包 (resourcepacks) 或光影包 (shaderpacks)
    pub fn scan_folder<P: AsRef<Path>>(
        version_dir: P,
        game_dir: P,
        folder_name: &str,
    ) -> Vec<ResourceItem> {
        let target_dir = Self::resolve_resource_folder(version_dir, game_dir, folder_name);
        if !target_dir.is_dir() {
            return Vec::new();
        }

        let mut items = Vec::new();

        if let Ok(entries) = fs::read_dir(&target_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                if file_name.ends_with(".txt") || file_name.starts_with('.') {
                    continue;
                }

                let is_dir = path.is_dir();
                let is_zip = file_name.ends_with(".zip");
                let is_disabled = file_name.ends_with(".disabled");

                if is_dir || is_zip || is_disabled {
                    let size_mb = if is_dir {
                        Self::dir_size_mb(&path)
                    } else {
                        fs::metadata(&path)
                            .map(|m| m.len() as f32 / (1024.0 * 1024.0))
                            .unwrap_or(0.0)
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

    /// 扫描单人世界存档 (saves)
    pub fn scan_saves<P: AsRef<Path>>(version_dir: P, game_dir: P) -> Vec<SaveInfo> {
        let target_dir = Self::resolve_resource_folder(version_dir, game_dir, "saves");
        if !target_dir.is_dir() {
            return Vec::new();
        }

        let mut saves = Vec::new();
        if let Ok(entries) = fs::read_dir(&target_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let folder_name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string();
                    if folder_name.is_empty() || folder_name.starts_with('.') {
                        continue;
                    }
                    let has_level_dat = path.join("level.dat").is_file();
                    let size_mb = Self::dir_size_mb(&path);
                    saves.push(SaveInfo {
                        folder_name,
                        size_mb,
                        path: path.to_string_lossy().to_string(),
                        has_level_dat,
                    });
                }
            }
        }

        saves.sort_by(|a, b| a.folder_name.to_lowercase().cmp(&b.folder_name.to_lowercase()));
        saves
    }

    fn dir_size_mb(dir: &Path) -> f32 {
        let mut total_bytes: u64 = 0;
        for entry in WalkDir::new(dir).max_depth(4).into_iter().flatten() {
            if let Ok(meta) = entry.metadata() {
                if meta.is_file() {
                    total_bytes += meta.len();
                }
            }
        }
        total_bytes as f32 / (1024.0 * 1024.0)
    }

    /// 调用平台层打开本地文件夹或定位文件（§3.4）
    pub fn open_folder<P: AsRef<Path>>(path: P) -> Result<(), String> {
        platform::open_folder(path.as_ref())
    }

    /// 递归扫描默认音乐目录与用户添加的所有自定义音乐文件夹，返回去重后的曲目列表
    pub fn scan_music_folders(custom_dirs: &[String]) -> Vec<MusicTrack> {
        let default_dir = platform::default_music_dir();
        if !default_dir.exists() {
            let _ = fs::create_dir_all(&default_dir);
        }

        let mut scan_roots: Vec<PathBuf> = vec![default_dir];
        for raw_dir in custom_dirs {
            let trimmed = raw_dir.trim();
            if !trimmed.is_empty() {
                let p = PathBuf::from(trimmed);
                if p.is_dir() {
                    scan_roots.push(p);
                }
            }
        }

        let mut seen_paths: HashSet<PathBuf> = HashSet::new();
        let mut tracks: Vec<MusicTrack> = Vec::new();

        for root in scan_roots {
            for entry in WalkDir::new(&root).follow_links(true).into_iter().flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }

                let file_name = match path.file_name().and_then(|n| n.to_str()) {
                    Some(n) if !n.is_empty() && !n.starts_with('.') => n,
                    _ => continue,
                };

                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.to_ascii_lowercase())
                    .unwrap_or_default();

                if !SUPPORTED_AUDIO_EXTENSIONS.contains(&ext.as_str()) {
                    continue;
                }

                let canonical = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
                if !seen_paths.insert(canonical) {
                    continue;
                }

                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(file_name)
                    .to_string();

                let folder_path = path
                    .parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();

                let size_mb = entry
                    .metadata()
                    .map(|m| m.len() as f32 / (1024.0 * 1024.0))
                    .unwrap_or(0.0);

                tracks.push(MusicTrack {
                    name,
                    file_name: file_name.to_string(),
                    path: path.to_string_lossy().to_string(),
                    folder_path,
                    size_mb,
                });
            }
        }

        tracks.sort_by(|a, b| {
            a.name
                .to_lowercase()
                .cmp(&b.name.to_lowercase())
                .then_with(|| a.path.cmp(&b.path))
        });
        tracks
    }

    /// 读取本地音乐文件并编码为 Base64 Data URL，供前端 Audio 引擎解码为 Blob 播放
    pub fn read_music_data_url<P: AsRef<Path>>(file_path: P) -> Result<String, String> {
        let path = file_path.as_ref();
        if !path.is_file() {
            return Err(format!("音乐文件不存在: {}", path.display()));
        }

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .unwrap_or_default();

        if !SUPPORTED_AUDIO_EXTENSIONS.contains(&ext.as_str()) {
            return Err(format!("不支持的音频文件格式: .{}", ext));
        }

        let mime = match ext.as_str() {
            "mp3" => "audio/mpeg",
            "flac" => "audio/flac",
            "wav" => "audio/wav",
            "ogg" | "opus" => "audio/ogg",
            "m4a" => "audio/mp4",
            "aac" => "audio/aac",
            "webm" => "audio/webm",
            _ => "audio/mpeg",
        };

        let bytes = fs::read(path).map_err(|e| format!("读取音频文件失败: {}", e))?;
        let encoded = encode_base64(&bytes);
        Ok(format!("data:{};base64,{}", mime, encoded))
    }
}

/// 标准 RFC 4648 Base64 编码器（零外部依赖，高性能纯安全实现）
fn encode_base64(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::with_capacity(((input.len() + 2) / 3) * 4);

    let mut chunks = input.chunks_exact(3);
    for chunk in &mut chunks {
        let n = ((chunk[0] as u32) << 16) | ((chunk[1] as u32) << 8) | (chunk[2] as u32);
        out.push(TABLE[((n >> 18) & 0x3F) as usize]);
        out.push(TABLE[((n >> 12) & 0x3F) as usize]);
        out.push(TABLE[((n >> 6) & 0x3F) as usize]);
        out.push(TABLE[(n & 0x3F) as usize]);
    }

    let rem = chunks.remainder();
    if rem.len() == 1 {
        let n = (rem[0] as u32) << 16;
        out.push(TABLE[((n >> 18) & 0x3F) as usize]);
        out.push(TABLE[((n >> 12) & 0x3F) as usize]);
        out.push(b'=');
        out.push(b'=');
    } else if rem.len() == 2 {
        let n = ((rem[0] as u32) << 16) | ((rem[1] as u32) << 8);
        out.push(TABLE[((n >> 18) & 0x3F) as usize]);
        out.push(TABLE[((n >> 12) & 0x3F) as usize]);
        out.push(TABLE[((n >> 6) & 0x3F) as usize]);
        out.push(b'=');
    }

    String::from_utf8(out).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn test_base64_encoding_vectors() {
        assert_eq!(encode_base64(b""), "");
        assert_eq!(encode_base64(b"f"), "Zg==");
        assert_eq!(encode_base64(b"fo"), "Zm8=");
        assert_eq!(encode_base64(b"foo"), "Zm9v");
        assert_eq!(encode_base64(b"foob"), "Zm9vYg==");
        assert_eq!(encode_base64(b"fooba"), "Zm9vYmE=");
        assert_eq!(encode_base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn test_scan_music_folders_recursive_and_dedup() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_root = std::env::temp_dir().join(format!("pcl_music_test_{}", nanos));
        let sub_dir = temp_root.join("AlbumA").join("Disc1");
        fs::create_dir_all(&sub_dir).unwrap();

        let track1 = temp_root.join("C418 - Sweden.mp3");
        let track2 = sub_dir.join("Pigstep.flac");
        let ignored = sub_dir.join("cover.jpg");
        fs::write(&track1, b"dummy mp3").unwrap();
        fs::write(&track2, b"dummy flac").unwrap();
        fs::write(&ignored, b"dummy image").unwrap();

        let root_str = temp_root.to_string_lossy().to_string();
        let sub_str = sub_dir.to_string_lossy().to_string();
        let scanned = ResourceScanner::scan_music_folders(&[root_str, sub_str]);

        let names: Vec<String> = scanned
            .into_iter()
            .filter(|t| t.path.contains(&format!("pcl_music_test_{}", nanos)))
            .map(|t| t.name)
            .collect();

        assert_eq!(names, vec!["C418 - Sweden".to_string(), "Pigstep".to_string()]);
        let _ = fs::remove_dir_all(&temp_root);
    }
}
