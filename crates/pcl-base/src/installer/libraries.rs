use crate::endpoints;
use crate::installer::download::BatchDownloadTask;
use crate::launch::{LaunchLibrary, MinecraftLauncher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// 将 Maven 坐标解析为本地存储相对路径及远程 URL 子路径
pub fn maven_to_path_and_url(maven_coord: &str) -> (PathBuf, String) {
    let parts: Vec<&str> = maven_coord.split(':').collect();
    if parts.len() < 3 {
        return (PathBuf::from(maven_coord), maven_coord.to_string());
    }

    let group = parts[0];
    let artifact = parts[1];
    let version = parts[2];
    let (classifier, ext) = if parts.len() >= 4 {
        let last_part = parts[3];
        if let Some(dot_idx) = last_part.rfind('@') {
            (Some(&last_part[..dot_idx]), &last_part[dot_idx + 1..])
        } else {
            (Some(last_part), "jar")
        }
    } else {
        (None, "jar")
    };

    let mut rel_path = PathBuf::new();
    for seg in group.split('.') {
        rel_path.push(seg);
    }
    rel_path.push(artifact);
    rel_path.push(version);

    let file_name = if let Some(cls) = classifier {
        format!("{}-{}-{}.{}", artifact, version, cls, ext)
    } else {
        format!("{}-{}.{}", artifact, version, ext)
    };
    rel_path.push(&file_name);

    let url_subpath = rel_path
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/");

    (rel_path, url_subpath)
}

/// 从版本 libraries JSON 数组中收集适用于当前平台的所有支持库与 Natives 下载任务
pub fn collect_library_tasks(
    lib_values: &[serde_json::Value],
    libraries_dir: &Path,
    natives_dir: Option<&Path>,
    prefer_mirror: bool,
    target_os: &str,
    target_arch: &str,
    default_maven_base: Option<&str>,
) -> Vec<BatchDownloadTask> {
    let active_features = HashMap::new();
    let mut tasks = Vec::new();

    for lib_val in lib_values {
        let Ok(parsed_lib) = serde_json::from_value::<LaunchLibrary>(lib_val.clone()) else {
            continue;
        };

        // 不适用于当前平台的库不下载、不进 classpath
        if !MinecraftLauncher::check_rules_for_target(
            &parsed_lib.rules,
            target_os,
            target_arch,
            None,
            &active_features,
        ) {
            continue;
        }

        let extract_opt = if MinecraftLauncher::is_modern_native_library(&parsed_lib) {
            natives_dir.map(|nd| {
                (
                    nd.to_path_buf(),
                    parsed_lib.extract.as_ref().and_then(|e| e.exclude.clone()),
                )
            })
        } else {
            None
        };

        if let Some(ref artifact) = parsed_lib
            .downloads
            .as_ref()
            .and_then(|d| d.artifact.clone())
        {
            if let (Some(path_str), Some(url_str)) = (&artifact.path, &artifact.url) {
                if !url_str.trim().is_empty() {
                    let rel_path = MinecraftLauncher::artifact_path_to_pathbuf(path_str);
                    let urls = endpoints::resolve_mojang_library_urls(url_str, prefer_mirror);
                    tasks.push(BatchDownloadTask {
                        target_path: libraries_dir.join(rel_path),
                        urls,
                        expected_size: artifact.size,
                        expected_sha1: artifact.sha1.clone(),
                        extract_natives: extract_opt,
                    });
                }
            }
        } else if parsed_lib.downloads.is_none() {
            let (rel_path, url_subpath) = maven_to_path_and_url(&parsed_lib.name);
            let repo_base = parsed_lib.url.as_deref().or(default_maven_base);
            let urls = endpoints::resolve_maven_library_urls_list(
                repo_base,
                &url_subpath,
                prefer_mirror,
            );
            tasks.push(BatchDownloadTask {
                target_path: libraries_dir.join(rel_path),
                urls,
                expected_size: None,
                expected_sha1: None, // shortcut: Fabric/Forge 等第三方自定义 Maven 依赖库在版本清单中未提供 SHA-1，维持大小校验
                extract_natives: extract_opt,
            });
        }

        // 查表收集 classifiers 中的平台专属 natives 罐
        if let Some((_cls_key, native_art)) =
            MinecraftLauncher::select_native_classifier(&parsed_lib, target_os, target_arch)
        {
            if let (Some(path_str), Some(url_str)) = (&native_art.path, &native_art.url) {
                let rel_path = MinecraftLauncher::artifact_path_to_pathbuf(path_str);
                let urls = endpoints::resolve_mojang_library_urls(url_str, prefer_mirror);
                let extract_cls = natives_dir.map(|nd| {
                    (
                        nd.to_path_buf(),
                        parsed_lib.extract.as_ref().and_then(|e| e.exclude.clone()),
                    )
                });
                tasks.push(BatchDownloadTask {
                    target_path: libraries_dir.join(rel_path),
                    urls,
                    expected_size: native_art.size,
                    expected_sha1: native_art.sha1.clone(),
                    extract_natives: extract_cls,
                });
            }
        }
    }

    tasks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_maven_to_path_and_url() {
        let (rel_path, url_subpath) = maven_to_path_and_url("org.ow2.asm:asm-commons:9.6");
        assert_eq!(
            rel_path,
            PathBuf::from("org")
                .join("ow2")
                .join("asm")
                .join("asm-commons")
                .join("9.6")
                .join("asm-commons-9.6.jar")
        );
        assert_eq!(url_subpath, "org/ow2/asm/asm-commons/9.6/asm-commons-9.6.jar");

        let (rel_path_clf, url_subpath_clf) =
            maven_to_path_and_url("net.minecraftforge:forge:1.20.1-47.2.0:universal");
        assert_eq!(
            rel_path_clf,
            PathBuf::from("net")
                .join("minecraftforge")
                .join("forge")
                .join("1.20.1-47.2.0")
                .join("forge-1.20.1-47.2.0-universal.jar")
        );
        assert_eq!(
            url_subpath_clf,
            "net/minecraftforge/forge/1.20.1-47.2.0/forge-1.20.1-47.2.0-universal.jar"
        );
    }

    #[test]
    fn test_collect_library_tasks_four_platform_vectors() {
        let sample_libs = vec![
            serde_json::json!({
                "name": "com.google.guava:guava:31.1-jre",
                "downloads": {
                    "artifact": {
                        "path": "com/google/guava/guava/31.1-jre/guava-31.1-jre.jar",
                        "url": "https://libraries.minecraft.net/com/google/guava/guava/31.1-jre/guava-31.1-jre.jar",
                        "size": 2991871
                    }
                }
            }),
            serde_json::json!({
                "name": "net.java.jinput:jinput-platform:2.0.5",
                "rules": [
                    { "action": "allow", "os": { "name": "windows" } }
                ],
                "downloads": {
                    "artifact": {
                        "path": "net/java/jinput/jinput-platform/2.0.5/jinput-platform-windows.jar",
                        "url": "https://libraries.minecraft.net/net/java/jinput/jinput-platform/2.0.5/jinput-platform-windows.jar",
                        "size": 1000
                    }
                }
            }),
            serde_json::json!({
                "name": "ca.weblite:java-objc-bridge:1.1.0",
                "rules": [
                    { "action": "allow", "os": { "name": "osx", "arch": "arm64" } }
                ],
                "downloads": {
                    "artifact": {
                        "path": "ca/weblite/java-objc-bridge/1.1.0/java-objc-bridge-1.1.0-arm64.jar",
                        "url": "https://libraries.minecraft.net/ca/weblite/java-objc-bridge/1.1.0/java-objc-bridge-1.1.0-arm64.jar",
                        "size": 2000
                    }
                }
            }),
            serde_json::json!({
                "name": "org.lwjgl:lwjgl:3.3.1",
                "natives": {
                    "windows": "natives-windows",
                    "osx": "natives-macos",
                    "linux": "natives-linux"
                },
                "downloads": {
                    "artifact": {
                        "path": "org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1.jar",
                        "url": "https://libraries.minecraft.net/org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1.jar",
                        "size": 3000
                    },
                    "classifiers": {
                        "natives-windows": {
                            "path": "org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1-natives-windows.jar",
                            "url": "https://libraries.minecraft.net/org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1-natives-windows.jar",
                            "size": 4000
                        },
                        "natives-macos": {
                            "path": "org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1-natives-macos.jar",
                            "url": "https://libraries.minecraft.net/org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1-natives-macos.jar",
                            "size": 5000
                        },
                        "natives-macos-arm64": {
                            "path": "org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1-natives-macos-arm64.jar",
                            "url": "https://libraries.minecraft.net/org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1-natives-macos-arm64.jar",
                            "size": 6000
                        },
                        "natives-linux": {
                            "path": "org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1-natives-linux.jar",
                            "url": "https://libraries.minecraft.net/org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1-natives-linux.jar",
                            "size": 7000
                        }
                    }
                }
            })
        ];

        let libs_dir = Path::new("mock_libraries");
        let natives_dir = Path::new("mock_natives");

        let win_tasks = collect_library_tasks(
            &sample_libs,
            libs_dir,
            Some(natives_dir),
            false,
            "windows",
            "x64",
            None,
        );
        let win_paths: Vec<String> = win_tasks
            .iter()
            .map(|t| t.target_path.to_string_lossy().replace('\\', "/"))
            .collect();
        assert!(win_paths.iter().any(|p| p.contains("guava-31.1-jre.jar")));
        assert!(win_paths.iter().any(|p| p.contains("jinput-platform-windows.jar")));
        assert!(!win_paths.iter().any(|p| p.contains("java-objc-bridge")));
        assert!(win_paths.iter().any(|p| p.contains("lwjgl-3.3.1.jar")));
        assert!(win_paths.iter().any(|p| p.contains("lwjgl-3.3.1-natives-windows.jar")));

        let mac_arm_tasks = collect_library_tasks(
            &sample_libs,
            libs_dir,
            Some(natives_dir),
            false,
            "osx",
            "arm64",
            None,
        );
        let mac_arm_paths: Vec<String> = mac_arm_tasks
            .iter()
            .map(|t| t.target_path.to_string_lossy().replace('\\', "/"))
            .collect();
        assert!(mac_arm_paths.iter().any(|p| p.contains("guava-31.1-jre.jar")));
        assert!(!mac_arm_paths.iter().any(|p| p.contains("jinput-platform-windows.jar")));
        assert!(mac_arm_paths.iter().any(|p| p.contains("java-objc-bridge-1.1.0-arm64.jar")));
        assert!(mac_arm_paths.iter().any(|p| p.contains("lwjgl-3.3.1.jar")));
        assert!(mac_arm_paths.iter().any(|p| p.contains("lwjgl-3.3.1-natives-macos.jar")));

        let mac_x64_tasks = collect_library_tasks(
            &sample_libs,
            libs_dir,
            Some(natives_dir),
            false,
            "osx",
            "x64",
            None,
        );
        let mac_x64_paths: Vec<String> = mac_x64_tasks
            .iter()
            .map(|t| t.target_path.to_string_lossy().replace('\\', "/"))
            .collect();
        assert!(mac_x64_paths.iter().any(|p| p.contains("guava-31.1-jre.jar")));
        assert!(!mac_x64_paths.iter().any(|p| p.contains("jinput-platform-windows.jar")));
        assert!(!mac_x64_paths.iter().any(|p| p.contains("java-objc-bridge")));
        assert!(mac_x64_paths.iter().any(|p| p.contains("lwjgl-3.3.1.jar")));
        assert!(mac_x64_paths.iter().any(|p| p.contains("lwjgl-3.3.1-natives-macos.jar")));

        let linux_tasks = collect_library_tasks(
            &sample_libs,
            libs_dir,
            Some(natives_dir),
            false,
            "linux",
            "x64",
            None,
        );
        let linux_paths: Vec<String> = linux_tasks
            .iter()
            .map(|t| t.target_path.to_string_lossy().replace('\\', "/"))
            .collect();
        assert!(linux_paths.iter().any(|p| p.contains("guava-31.1-jre.jar")));
        assert!(!linux_paths.iter().any(|p| p.contains("jinput-platform-windows.jar")));
        assert!(!linux_paths.iter().any(|p| p.contains("java-objc-bridge")));
        assert!(linux_paths.iter().any(|p| p.contains("lwjgl-3.3.1.jar")));
        assert!(linux_paths.iter().any(|p| p.contains("lwjgl-3.3.1-natives-linux.jar")));
    }
}
