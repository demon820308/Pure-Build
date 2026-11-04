pub mod args;
pub mod natives;
pub mod process;

use crate::reporter::ReporterHandle;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchProgress {
    pub step: String,
    pub percentage: f32,
    pub pid: Option<u32>,
    pub is_finished: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaunchArgumentsResult {
    pub required_java_major: u32,
    pub effective_game_dir: PathBuf,
    pub natives_dir: PathBuf,
    pub classpath_entries: Vec<String>,
    pub classpath: String,
    pub main_class: String,
    pub jvm_args: Vec<String>,
    pub game_args: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LaunchJavaVersion {
    pub component: Option<String>,
    #[serde(rename = "majorVersion")]
    pub major_version: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LaunchVersionJson {
    pub id: String,
    #[serde(rename = "mainClass")]
    pub main_class: String,
    pub arguments: Option<LaunchArguments>,
    #[serde(rename = "minecraftArguments")]
    pub minecraft_arguments: Option<String>,
    pub libraries: Vec<LaunchLibrary>,
    #[serde(rename = "inheritsFrom")]
    pub inherits_from: Option<String>,
    #[serde(rename = "assetIndex")]
    pub asset_index: Option<LaunchAssetIndex>,
    pub assets: Option<String>,
    #[serde(rename = "javaVersion")]
    pub java_version: Option<LaunchJavaVersion>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LaunchAssetIndex {
    pub id: String,
    pub url: Option<String>,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LaunchArguments {
    pub game: Option<Vec<serde_json::Value>>,
    pub jvm: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LaunchLibrary {
    pub name: String,
    pub downloads: Option<LaunchLibraryDownloads>,
    pub rules: Option<Vec<LaunchRule>>,
    pub natives: Option<HashMap<String, String>>,
    pub extract: Option<LaunchExtractRule>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LaunchLibraryDownloads {
    pub artifact: Option<LaunchArtifact>,
    pub classifiers: Option<HashMap<String, LaunchArtifact>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LaunchArtifact {
    pub path: Option<String>,
    pub url: Option<String>,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LaunchExtractRule {
    pub exclude: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LaunchRule {
    pub action: String,
    pub os: Option<LaunchOsRule>,
    pub features: Option<HashMap<String, bool>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LaunchOsRule {
    pub name: Option<String>,
    pub arch: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ConditionalArgument {
    pub rules: Option<Vec<LaunchRule>>,
    pub value: serde_json::Value,
}

pub struct MinecraftLauncher;

impl MinecraftLauncher {
    /// 执行拉起游戏进程 (带正版/离线身份，解压当前平台 natives，并异步监控崩溃日志)
    pub async fn launch_with_auth(
        reporter: ReporterHandle,
        game_dir_str: &str,
        version_id: &str,
        java_path_str: &str,
        player_name: &str,
        max_memory_mb: u32,
        uuid: Option<&str>,
        access_token: Option<&str>,
    ) -> Result<String, String> {
        process::launch_with_auth(
            reporter,
            game_dir_str,
            version_id,
            java_path_str,
            player_name,
            max_memory_mb,
            uuid,
            access_token,
        )
        .await
    }

    /// 纯函数式构造 Minecraft 启动所需的所有命令行参数与运行时上下文
    pub fn build_launch_arguments(
        ver_json: &LaunchVersionJson,
        game_dir: &Path,
        version_id: &str,
        target_os: &str,
        target_arch: &str,
        target_os_version: Option<&str>,
        player_name: &str,
        max_memory_mb: u32,
        uuid: Option<&str>,
        access_token: Option<&str>,
        window_width: u32,
        window_height: u32,
        extract_natives: bool,
    ) -> Result<LaunchArgumentsResult, String> {
        args::build_launch_arguments(
            ver_json,
            game_dir,
            version_id,
            target_os,
            target_arch,
            target_os_version,
            player_name,
            max_memory_mb,
            uuid,
            access_token,
            window_width,
            window_height,
            extract_natives,
        )
    }

    /// 读取版本 JSON，若存在 `inheritsFrom` 且未内联父版本支持库，则自动合并父版本 JSON
    pub fn load_version_json_with_inheritance(
        game_dir: &Path,
        version_dir: &Path,
        version_id: &str,
    ) -> Result<LaunchVersionJson, String> {
        args::load_version_json_with_inheritance(game_dir, version_dir, version_id)
    }

    /// 将父版本（原版）与子版本（Forge/Fabric/NeoForge 等）的 LaunchVersionJson 合并
    pub fn merge_inherited_json(
        parent: LaunchVersionJson,
        child: LaunchVersionJson,
    ) -> LaunchVersionJson {
        args::merge_inherited_json(parent, child)
    }

    /// 按 Mojang 官方 rules 语义判定库或参数是否适用于目标平台
    pub fn check_rules_for_target(
        rules: &Option<Vec<LaunchRule>>,
        target_os: &str,
        target_arch: &str,
        target_os_version: Option<&str>,
        active_features: &HashMap<String, bool>,
    ) -> bool {
        natives::check_rules_for_target(
            rules,
            target_os,
            target_arch,
            target_os_version,
            active_features,
        )
    }

    /// 从版本 JSON 的 natives 映射表查表取用 classifier
    pub fn select_native_classifier<'a>(
        lib: &'a LaunchLibrary,
        target_os: &str,
        target_arch: &str,
    ) -> Option<(String, &'a LaunchArtifact)> {
        natives::select_native_classifier(lib, target_os, target_arch)
    }

    /// 判断是否为 1.19+ 风格通过独立库坐标声明的 native 罐
    pub fn is_modern_native_library(lib: &LaunchLibrary) -> bool {
        natives::is_modern_native_library(lib)
    }

    /// 安全解压 Native JAR 到目标目录
    pub fn extract_native_jar(
        jar_path: &Path,
        natives_dir: &Path,
        custom_excludes: Option<&[String]>,
    ) -> Result<(), String> {
        natives::extract_native_jar(jar_path, natives_dir, custom_excludes)
    }

    /// 生成去重用的 package key
    pub fn get_package_key(name: &str) -> String {
        args::get_package_key(name)
    }

    /// 将 artifact.path（正斜杠相对路径）按 PathBuf::join 拆解组合
    pub fn artifact_path_to_pathbuf(path_str: &str) -> PathBuf {
        args::artifact_path_to_pathbuf(path_str)
    }

    /// 将 Maven 坐标转换为本地相对路径
    pub fn maven_to_path(name: &str) -> PathBuf {
        args::maven_to_path(name)
    }

    pub fn replace_vars(template: &str, vars: &HashMap<&str, String>) -> String {
        args::replace_vars(template, vars)
    }

    pub fn resolve_arguments(
        raw_args: &[serde_json::Value],
        target_os: &str,
        target_arch: &str,
        target_os_version: Option<&str>,
        active_features: &HashMap<String, bool>,
        vars: &HashMap<&str, String>,
    ) -> Vec<String> {
        args::resolve_arguments(
            raw_args,
            target_os,
            target_arch,
            target_os_version,
            active_features,
            vars,
        )
    }

    /// 强制终止指定的 Minecraft 游戏进程
    pub fn kill_process(pid: u32) -> Result<(), String> {
        process::kill_process(pid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_version_json() -> LaunchVersionJson {
        let raw = r#"{
            "id": "1.20.4-multi-test",
            "mainClass": "net.minecraft.client.main.Main",
            "arguments": {
                "game": [
                    "--username",
                    "${auth_player_name}",
                    {
                        "rules": [{ "action": "allow", "features": { "is_demo_user": true } }],
                        "value": "--demo"
                    },
                    {
                        "rules": [{ "action": "allow", "features": { "has_custom_resolution": true } }],
                        "value": ["--width", "${resolution_width}", "--height", "${resolution_height}"]
                    }
                ],
                "jvm": [
                    {
                        "rules": [{ "action": "allow", "os": { "name": "osx" } }],
                        "value": ["-XstartOnFirstThread"]
                    },
                    {
                        "rules": [{ "action": "allow", "os": { "name": "windows" } }],
                        "value": "-XX:HeapDumpPath=MojangTricksIntelDriversForPerformance_javaw.exe_minecraft.exe.heapdump"
                    },
                    {
                        "rules": [{ "action": "allow", "os": { "arch": "x86" } }],
                        "value": "-Xss1M"
                    }
                ]
            },
            "libraries": [
                {
                    "name": "org.lwjgl:lwjgl:3.3.2",
                    "downloads": {
                        "artifact": { "path": "org/lwjgl/lwjgl/3.3.2/lwjgl-3.3.2.jar" }
                    }
                },
                {
                    "name": "org.lwjgl:lwjgl-glfw:3.3.2:natives-windows",
                    "downloads": {
                        "artifact": { "path": "org/lwjgl/lwjgl-glfw/3.3.2/lwjgl-glfw-3.3.2-natives-windows.jar" }
                    },
                    "rules": [
                        { "action": "allow", "os": { "name": "windows", "arch": "x64" } }
                    ]
                },
                {
                    "name": "org.lwjgl:lwjgl-glfw:3.3.2:natives-macos",
                    "downloads": {
                        "artifact": { "path": "org/lwjgl/lwjgl-glfw/3.3.2/lwjgl-glfw-3.3.2-natives-macos.jar" }
                    },
                    "rules": [
                        { "action": "allow", "os": { "name": "osx", "arch": "x64" } }
                    ]
                },
                {
                    "name": "org.lwjgl:lwjgl-glfw:3.3.2:natives-macos-arm64",
                    "downloads": {
                        "artifact": { "path": "org/lwjgl/lwjgl-glfw/3.3.2/lwjgl-glfw-3.3.2-natives-macos-arm64.jar" }
                    },
                    "rules": [
                        { "action": "allow", "os": { "name": "osx", "arch": "arm64" } }
                    ]
                },
                {
                    "name": "org.lwjgl:lwjgl-glfw:3.3.2:natives-linux",
                    "downloads": {
                        "artifact": { "path": "org/lwjgl/lwjgl-glfw/3.3.2/lwjgl-glfw-3.3.2-natives-linux.jar" }
                    },
                    "rules": [
                        { "action": "allow", "os": { "name": "linux", "arch": "x64" } }
                    ]
                },
                {
                    "name": "org.lwjgl:lwjgl-legacy:2.9.4-nightly",
                    "natives": {
                        "linux": "natives-linux",
                        "osx": "natives-osx",
                        "windows": "natives-windows-${arch}"
                    },
                    "downloads": {
                        "classifiers": {
                            "natives-linux": { "path": "org/lwjgl/lwjgl-legacy/2.9.4-nightly/lwjgl-legacy-2.9.4-nightly-natives-linux.jar" },
                            "natives-osx": { "path": "org/lwjgl/lwjgl-legacy/2.9.4-nightly/lwjgl-legacy-2.9.4-nightly-natives-osx.jar" },
                            "natives-windows-32": { "path": "org/lwjgl/lwjgl-legacy/2.9.4-nightly/lwjgl-legacy-2.9.4-nightly-natives-windows-32.jar" },
                            "natives-windows-64": { "path": "org/lwjgl/lwjgl-legacy/2.9.4-nightly/lwjgl-legacy-2.9.4-nightly-natives-windows-64.jar" }
                        }
                    },
                    "rules": [
                        { "action": "allow" }
                    ]
                }
            ]
        }"#;

        serde_json::from_str(raw).expect("反序列化多平台测试 JSON 样本失败")
    }

    #[test]
    fn test_rules_and_classifiers_across_four_platform_vectors() {
        let sample = sample_version_json();

        let mut features = HashMap::new();
        features.insert("is_demo_user".to_string(), false);
        features.insert("has_custom_resolution".to_string(), true);

        let mut vars = HashMap::new();
        vars.insert("auth_player_name", "TestUser".to_string());
        vars.insert("resolution_width", "1280".to_string());
        vars.insert("resolution_height", "720".to_string());

        let collect_for_target = |os: &str, arch: &str| -> (Vec<String>, Vec<String>, Vec<String>) {
            let mut accepted_libs = Vec::new();
            let mut classifiers = Vec::new();

            for lib in &sample.libraries {
                if MinecraftLauncher::check_rules_for_target(
                    &lib.rules,
                    os,
                    arch,
                    None,
                    &features,
                ) {
                    accepted_libs.push(lib.name.clone());
                }

                if let Some((cls, _)) = MinecraftLauncher::select_native_classifier(lib, os, arch) {
                    classifiers.push(cls);
                }
            }

            let jvm_resolved = sample
                .arguments
                .as_ref()
                .and_then(|a| a.jvm.as_ref())
                .map(|jvms| {
                    MinecraftLauncher::resolve_arguments(
                        jvms,
                        os,
                        arch,
                        None,
                        &features,
                        &vars,
                    )
                })
                .unwrap_or_default();

            (accepted_libs, classifiers, jvm_resolved)
        };

        // 1. (windows, x64) 向量
        let (win_libs, win_cls, win_jvm) = collect_for_target("windows", "x64");
        assert!(win_libs.contains(&"org.lwjgl:lwjgl:3.3.2".to_string()));
        assert!(win_libs.contains(&"org.lwjgl:lwjgl-glfw:3.3.2:natives-windows".to_string()));
        assert!(!win_libs.contains(&"org.lwjgl:lwjgl-glfw:3.3.2:natives-macos".to_string()));
        assert!(!win_libs.contains(&"org.lwjgl:lwjgl-glfw:3.3.2:natives-linux".to_string()));
        assert_eq!(win_cls, vec!["natives-windows-64"]);
        assert!(win_jvm.iter().any(|arg| arg.contains("HeapDumpPath")));
        assert!(!win_jvm.contains(&"-XstartOnFirstThread".to_string()));

        // 2. (osx, arm64) 向量
        let (mac_arm_libs, mac_arm_cls, mac_arm_jvm) = collect_for_target("osx", "arm64");
        assert!(mac_arm_libs.contains(&"org.lwjgl:lwjgl:3.3.2".to_string()));
        assert!(mac_arm_libs.contains(&"org.lwjgl:lwjgl-glfw:3.3.2:natives-macos-arm64".to_string()));
        assert!(!mac_arm_libs.contains(&"org.lwjgl:lwjgl-glfw:3.3.2:natives-macos".to_string()));
        assert!(!mac_arm_libs.contains(&"org.lwjgl:lwjgl-glfw:3.3.2:natives-windows".to_string()));
        assert_eq!(mac_arm_cls, vec!["natives-osx"]);
        assert!(mac_arm_jvm.contains(&"-XstartOnFirstThread".to_string()));
        assert!(!mac_arm_jvm.iter().any(|arg| arg.contains("HeapDumpPath")));

        // 3. (osx, x64) 向量
        let (mac_x64_libs, mac_x64_cls, mac_x64_jvm) = collect_for_target("osx", "x64");
        assert!(mac_x64_libs.contains(&"org.lwjgl:lwjgl:3.3.2".to_string()));
        assert!(mac_x64_libs.contains(&"org.lwjgl:lwjgl-glfw:3.3.2:natives-macos".to_string()));
        assert!(!mac_x64_libs.contains(&"org.lwjgl:lwjgl-glfw:3.3.2:natives-macos-arm64".to_string()));
        assert_eq!(mac_x64_cls, vec!["natives-osx"]);
        assert!(mac_x64_jvm.contains(&"-XstartOnFirstThread".to_string()));

        // 4. (linux, x64) 向量
        let (linux_libs, linux_cls, linux_jvm) = collect_for_target("linux", "x64");
        assert!(linux_libs.contains(&"org.lwjgl:lwjgl:3.3.2".to_string()));
        assert!(linux_libs.contains(&"org.lwjgl:lwjgl-glfw:3.3.2:natives-linux".to_string()));
        assert!(!linux_libs.contains(&"org.lwjgl:lwjgl-glfw:3.3.2:natives-windows".to_string()));
        assert_eq!(linux_cls, vec!["natives-linux"]);
        assert!(!linux_jvm.contains(&"-XstartOnFirstThread".to_string()));
        assert!(!linux_jvm.iter().any(|arg| arg.contains("HeapDumpPath")));
    }

    #[test]
    fn test_last_matching_rule_wins_and_os_version_regex() {
        let rules_json = r#"[
            { "action": "allow" },
            { "action": "disallow", "os": { "name": "osx", "version": "^10\\.15\\." } },
            { "action": "allow", "os": { "name": "osx", "version": "^10\\.15\\.7" } }
        ]"#;
        let rules: Vec<LaunchRule> = serde_json::from_str(rules_json).unwrap();
        let features = HashMap::new();

        // 10.15.6 命中第二条 disallow 且未命中第三条 -> 结果 disallow (false)
        assert!(!MinecraftLauncher::check_rules_for_target(
            &Some(rules.clone()),
            "osx",
            "x64",
            Some("10.15.6"),
            &features,
        ));

        // 10.15.7 命中第三条 allow（最后匹配胜出） -> 结果 allow (true)
        assert!(MinecraftLauncher::check_rules_for_target(
            &Some(rules.clone()),
            "osx",
            "x64",
            Some("10.15.7"),
            &features,
        ));

        // Linux 不匹配任何针对 osx 的规则，由第一条兜底规则 allow -> 结果 allow (true)
        assert!(MinecraftLauncher::check_rules_for_target(
            &Some(rules),
            "linux",
            "x64",
            None,
            &features,
        ));
    }

    #[test]
    fn test_merge_inherited_json_child_priority_and_args() {
        let parent_raw = r#"{
            "id": "1.20.4",
            "mainClass": "net.minecraft.client.main.Main",
            "assets": "12",
            "arguments": {
                "game": ["--username", "${auth_player_name}"],
                "jvm": ["-cp", "${classpath}"]
            },
            "libraries": [
                { "name": "com.mojang:logging:1.1.1" }
            ]
        }"#;
        let child_raw = r#"{
            "id": "1.20.4-Forge",
            "inheritsFrom": "1.20.4",
            "mainClass": "cpw.mods.bootstraplauncher.BootstrapLauncher",
            "arguments": {
                "game": ["--fml.forgeVersion", "49.0.30"],
                "jvm": ["-Djava.net.preferIPv6Addresses=system"]
            },
            "libraries": [
                { "name": "net.minecraftforge:fmlloader:1.20.4-49.0.30" }
            ]
        }"#;

        let parent: LaunchVersionJson = serde_json::from_str(parent_raw).unwrap();
        let child: LaunchVersionJson = serde_json::from_str(child_raw).unwrap();
        let merged = MinecraftLauncher::merge_inherited_json(parent, child);

        assert_eq!(
            merged.main_class,
            "cpw.mods.bootstraplauncher.BootstrapLauncher"
        );
        assert_eq!(merged.assets.as_deref(), Some("12"));
        assert_eq!(merged.libraries.len(), 2);
        // Child library is appended after parent so cp_map.insert(pkg_key, path) overrides parent version
        assert_eq!(
            merged.libraries[1].name,
            "net.minecraftforge:fmlloader:1.20.4-49.0.30"
        );
    }
}
