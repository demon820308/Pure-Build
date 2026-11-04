use sha1::{Digest, Sha1};
use sha2::Sha512;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// 计算指定本地文件的 SHA-1 十六进制哈希串
pub fn compute_file_sha1(path: &Path) -> Result<String, std::io::Error> {
    let mut file = File::open(path)?;
    let mut hasher = Sha1::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    let res = hasher.finalize();
    Ok(format!("{:x}", res))
}

/// 计算指定本地文件的 SHA-512 十六进制哈希串
pub fn compute_file_sha512(path: &Path) -> Result<String, std::io::Error> {
    let mut file = File::open(path)?;
    let mut hasher = Sha512::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    let res = hasher.finalize();
    Ok(format!("{:x}", res))
}

/// 获取针对特定本地目标路径的异步互斥锁（全局下载去重，防止多次点击或并发任务写同一文件冲突）
pub fn acquire_file_lock(path: &Path) -> Arc<tokio::sync::Mutex<()>> {
    static FILE_LOCKS: std::sync::OnceLock<
        std::sync::Mutex<HashMap<PathBuf, Arc<tokio::sync::Mutex<()>>>>,
    > = std::sync::OnceLock::new();
    let map_mutex = FILE_LOCKS.get_or_init(|| std::sync::Mutex::new(HashMap::new()));
    let mut map = map_mutex.lock().unwrap();
    map.entry(path.to_path_buf())
        .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
        .clone()
}

/// 校验本地文件是否完整有效：
/// 1. 校验文件存在且大小匹配 `expected_size`（未指定时要求 > 0）；
/// 2. 若扩展名为 `.jar` 或 `.zip`，校验 ZIP 魔数 `PK\x03\x04` 或 `PK\x05\x06`，防止网关 429/502 HTML 报错页伪装成有效构件；
/// 3. 若提供期望哈希（SHA-1 / SHA-512），计算文件哈希比对，任何不符均拒绝。
pub fn is_file_valid_with_hash(
    path: &Path,
    expected_size: Option<u64>,
    expected_sha1: Option<&str>,
    expected_sha512: Option<&str>,
) -> bool {
    let Ok(meta) = fs::metadata(path) else {
        return false;
    };
    if !meta.is_file() {
        return false;
    }
    let len = meta.len();
    if let Some(exp) = expected_size {
        if exp > 0 && len != exp {
            return false;
        }
    } else if len == 0 {
        return false;
    }

    let is_archive = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("jar") || ext.eq_ignore_ascii_case("zip"))
        .unwrap_or(false);

    if is_archive {
        if len < 4 {
            return false;
        }
        let Ok(mut f) = File::open(path) else {
            return false;
        };
        let mut magic = [0u8; 4];
        if f.read_exact(&mut magic).is_err() {
            return false;
        }
        if &magic != b"PK\x03\x04" && &magic != b"PK\x05\x06" {
            return false;
        }
    }

    if let Some(exp_sha1) = expected_sha1 {
        let trimmed = exp_sha1.trim();
        if !trimmed.is_empty() {
            match compute_file_sha1(path) {
                Ok(actual) => {
                    if !actual.eq_ignore_ascii_case(trimmed) {
                        return false;
                    }
                }
                Err(_) => return false,
            }
        }
    }

    if let Some(exp_sha512) = expected_sha512 {
        let trimmed = exp_sha512.trim();
        if !trimmed.is_empty() {
            match compute_file_sha512(path) {
                Ok(actual) => {
                    if !actual.eq_ignore_ascii_case(trimmed) {
                        return false;
                    }
                }
                Err(_) => return false,
            }
        }
    }

    true
}

#[inline]
pub fn is_file_valid(
    path: &Path,
    expected_size: Option<u64>,
    expected_sha1: Option<&str>,
) -> bool {
    is_file_valid_with_hash(path, expected_size, expected_sha1, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self, Write};
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    #[test]
    fn test_is_file_valid_tampered_same_size_fails_and_correct_hash_passes() {
        let temp_dir = std::env::temp_dir().join(format!("pcl_sha1_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);

        let valid_jar = temp_dir.join("valid.jar");
        let mut zip_bytes = Vec::new();
        {
            let mut zw = ZipWriter::new(io::Cursor::new(&mut zip_bytes));
            zw.start_file("hello.txt", SimpleFileOptions::default()).unwrap();
            zw.write_all(b"Original File Content For SHA1 Check").unwrap();
            zw.finish().unwrap();
        }
        fs::write(&valid_jar, &zip_bytes).unwrap();

        let real_sha1 = compute_file_sha1(&valid_jar).unwrap();
        let size = zip_bytes.len() as u64;

        // 1. 真实哈希与大小一致 -> 通过
        assert!(is_file_valid(&valid_jar, Some(size), Some(&real_sha1)));

        // 2. 篡改文件内容（保持大小精确相等）
        let tampered_jar = temp_dir.join("tampered.jar");
        let mut tampered_bytes = zip_bytes.clone();
        let target_idx = tampered_bytes.len() - 5;
        tampered_bytes[target_idx] ^= 0xFF;
        fs::write(&tampered_jar, &tampered_bytes).unwrap();

        assert_eq!(tampered_bytes.len() as u64, size);
        assert!(!is_file_valid(&tampered_jar, Some(size), Some(&real_sha1)));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_is_file_valid_rejects_html_jar_and_size_mismatch() {
        let temp_dir = std::env::temp_dir().join(format!("pcl_valid_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);

        let fake_jar = temp_dir.join("corrupted.jar");
        let html_err = b"<html><body>429 Too Many Requests</body></html>";
        fs::write(&fake_jar, html_err).unwrap();

        assert!(!is_file_valid(
            &fake_jar,
            Some(html_err.len() as u64),
            None
        ));

        let valid_jar = temp_dir.join("valid.jar");
        let mut zip_bytes = Vec::new();
        {
            let mut zw = ZipWriter::new(io::Cursor::new(&mut zip_bytes));
            zw.start_file("META-INF/MANIFEST.MF", SimpleFileOptions::default())
                .unwrap();
            zw.write_all(b"Manifest-Version: 1.0\n").unwrap();
            zw.finish().unwrap();
        }
        fs::write(&valid_jar, &zip_bytes).unwrap();
        assert!(is_file_valid(
            &valid_jar,
            Some(zip_bytes.len() as u64),
            None
        ));
        assert!(!is_file_valid(
            &valid_jar,
            Some(zip_bytes.len() as u64 + 10),
            None
        ));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_is_file_valid_zero_size_and_html_error_pages() {
        let temp_dir = std::env::temp_dir().join(format!("pcl_edge_valid_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);

        let empty_file = temp_dir.join("empty.jar");
        fs::write(&empty_file, b"").unwrap();
        assert!(!is_file_valid(&empty_file, None, None));
        assert!(!is_file_valid(&empty_file, Some(0), None));

        let tiny_jar = temp_dir.join("tiny.jar");
        fs::write(&tiny_jar, b"PK\x03").unwrap();
        assert!(!is_file_valid(&tiny_jar, None, None));

        let bad_gateway_jar = temp_dir.join("gateway.jar");
        let html_502 = b"<!DOCTYPE html><html><head><title>502 Bad Gateway</title></head></html>";
        fs::write(&bad_gateway_jar, html_502).unwrap();
        assert!(!is_file_valid(&bad_gateway_jar, Some(html_502.len() as u64), None));

        let text_file = temp_dir.join("custom.txt");
        fs::write(&text_file, b"sample text").unwrap();
        assert!(is_file_valid(&text_file, Some(11), None));
        assert!(!is_file_valid(&text_file, Some(12), None));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_acquire_file_lock_dedup() {
        let path1 = Path::new("/mock/path/to/game.jar");
        let path2 = Path::new("/mock/path/to/game.jar");
        let lock1 = acquire_file_lock(path1);
        let lock2 = acquire_file_lock(path2);
        assert!(Arc::ptr_eq(&lock1, &lock2));

        let path3 = Path::new("/mock/path/to/other.jar");
        let lock3 = acquire_file_lock(path3);
        assert!(!Arc::ptr_eq(&lock1, &lock3));
    }
}
