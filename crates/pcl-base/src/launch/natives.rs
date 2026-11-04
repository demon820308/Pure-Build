use super::{LaunchArtifact, LaunchLibrary, LaunchRule};
use regex::Regex;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io;
use std::path::{Component, Path};
use zip::ZipArchive;

/// 按 Mojang 官方 rules 语义判定库或参数是否适用于目标平台
/// 无 rules -> 允许；有 rules -> 从上到下逐条匹配，最后一条匹配的规则决定 allow/disallow。
pub fn check_rules_for_target(
    rules: &Option<Vec<LaunchRule>>,
    target_os: &str,
    target_arch: &str,
    target_os_version: Option<&str>,
    active_features: &HashMap<String, bool>,
) -> bool {
    let Some(rule_list) = rules else {
        return true;
    };
    if rule_list.is_empty() {
        return true;
    }

    let mut allow = false;
    for rule in rule_list {
        let mut matched = true;

        if let Some(ref os_rule) = rule.os {
            if let Some(ref req_name) = os_rule.name {
                if req_name != target_os {
                    matched = false;
                }
            }
            if matched {
                if let Some(ref req_arch) = os_rule.arch {
                    if req_arch != target_arch {
                        matched = false;
                    }
                }
            }
            if matched {
                if let Some(ref ver_regex) = os_rule.version {
                    if let Some(actual_ver) = target_os_version {
                        if let Ok(re) = Regex::new(ver_regex) {
                            if !re.is_match(actual_ver) {
                                matched = false;
                            }
                        } else {
                            matched = false;
                        }
                    } else {
                        // 若未提供系统版本号，默认不因 os.version 拦截（兼容未指定系统小版本的常规匹配）
                        matched = true;
                    }
                }
            }
        }

        if matched {
            if let Some(ref feat_map) = rule.features {
                for (feat_key, expected_val) in feat_map {
                    let actual_val = active_features.get(feat_key).copied().unwrap_or(false);
                    if actual_val != *expected_val {
                        matched = false;
                        break;
                    }
                }
            }
        }

        if matched {
            allow = rule.action == "allow";
        }
    }

    allow
}

/// classifier 必须从版本 JSON 的 natives 映射表查表取用，禁止自行拼接 — 拼错会导致 natives 选错平台。
pub fn select_native_classifier<'a>(
    lib: &'a LaunchLibrary,
    target_os: &str,
    target_arch: &str,
) -> Option<(String, &'a LaunchArtifact)> {
    let natives_map = lib.natives.as_ref()?;
    let classifiers_map = lib.downloads.as_ref()?.classifiers.as_ref()?;

    let raw_classifier_template = natives_map.get(target_os)?;

    // 处理模板中可能包含的 ${arch} 占位符（如 natives-<os>-${arch}）
    let bit_str = match target_arch {
        "x86" => "32",
        _ => "64",
    };
    let resolved_key = raw_classifier_template.replace("${arch}", bit_str);

    if let Some(artifact) = classifiers_map.get(&resolved_key) {
        return Some((resolved_key, artifact));
    }

    // 若模板按 arch 名称直接替换（如 arm64）
    let alt_key = raw_classifier_template.replace("${arch}", target_arch);
    if let Some(artifact) = classifiers_map.get(&alt_key) {
        return Some((alt_key, artifact));
    }

    None
}

/// 判断是否为 1.19+ 风格通过独立库坐标声明的 native 罐（第 4 段 classifier 以 `natives-` 开头）
pub fn is_modern_native_library(lib: &LaunchLibrary) -> bool {
    let parts: Vec<&str> = lib.name.split(':').collect();
    parts.len() >= 4 && parts[3].starts_with("natives-")
}

/// 安全解压 Native JAR 到目标目录（剔除 META-INF 与签名文件，使用 Path 组件防止 zip-slip）
pub fn extract_native_jar(
    jar_path: &Path,
    natives_dir: &Path,
    custom_excludes: Option<&[String]>,
) -> Result<(), String> {
    let file = File::open(jar_path).map_err(|e| format!("打开 native jar 失败: {}", e))?;
    let mut archive =
        ZipArchive::new(file).map_err(|e| format!("解析 native jar 失败: {}", e))?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        if entry.is_dir() {
            continue;
        }

        // enclosed_name 自动拒绝包含 `..` 或绝对盘符/根目录的逃逸路径
        let Some(enclosed) = entry.enclosed_name() else {
            continue;
        };

        let normalized_str = enclosed.to_string_lossy();
        // 剔除 META-INF 及签名/校验文件
        if normalized_str.starts_with("META-INF")
            || normalized_str.ends_with(".SF")
            || normalized_str.ends_with(".DSA")
            || normalized_str.ends_with(".RSA")
            || normalized_str.ends_with(".git")
            || normalized_str.ends_with(".sha1")
        {
            continue;
        }

        if let Some(excludes) = custom_excludes {
            if excludes.iter().any(|ex| normalized_str.starts_with(ex)) {
                continue;
            }
        }

        // 以 Path::components 方式逐级安全 join 落盘（防 zip-slip）
        let mut dest_path = natives_dir.to_path_buf();
        for comp in enclosed.components() {
            if let Component::Normal(part) = comp {
                dest_path = dest_path.join(part);
            }
        }

        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        if let Ok(mut out_file) = File::create(&dest_path) {
            let _ = io::copy(&mut entry, &mut out_file);
        }
    }

    Ok(())
}
