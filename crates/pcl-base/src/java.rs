use crate::config::ConfigManager;
use crate::platform;
use crate::reporter::ReporterHandle;
use flate2::read::GzDecoder;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use tar::Archive;
use tokio::io::AsyncWriteExt;
use ts_rs::TS;
use zip::ZipArchive;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct JavaInfo {
    pub path: String,
    pub version: String,
    pub major_version: u32,
    pub is_64_bit: bool,
    pub is_jdk: bool,
    pub arch: String,
}

pub struct JavaScanner;

impl JavaScanner {
    /// 跨平台扫描系统中的所有可用 Java（候选搜索目录统一由 platform.rs 集中提供）
    pub fn scan_all() -> Vec<JavaInfo> {
        let mut found_paths = HashSet::new();
        let exec_name = platform::java_executable_name();

        // 1. 从环境变量 PATH 查找
        if let Some(path_env) = env::var_os("PATH") {
            for entry in env::split_paths(&path_env) {
                let candidate = entry.join(exec_name);
                if candidate.is_file() {
                    found_paths.insert(candidate);
                }
            }
        }

        // 2. 从 JAVA_HOME 查找
        if let Ok(java_home) = env::var("JAVA_HOME") {
            let candidate = PathBuf::from(java_home).join("bin").join(exec_name);
            if candidate.is_file() {
                found_paths.insert(candidate);
            }
        }

        // 3. 遍历 platform 层提供的本平台候选目录 + 当前配置游戏目录下的 runtime
        let mut search_dirs = platform::java_search_dirs();
        let configured_game_dir = PathBuf::from(ConfigManager::load().game_dir);
        let custom_runtime = configured_game_dir.join("runtime");
        if !search_dirs.contains(&custom_runtime) {
            search_dirs.push(custom_runtime);
        }

        for dir in search_dirs {
            let direct_bin = dir.join("bin").join(exec_name);
            if direct_bin.is_file() {
                found_paths.insert(direct_bin);
            }
            if dir.is_dir() {
                Self::scan_directory_for_java(&dir, exec_name, &mut found_paths, 5);
            }
        }

        // 4. 解析每一个 Java 路径的具体版本与架构
        let mut java_list = Vec::new();
        let mut seen_canonical = HashSet::new();
        for path in found_paths {
            let canonical_key = std::fs::canonicalize(&path)
                .unwrap_or_else(|_| path.clone())
                .to_string_lossy()
                .to_string();
            if !seen_canonical.insert(canonical_key) {
                continue;
            }
            if let Some(info) = Self::parse_java(&path) {
                java_list.push(info);
            }
        }

        // 按版本号降序排序
        java_list.sort_by(|a, b| b.major_version.cmp(&a.major_version));
        java_list
    }

    fn scan_directory_for_java(
        dir: &Path,
        exec_name: &str,
        found: &mut HashSet<PathBuf>,
        max_depth: usize,
    ) {
        if max_depth == 0 {
            return;
        }

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if name.eq_ignore_ascii_case(exec_name) {
                        found.insert(path);
                    }
                } else if path.is_dir() {
                    Self::scan_directory_for_java(&path, exec_name, found, max_depth - 1);
                }
            }
        }
    }

    /// 根据 `java -version` 输出文本识别架构与 64 位标识（纯函数，支持单元测试 Apple Silicon arm64 与 x64）
    pub fn detect_arch_and_bitness(version_output: &str) -> (&'static str, bool) {
        let lower = version_output.to_lowercase();
        if lower.contains("aarch64") || lower.contains("arm64") {
            return ("arm64", true);
        }
        if lower.contains("x86_64") || lower.contains("amd64") {
            return ("x64", true);
        }
        if lower.contains("64-bit") {
            return (platform::mc_os_arch(), true);
        }
        ("x86", false)
    }

    /// 执行 java -version 并提取版本与架构信息
    pub fn parse_java(path: &Path) -> Option<JavaInfo> {
        let output = Command::new(path).arg("-version").output().ok()?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let text = format!("{}\n{}", stderr, stdout);

        let (arch, is_64_bit) = Self::detect_arch_and_bitness(&text);
        let is_jdk = path
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("include").exists() || p.join("jmods").exists())
            .unwrap_or(false);

        let re = Regex::new(r#""(\d+)(\.(\d+))?(\.[\d_\w]+)?"#).ok()?;
        let (full_version, major_version) = if let Some(caps) = re.captures(&text) {
            let full = caps.get(0)?.as_str().replace('"', "");
            let first = caps.get(1)?.as_str().parse::<u32>().unwrap_or(0);
            let major = if first == 1 {
                caps.get(3)
                    .and_then(|m| m.as_str().parse::<u32>().ok())
                    .unwrap_or(8)
            } else {
                first
            };
            (full, major)
        } else {
            ("未知版本".to_string(), 0)
        };

        Some(JavaInfo {
            path: path.to_string_lossy().to_string(),
            version: full_version,
            major_version,
            is_64_bit,
            is_jdk,
            arch: arch.to_string(),
        })
    }

    /// 根据所需 Java 主版本在已安装列表中选取最佳匹配
    pub fn select_best_matching_java<'a>(
        installed: &'a [JavaInfo],
        required_major: u32,
    ) -> Option<&'a JavaInfo> {
        let target_arch = platform::mc_os_arch();
        installed
            .iter()
            .filter(|j| j.major_version == required_major)
            .max_by_key(|j| {
                let mut score = 0i32;
                if j.is_64_bit {
                    score += 10;
                }
                if j.arch == target_arch {
                    score += 5;
                }
                if j.is_jdk {
                    score += 1;
                }
                score
            })
    }
}

pub struct JavaDownloader;

impl JavaDownloader {
    /// 在指定目录下递归搜索 Java 可执行文件
    pub fn find_java_in_dir(dir: &Path) -> Option<PathBuf> {
        if !dir.is_dir() {
            return None;
        }
        let exec_name = platform::java_executable_name();
        let mut found = None;
        Self::find_file_recursive(dir, exec_name, &mut found, 6);
        found
    }

    fn find_file_recursive(
        dir: &Path,
        target_name: &str,
        found: &mut Option<PathBuf>,
        max_depth: usize,
    ) {
        if max_depth == 0 || found.is_some() {
            return;
        }
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name.eq_ignore_ascii_case(target_name) {
                            *found = Some(path);
                            return;
                        }
                    }
                } else if path.is_dir() {
                    Self::find_file_recursive(&path, target_name, found, max_depth - 1);
                }
            }
        }
    }

    /// 从 Adoptium API 下载对应平台的 Temurin JRE 并解压至 <game_dir>/runtime/java-<major>
    pub async fn download_and_extract_temurin(
        reporter: &ReporterHandle,
        game_dir: &Path,
        major_version: u32,
    ) -> Result<PathBuf, String> {
        let os = platform::adoptium_os_name();
        let arch = platform::adoptium_arch_name();
        let query_url = crate::endpoints::adoptium_assets_url(major_version, os, arch);

        reporter.emit_launch(crate::launch::LaunchProgress {
            step: format!("正在查询 Adoptium Java {} 资产元数据...", major_version),
            percentage: 20.0,
            pid: None,
            is_finished: false,
            error: None,
        });

        let client = crate::endpoints::shared_http_client();
        let resp = client
            .get(&query_url)
            .send()
            .await
            .map_err(|e| format!("查询 Adoptium API 失败: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!(
                "Adoptium API 返回错误状态码: {} (URL: {})",
                resp.status(),
                query_url
            ));
        }

        let assets: Vec<serde_json::Value> = resp
            .json()
            .await
            .map_err(|e| format!("解析 Adoptium 资产元数据失败: {}", e))?;

        let first = assets
            .first()
            .ok_or_else(|| format!("Adoptium 未找到 Java {} 在此平台下的可用构建", major_version))?;

        let pkg = first
            .get("binary")
            .and_then(|b| b.get("package"))
            .ok_or_else(|| "Adoptium 响应中缺少 binary.package 元数据".to_string())?;

        let download_link = pkg
            .get("link")
            .and_then(|l| l.as_str())
            .ok_or_else(|| "Adoptium 资产缺少下载链接".to_string())?;

        let pkg_name = pkg
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("temurin_jre.archive");

        let expected_checksum = pkg.get("checksum").and_then(|c| c.as_str());
        let total_size = pkg.get("size").and_then(|s| s.as_u64());

        reporter.emit_launch(crate::launch::LaunchProgress {
            step: format!("正在连接下载源获取 Java {}...", major_version),
            percentage: 25.0,
            pid: None,
            is_finished: false,
            error: None,
        });

        let mut stream_resp = client
            .get(download_link)
            .send()
            .await
            .map_err(|e| format!("请求下载 Java 归档失败: {}", e))?;

        if !stream_resp.status().is_success() {
            return Err(format!("下载 Java 归档失败: HTTP {}", stream_resp.status()));
        }

        let runtime_root = game_dir.join("runtime");
        let dest_dir = runtime_root.join(format!("java-{}", major_version));
        let temp_archive_path = runtime_root.join(format!("temp_java_{}.tmp", major_version));

        if let Some(p) = temp_archive_path.parent() {
            let _ = std::fs::create_dir_all(p);
        }

        let mut temp_file = tokio::fs::File::create(&temp_archive_path)
            .await
            .map_err(|e| format!("创建临时文件失败: {}", e))?;

        let mut hasher = Sha256::new();
        let mut downloaded_bytes = 0u64;

        while let Some(chunk) = stream_resp
            .chunk()
            .await
            .map_err(|e| format!("读取 Java 数据流异常: {}", e))?
        {
            hasher.update(&chunk);
            temp_file
                .write_all(&chunk)
                .await
                .map_err(|e| format!("写入临时文件失败: {}", e))?;

            downloaded_bytes += chunk.len() as u64;
            if let Some(total) = total_size {
                if total > 0 {
                    let pct = 25.0 + ((downloaded_bytes as f32 / total as f32) * 45.0).min(45.0);
                    let mb_down = downloaded_bytes as f64 / 1_048_576.0;
                    let mb_total = total as f64 / 1_048_576.0;
                    reporter.emit_launch(crate::launch::LaunchProgress {
                        step: format!(
                            "正在下载 Java {} 运行环境 ({:.1}MB / {:.1}MB)...",
                            major_version, mb_down, mb_total
                        ),
                        percentage: pct,
                        pid: None,
                        is_finished: false,
                        error: None,
                    });
                }
            }
        }
        let _ = temp_file.flush().await;
        drop(temp_file);

        let actual_hash = format!("{:x}", hasher.finalize());
        if let Some(expected) = expected_checksum {
            if !actual_hash.eq_ignore_ascii_case(expected) {
                let _ = std::fs::remove_file(&temp_archive_path);
                return Err(format!(
                    "Java 下载完整性校验失败: 期望 SHA256 {}, 实际 {}",
                    expected, actual_hash
                ));
            }
        }

        reporter.emit_launch(crate::launch::LaunchProgress {
            step: format!("下载完成，正在安全解压 Java {} 运行环境...", major_version),
            percentage: 75.0,
            pid: None,
            is_finished: false,
            error: None,
        });

        if dest_dir.exists() {
            let _ = std::fs::remove_dir_all(&dest_dir);
        }
        std::fs::create_dir_all(&dest_dir)
            .map_err(|e| format!("创建目标解压目录失败: {}", e))?;

        let extract_res = Self::extract_archive(&temp_archive_path, &dest_dir, pkg_name);
        let _ = std::fs::remove_file(&temp_archive_path);
        extract_res?;

        let exe_path = Self::find_java_in_dir(&dest_dir)
            .ok_or_else(|| format!("在解压后的目录中未找到 Java 可执行文件: {:?}", dest_dir))?;

        let _ = platform::make_executable(&exe_path);

        reporter.emit_launch(crate::launch::LaunchProgress {
            step: format!("Java {} 环境就绪: {:?}", major_version, exe_path),
            percentage: 80.0,
            pid: None,
            is_finished: false,
            error: None,
        });

        Ok(exe_path)
    }

    fn extract_archive(archive_path: &Path, dest_dir: &Path, pkg_name: &str) -> Result<(), String> {
        let is_zip = pkg_name.ends_with(".zip");
        if is_zip {
            let file = std::fs::File::open(archive_path).map_err(|e| e.to_string())?;
            let mut zip = ZipArchive::new(file).map_err(|e| format!("打开 zip 归档失败: {}", e))?;
            for i in 0..zip.len() {
                let mut file = zip.by_index(i).map_err(|e| e.to_string())?;
                let raw_name = file.name().to_string();
                let Some(target) =
                    crate::modpack::ModpackManager::sanitize_relative_path(dest_dir, &raw_name)
                else {
                    continue;
                };
                if file.is_dir() {
                    let _ = std::fs::create_dir_all(&target);
                } else {
                    if let Some(parent) = target.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    let mut out = std::fs::File::create(&target).map_err(|e| e.to_string())?;
                    std::io::copy(&mut file, &mut out).map_err(|e| e.to_string())?;
                    let _ = platform::make_executable(&target);
                }
            }
        } else {
            let file = std::fs::File::open(archive_path).map_err(|e| e.to_string())?;
            let gz = GzDecoder::new(file);
            let mut tar = Archive::new(gz);
            let entries = tar.entries().map_err(|e| format!("读取 tar 条目失败: {}", e))?;
            for entry_res in entries {
                let mut entry = entry_res.map_err(|e| format!("解析 tar 记录失败: {}", e))?;
                let path = entry.path().map_err(|e| e.to_string())?;
                let path_str = path.to_string_lossy().to_string();
                let Some(target) =
                    crate::modpack::ModpackManager::sanitize_relative_path(dest_dir, &path_str)
                else {
                    continue;
                };
                if entry.header().entry_type().is_dir() {
                    let _ = std::fs::create_dir_all(&target);
                } else {
                    if let Some(parent) = target.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    entry
                        .unpack(&target)
                        .map_err(|e| format!("解压文件失败 {:?}: {}", target, e))?;
                    let _ = platform::make_executable(&target);
                }
            }
        }
        Ok(())
    }

    /// 校验当前候选路径或自动下载/选用匹配的 Java 主版本
    pub async fn ensure_java_for_version(
        reporter: &ReporterHandle,
        game_dir: &Path,
        candidate_path: &str,
        required_major: u32,
    ) -> Result<PathBuf, String> {
        if !candidate_path.trim().is_empty() {
            let candidate_p = Path::new(candidate_path);
            if candidate_p.is_file() {
                if let Some(info) = JavaScanner::parse_java(candidate_p) {
                    if info.major_version == required_major {
                        return Ok(candidate_p.to_path_buf());
                    }
                }
            }
        }

        let scanned = JavaScanner::scan_all();
        if let Some(best) = JavaScanner::select_best_matching_java(&scanned, required_major) {
            return Ok(PathBuf::from(&best.path));
        }

        let runtime_java_dir = game_dir.join("runtime").join(format!("java-{}", required_major));
        if let Some(local_exe) = Self::find_java_in_dir(&runtime_java_dir) {
            if let Some(info) = JavaScanner::parse_java(&local_exe) {
                if info.major_version == required_major {
                    return Ok(local_exe);
                }
            }
        }

        Self::download_and_extract_temurin(reporter, game_dir, required_major).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_apple_silicon_arm64_and_x64_java() {
        let apple_silicon_out = r#"openjdk version "21.0.2" 2024-01-16 LTS
OpenJDK Runtime Environment Temurin-21.0.2+13 (build 21.0.2+13-LTS)
OpenJDK 64-Bit Server VM Temurin-21.0.2+13 (build 21.0.2+13-LTS, mixed mode, aarch64)"#;
        assert_eq!(
            JavaScanner::detect_arch_and_bitness(apple_silicon_out),
            ("arm64", true)
        );

        let x64_out = r#"openjdk version "17.0.9" 2023-10-17
OpenJDK Runtime Environment (build 17.0.9+9-amd64)"#;
        assert_eq!(JavaScanner::detect_arch_and_bitness(x64_out), ("x64", true));
    }

    #[test]
    fn test_select_best_matching_java() {
        let list = vec![
            JavaInfo {
                path: "/path/to/java8_32".to_string(),
                version: "1.8.0_351".to_string(),
                major_version: 8,
                is_64_bit: false,
                is_jdk: false,
                arch: "x86".to_string(),
            },
            JavaInfo {
                path: "/path/to/java8_64".to_string(),
                version: "1.8.0_351".to_string(),
                major_version: 8,
                is_64_bit: true,
                is_jdk: true,
                arch: "x64".to_string(),
            },
            JavaInfo {
                path: "/path/to/java17_64".to_string(),
                version: "17.0.9".to_string(),
                major_version: 17,
                is_64_bit: true,
                is_jdk: true,
                arch: "x64".to_string(),
            },
        ];

        // 匹配 Java 8，优先选择 64 位版本
        let best8 = JavaScanner::select_best_matching_java(&list, 8);
        assert!(best8.is_some());
        assert_eq!(best8.unwrap().path, "/path/to/java8_64");

        // 匹配 Java 17
        let best17 = JavaScanner::select_best_matching_java(&list, 17);
        assert!(best17.is_some());
        assert_eq!(best17.unwrap().path, "/path/to/java17_64");

        // 匹配未安装的 Java 21，应返回 None
        let best21 = JavaScanner::select_best_matching_java(&list, 21);
        assert!(best21.is_none());
    }
}
