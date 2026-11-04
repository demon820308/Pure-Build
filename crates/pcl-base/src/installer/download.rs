pub use super::verify::*;

use crate::endpoints;
use crate::installer::DownloadProgress;
use crate::launch::MinecraftLauncher;
use crate::reporter::ReporterHandle;
use sha1::{Digest, Sha1};
use sha2::Sha512;
use std::fs::{self, File};
use std::io::Write;
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
            let mut sha1_hasher = expected_sha1.map(|_| Sha1::new());
            let mut sha512_hasher = expected_sha512.map(|_| Sha512::new());
            let mut magic_header = Vec::with_capacity(4);

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
                                if magic_header.len() < 4 {
                                    let need = 4 - magic_header.len();
                                    magic_header.extend_from_slice(&chunk[..chunk.len().min(need)]);
                                }
                                if let Some(h) = sha1_hasher.as_mut() {
                                    h.update(&chunk);
                                }
                                if let Some(h) = sha512_hasher.as_mut() {
                                    h.update(&chunk);
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
                let mut valid = true;

                if let Some(exp) = check_size {
                    if exp > 0 && downloaded != exp {
                        last_err = format!(
                            "{} 大小不匹配 (期望: {} 字节, 实际: {} 字节)",
                            file_name, exp, downloaded
                        );
                        valid = false;
                    }
                } else if downloaded == 0 {
                    last_err = format!("{} 下载内容为空 (0 字节)", file_name);
                    valid = false;
                }

                if valid {
                    let is_archive = target_path
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|ext| ext.eq_ignore_ascii_case("jar") || ext.eq_ignore_ascii_case("zip"))
                        .unwrap_or(false);
                    if is_archive
                        && (downloaded < 4
                            || (magic_header.as_slice() != b"PK\x03\x04"
                                && magic_header.as_slice() != b"PK\x05\x06"))
                    {
                        last_err = format!("{} 无效的 ZIP/JAR 归档文件（缺少 PK 魔数）", file_name);
                        valid = false;
                    }
                }

                if valid {
                    if let Some(exp_sha1) = expected_sha1 {
                        let actual = sha1_hasher
                            .map(|h| format!("{:x}", h.finalize()))
                            .unwrap_or_default();
                        if !actual.eq_ignore_ascii_case(exp_sha1.trim()) {
                            let exp_short =
                                if exp_sha1.len() >= 8 { &exp_sha1[..8] } else { exp_sha1 };
                            let act_short =
                                if actual.len() >= 8 { &actual[..8] } else { &actual };
                            last_err = format!(
                                "{} SHA1 校验不匹配 (期望: {}..., 实际: {}...)",
                                file_name, exp_short, act_short
                            );
                            valid = false;
                        }
                    }
                }

                if valid {
                    if let Some(exp_sha512) = expected_sha512 {
                        let actual = sha512_hasher
                            .map(|h| format!("{:x}", h.finalize()))
                            .unwrap_or_default();
                        if !actual.eq_ignore_ascii_case(exp_sha512.trim()) {
                            let exp_short =
                                if exp_sha512.len() >= 8 { &exp_sha512[..8] } else { exp_sha512 };
                            let act_short =
                                if actual.len() >= 8 { &actual[..8] } else { &actual };
                            last_err = format!(
                                "{} SHA512 校验不匹配 (期望: {}..., 实际: {}...)",
                                file_name, exp_short, act_short
                            );
                            valid = false;
                        }
                    }
                }

                if valid {
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
