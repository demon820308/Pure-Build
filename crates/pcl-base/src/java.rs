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
}

pub struct JavaScanner;

impl JavaScanner {
    /// 跨平台扫描系统中的所有可用 Java
    pub fn scan_all() -> Vec<JavaInfo> {
        let mut found_paths = HashSet::new();

        // 1. 从环境变量 PATH 查找
        if let Some(path_env) = env::var_os("PATH") {
            for entry in env::split_paths(&path_env) {
                let candidate = entry.join(Self::executable_name());
                if candidate.is_file() {
                    found_paths.insert(candidate);
                }
            }
        }

        // 2. 从 JAVA_HOME 查找
        if let Ok(java_home) = env::var("JAVA_HOME") {
            let candidate = PathBuf::from(java_home).join("bin").join(Self::executable_name());
            if candidate.is_file() {
                found_paths.insert(candidate);
            }
        }

        // 3. 扫描 Windows 常用目录 / Program Files / APPDATA 官方 Runtime
        #[cfg(target_os = "windows")]
        {
            let mut search_dirs = Vec::new();
            if let Ok(pf) = env::var("ProgramFiles") {
                let base = PathBuf::from(pf);
                search_dirs.push(base.join("Java"));
                search_dirs.push(base.join("Eclipse Adoptium"));
                search_dirs.push(base.join("BellSoft"));
                search_dirs.push(base.join("Zulu"));
                search_dirs.push(base.join("Microsoft"));
            }
            if let Ok(pf86) = env::var("ProgramFiles(x86)") {
                search_dirs.push(PathBuf::from(pf86).join("Java"));
            }

            // 扫描 Mojang 官方/启动器自动下载的 runtime 目录
            if let Ok(appdata) = env::var("APPDATA") {
                search_dirs.push(PathBuf::from(appdata).join(".minecraft").join("runtime"));
            }

            // 常见其它盘符路径探测
            search_dirs.push(PathBuf::from(r"D:\Program Files (x86)\.minecraft\runtime"));
            search_dirs.push(PathBuf::from(r"D:\Program Files\.minecraft\runtime"));

            for dir in search_dirs {
                if dir.is_dir() {
                    Self::scan_directory_for_java(&dir, &mut found_paths, 4);
                }
            }
        }

        // 4. 解析每一个 Java 路径的具体版本
        let mut java_list = Vec::new();
        for path in found_paths {
            if let Some(info) = Self::parse_java(&path) {
                java_list.push(info);
            }
        }

        // 按版本号降序排序
        java_list.sort_by(|a, b| b.major_version.cmp(&a.major_version));
        java_list
    }

    fn executable_name() -> &'static str {
        if cfg!(windows) {
            "javaw.exe"
        } else {
            "java"
        }
    }

    fn scan_directory_for_java(dir: &Path, found: &mut HashSet<PathBuf>, max_depth: usize) {
        if max_depth == 0 {
            return;
        }

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if name.eq_ignore_ascii_case("javaw.exe") {
                        found.insert(path);
                    }
                } else if path.is_dir() {
                    Self::scan_directory_for_java(&path, found, max_depth - 1);
                }
            }
        }
    }

    /// 执行 java -version 并提取版本信息
    pub fn parse_java(path: &Path) -> Option<JavaInfo> {
        let output = Command::new(path).arg("-version").output().ok()?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let text = format!("{}\n{}", stderr, stdout);

        let is_64_bit = text.contains("64-Bit") || text.contains("x86_64") || text.contains("amd64");
        let is_jdk = path.parent().and_then(|p| p.parent()).map(|p| p.join("include").exists()).unwrap_or(false);

        let re = Regex::new(r#""(\d+)(\.(\d+))?(\.[\d_\w]+)?"#).ok()?;
        let (full_version, major_version) = if let Some(caps) = re.captures(&text) {
            let full = caps.get(0)?.as_str().replace('"', "");
            let first = caps.get(1)?.as_str().parse::<u32>().unwrap_or(0);
            let major = if first == 1 {
                caps.get(3).and_then(|m| m.as_str().parse::<u32>().ok()).unwrap_or(8)
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
        })
    }
}
