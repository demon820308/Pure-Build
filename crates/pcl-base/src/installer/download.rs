use crate::endpoints;
use crate::installer::DownloadProgress;
use crate::launch::MinecraftLauncher;
use crate::reporter::ReporterHandle;
use sha1::{Digest as Sha1Digest, Sha1};
use sha2::Sha512;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

#[derive(Debug, Clone)]
pub struct BatchDownloadTask {
    pub target_path: PathBuf,
    pub urls: Vec<String>,
    pub expected_size: Option<u64>,
    pub expected_sha1: Option<String>,
    pub extract_natives: Option<(PathBuf, Option<Vec<String>>)>,
}

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

/// 获取针对特定本地目标路径的异步互斥锁（对齐 PCL2 `NetManagerClass.AllFiles` 全局正在下载去重机制，防止多次点击或并发任务写同一文件冲突）
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

/// 校验本地文件是否完整有效（对齐 PCL2 `FileChecker.Check` 与整改二规范）：
/// 1. 校验文件存在且大小匹配 `expected_size`（未指定时要求 > 0）；
/// 2. 若扩展名为 `.jar` 或 `.zip`，校验 ZIP 魔数 `PK\x03\x04` 或 `PK\x05\x06`，防止网关 429/502 HTML 报错页或截断文件伪装成有效构件；
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

/// 流式下载单个文件并原子落盘（带 SHA 完整性校验与多源重试）：
/// - 同路径全局互斥防冲突，复用已通过哈希与魔数校验的文件；
/// - 遇到已存在但损坏/篡改文件自动删除并触发重下；
/// - 支持多源轮询与两轮 Retried 兜底重试，遇 429/503 自动冷却等待；
/// - 先写入 `.tmp` 临时文件，校验哈希通过后原子替换目标文件。
pub async fn download_file_with_retry_and_hashes<F>(
    client: &reqwest::Client,
    urls: &[String],
    target_path: &Path,
    expected_size: Option<u64>,
    expected_sha1: Option<&str>,
    expected_sha512: Option<&str>,
    mut on_progress: F,
) -> Result<(), String>
where
    F: FnMut(u64, u64),
{
    let file_lock = acquire_file_lock(target_path);
    let _guard = file_lock.lock().await;

    if is_file_valid_with_hash(target_path, expected_size, expected_sha1, expected_sha512) {
        let sz = expected_size
            .or_else(|| fs::metadata(target_path).ok().map(|m| m.len()))
            .unwrap_or(1);
        on_progress(sz, sz);
        return Ok(());
    } else if target_path.exists() {
        // 已存在但校验未通过（损坏或被篡改），清理坏文件走重下修复
        let _ = fs::remove_file(target_path);
    }

    if let Some(parent) = target_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let file_name = target_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("download");
    let mut last_err = String::from("无可用下载源");

    // 对齐 PCL2 NetFile：两轮完整尝试（第一轮常规多源切换，第二轮 Retried 兜底重试）
    for pass in 0..2usize {
        if pass > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(800)).await;
        }
        for url in urls {
            if url.trim().is_empty() {
                continue;
            }
            endpoints::pace_mirror_request(url).await;

            let resp = match client.get(url).send().await {
                Ok(r) => r,
                Err(e) => {
                    last_err = format!("{} 请求失败: {}", file_name, e);
                    continue;
                }
            };

            let status = resp.status();
            if status.as_u16() == 429
                || status.as_u16() == 503
                || (status.as_u16() == 403 && endpoints::is_mirror_url(url))
            {
                last_err = format!("{} 镜像源限流 (HTTP {})", file_name, status.as_u16());
                tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
                continue;
            }
            if !status.is_success() {
                last_err = format!("{} HTTP {} ({})", file_name, status, url);
                continue;
            }

            let content_len = resp.content_length();
            if let (Some(exp), Some(cl)) = (expected_size, content_len) {
                if exp > 0 && cl > 0 && cl != exp {
                    last_err = format!(
                        "{} 远端 ContentLength ({}) 与预期大小 ({}) 不一致",
                        file_name, cl, exp
                    );
                    continue;
                }
            }

            let total_bytes = expected_size.or(content_len).unwrap_or(0);
            let tmp_path = target_path.with_file_name(format!(
                ".{}.{}.part.tmp",
                file_name,
                std::process::id()
            ));

            let mut stream_ok = true;
            let mut downloaded: u64 = 0;

            match File::create(&tmp_path) {
                Ok(mut tmp_file) => {
                    let mut resp_stream = resp;
                    loop {
                        match resp_stream.chunk().await {
                            Ok(Some(chunk)) => {
                                if let Err(e) = tmp_file.write_all(&chunk) {
                                    last_err = format!("写入临时文件失败: {}", e);
                                    stream_ok = false;
                                    break;
                                }
                                downloaded += chunk.len() as u64;
                                on_progress(downloaded, total_bytes.max(downloaded));
                            }
                            Ok(None) => break,
                            Err(e) => {
                                last_err = format!("{} 读取数据流中断: {}", file_name, e);
                                stream_ok = false;
                                break;
                            }
                        }
                    }
                    let _ = tmp_file.flush();
                }
                Err(e) => {
                    last_err = format!("创建临时文件失败: {}", e);
                    stream_ok = false;
                }
            }

            if stream_ok {
                let check_size = expected_size.or(content_len.filter(|&c| c > 0));
                if is_file_valid_with_hash(&tmp_path, check_size, expected_sha1, expected_sha512) {
                    if target_path.exists() {
                        let _ = fs::remove_file(target_path);
                    }
                    if let Err(e) = fs::rename(&tmp_path, target_path) {
                        // 跨卷或重命名受阻时回退 copy + remove
                        if fs::copy(&tmp_path, target_path).is_ok() {
                            let _ = fs::remove_file(&tmp_path);
                            return Ok(());
                        }
                        last_err = format!("替换目标文件失败: {}", e);
                    } else {
                        return Ok(());
                    }
                } else {
                    let actual_hash = if expected_sha1.is_some() {
                        compute_file_sha1(&tmp_path).unwrap_or_else(|_| "read_err".to_string())
                    } else if expected_sha512.is_some() {
                        compute_file_sha512(&tmp_path).unwrap_or_else(|_| "read_err".to_string())
                    } else {
                        "none".to_string()
                    };
                    let exp_short = expected_sha1
                        .or(expected_sha512)
                        .map(|h| if h.len() >= 8 { &h[0..8] } else { h })
                        .unwrap_or("none");
                    let act_short = if actual_hash.len() >= 8 {
                        &actual_hash[0..8]
                    } else {
                        &actual_hash
                    };
                    last_err = format!(
                        "{} SHA 校验不匹配 (期望: {}..., 实际: {}...)",
                        file_name, exp_short, act_short
                    );
                }
            }

            let _ = fs::remove_file(&tmp_path);
        }
    }

    Err(last_err)
}

#[inline]
pub async fn download_file_with_retry<F>(
    client: &reqwest::Client,
    urls: &[String],
    target_path: &Path,
    expected_size: Option<u64>,
    expected_sha1: Option<&str>,
    on_progress: F,
) -> Result<(), String>
where
    F: FnMut(u64, u64),
{
    download_file_with_retry_and_hashes(
        client,
        urls,
        target_path,
        expected_size,
        expected_sha1,
        None,
        on_progress,
    )
    .await
}

/// 发起单次二进制文件拉取（带 BMCLAPI 节流与 Mojang 备选回退），返回字节数组
pub async fn fetch_bytes_with_fallback(
    client: &reqwest::Client,
    primary_url: &str,
    backup_url: Option<&str>,
) -> Result<Vec<u8>, String> {
    let mut urls = vec![primary_url.to_string()];
    if let Some(b) = backup_url {
        if !b.is_empty() && b != primary_url {
            urls.push(b.to_string());
        }
    }
    let mut last_err = String::from("下载失败");
    for attempt in 0..3usize {
        if attempt > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
        for u in &urls {
            endpoints::pace_mirror_request(u).await;
            match client.get(u).send().await {
                Ok(resp) => {
                    let status = resp.status();
                    if status.as_u16() == 429
                        || status.as_u16() == 503
                        || (status.as_u16() == 403 && endpoints::is_mirror_url(u))
                    {
                        last_err = format!("镜像源限流 (HTTP {})", status.as_u16());
                        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                        continue;
                    }
                    if !status.is_success() {
                        last_err = format!("HTTP {}", status);
                        continue;
                    }
                    match resp.bytes().await {
                        Ok(b) => return Ok(b.to_vec()),
                        Err(e) => last_err = format!("读取响应流失败: {}", e),
                    }
                }
                Err(e) => last_err = format!("请求失败: {}", e),
            }
        }
    }
    Err(last_err)
}

/// 发起单次文本内容拉取（带 BMCLAPI 节流与 Mojang 备选回退），返回字符串
pub async fn fetch_text_with_fallback(
    client: &reqwest::Client,
    primary_url: &str,
    backup_url: &str,
) -> Result<String, String> {
    let urls = [primary_url, backup_url];
    let mut last_err = String::from("下载文本失败");
    for attempt in 0..3usize {
        if attempt > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
        for u in urls {
            if u.trim().is_empty() {
                continue;
            }
            endpoints::pace_mirror_request(u).await;
            match client.get(u).send().await {
                Ok(resp) => {
                    let status = resp.status();
                    if status.as_u16() == 429
                        || status.as_u16() == 503
                        || (status.as_u16() == 403 && endpoints::is_mirror_url(u))
                    {
                        last_err = format!("镜像源限流 (HTTP {})", status.as_u16());
                        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                        continue;
                    }
                    if !status.is_success() {
                        last_err = format!("HTTP {}", status);
                        continue;
                    }
                    match resp.text().await {
                        Ok(t) => return Ok(t),
                        Err(e) => last_err = format!("读取响应文本失败: {}", e),
                    }
                }
                Err(e) => last_err = format!("请求失败: {}", e),
            }
        }
    }
    Err(last_err)
}

/// 通用高并发批量下载池（使用 Semaphore 控制并发 + BMCLAPI 节流 + 临时文件原子校验 + 失败汇总报告）
pub async fn execute_batch_downloads(
    reporter: &ReporterHandle,
    version_id: &str,
    vanilla_version: Option<&str>,
    stage_label: &str,
    start_pct: f32,
    end_pct: f32,
    concurrency: usize,
    client: &reqwest::Client,
    tasks: Vec<BatchDownloadTask>,
) -> Result<(), String> {
    let total = tasks.len();
    if total == 0 {
        return Ok(());
    }

    let van_ver_owned = vanilla_version.map(|s| s.to_string());
    reporter.emit_download(DownloadProgress {
        version_id: version_id.to_string(),
        vanilla_version: van_ver_owned.clone(),
        step: format!("{} (0/{})", stage_label, total),
        percentage: start_pct,
        speed_text: format!("0/{}", total),
        is_finished: false,
        error: None,
    });

    let semaphore = Arc::new(Semaphore::new(concurrency.max(1)));
    let completed = Arc::new(AtomicUsize::new(0));
    let failures: Arc<std::sync::Mutex<Vec<String>>> =
        Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut join_set = JoinSet::new();
    let step_interval = (total / 40).max(1);

    for task in tasks {
        let sem = semaphore.clone();
        let cli = client.clone();
        let comp = completed.clone();
        let fail_list = failures.clone();
        let reporter_clone = reporter.clone();
        let ver_id = version_id.to_string();
        let van_ver = van_ver_owned.clone();
        let label = stage_label.to_string();

        join_set.spawn(async move {
            let _permit = sem.acquire_owned().await.ok();

            if let Err(err_msg) = download_file_with_retry(
                &cli,
                &task.urls,
                &task.target_path,
                task.expected_size,
                task.expected_sha1.as_deref(),
                |_, _| {},
            )
            .await
            {
                fail_list.lock().unwrap().push(err_msg);
                return;
            }

            // 若声明了需要提取的 Native 动态库，则安全解压到 natives 目录（剔除 META-INF 与签名）
            if let Some((natives_dir, exclude_rules)) = task.extract_natives {
                if let Err(e) = MinecraftLauncher::extract_native_jar(
                    &task.target_path,
                    &natives_dir,
                    exclude_rules.as_deref(),
                ) {
                    fail_list
                        .lock()
                        .unwrap()
                        .push(format!("解压 Natives 失败: {}", e));
                }
            }

            let done = comp.fetch_add(1, Ordering::Relaxed) + 1;
            if done % step_interval == 0 || done == total {
                let pct = start_pct + (done as f32 / total as f32) * (end_pct - start_pct);
                reporter_clone.emit_download(DownloadProgress {
                    version_id: ver_id,
                    vanilla_version: van_ver,
                    step: format!("{} ({}/{})", label, done, total),
                    percentage: pct,
                    speed_text: format!("{}/{}", done, total),
                    is_finished: false,
                    error: None,
                });
            }
        });
    }

    while let Some(res) = join_set.join_next().await {
        if let Err(e) = res {
            failures
                .lock()
                .unwrap()
                .push(format!("并发下载任务异常崩溃: {}", e));
        }
    }

    let errs = failures.lock().unwrap().clone();
    if !errs.is_empty() {
        return Err(format!(
            "批量下载完成，但存在 {} 个构件失败，首个错误: {}",
            errs.len(),
            errs.first().cloned().unwrap_or_default()
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
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
        // 篡改压缩内容中的几个字节（避开头部 PK 魔数）
        let target_idx = tampered_bytes.len() - 5;
        tampered_bytes[target_idx] ^= 0xFF;
        fs::write(&tampered_jar, &tampered_bytes).unwrap();

        assert_eq!(tampered_bytes.len() as u64, size);
        // 大小和 ZIP 魔数依然完全一致，但 SHA-1 已改变 -> 必须拒绝
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

        // 即使字节数碰巧一致，非 PK\x03\x04 开头的 .jar 也必须被拒绝
        assert!(!is_file_valid(
            &fake_jar,
            Some(html_err.len() as u64),
            None
        ));

        // 合法 ZIP/JAR 魔数头且尺寸吻合时通过
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

        // 1. 空文件 (0 字节) 必须被拒绝
        let empty_file = temp_dir.join("empty.jar");
        fs::write(&empty_file, b"").unwrap();
        assert!(!is_file_valid(&empty_file, None, None));
        assert!(!is_file_valid(&empty_file, Some(0), None));

        // 2. 截断文件 (小于 4 字节的 jar) 必须被拒绝
        let tiny_jar = temp_dir.join("tiny.jar");
        fs::write(&tiny_jar, b"PK\x03").unwrap();
        assert!(!is_file_valid(&tiny_jar, None, None));

        // 3. 502 Bad Gateway HTML 页面伪装的 jar 必须被拒绝
        let bad_gateway_jar = temp_dir.join("gateway.jar");
        let html_502 = b"<!DOCTYPE html><html><head><title>502 Bad Gateway</title></head></html>";
        fs::write(&bad_gateway_jar, html_502).unwrap();
        assert!(!is_file_valid(&bad_gateway_jar, Some(html_502.len() as u64), None));

        // 4. 普通非 archive 文本文件无魔数要求，尺寸吻合且大于 0 即可
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
