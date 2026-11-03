use crate::config::ConfigManager;
use crate::platform;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
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
}
