use crate::config::ConfigManager;
use crate::crash::CrashAnalyzer;
use crate::platform;
use crate::reporter::ReporterHandle;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use zip::ZipArchive;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchProgress {
    pub step: String,
    pub percentage: f32,
    pub pid: Option<u32>,
    pub is_finished: bool,
    pub error: Option<String>,
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
struct ConditionalArgument {
    rules: Option<Vec<LaunchRule>>,
    value: serde_json::Value,
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
        let emit_progress =
            |step: &str, pct: f32, pid: Option<u32>, finished: bool, err: Option<String>| {
                reporter.emit_launch(LaunchProgress {
                    step: step.to_string(),
                    percentage: pct,
                    pid,
                    is_finished: finished,
                    error: err,
                });
            };

        emit_progress("正在检查游戏环境与版本元数据...", 15.0, None, false, None);

        let game_dir = Path::new(game_dir_str);
        let version_dir = game_dir.join("versions").join(version_id);
        let json_path = version_dir.join(format!("{}.json", version_id));

        if !json_path.exists() {
            let err = format!("找不到版本元数据 JSON: {:?}", json_path);
            emit_progress("启动失败", 0.0, None, true, Some(err.clone()));
            return Err(err);
        }

        let ver_json = Self::load_version_json_with_inheritance(game_dir, &version_dir, version_id)?;

        let required_major = ver_json
            .java_version
            .as_ref()
            .and_then(|j| j.major_version)
            .unwrap_or(8);

        emit_progress(
            &format!("正在校验 Java 环境 (目标主版本: Java {})...", required_major),
            20.0,
            None,
            false,
            None,
        );

        let effective_java_buf = crate::java::JavaDownloader::ensure_java_for_version(
            &reporter,
            game_dir,
            java_path_str,
            required_major,
        )
        .await
        .map_err(|e| {
            let err_msg = format!("Java 校验或自动下载失败: {}", e);
            emit_progress("启动失败", 0.0, None, true, Some(err_msg.clone()));
            err_msg
        })?;

        let effective_java_str = effective_java_buf.to_string_lossy().to_string();

        let target_os = platform::mc_os_name();
        let target_arch = platform::mc_os_arch();
        let cfg = ConfigManager::load();

        let mut active_features = HashMap::new();
        active_features.insert("is_demo_user".to_string(), false);
        active_features.insert(
            "has_custom_resolution".to_string(),
            cfg.window_width > 0 && cfg.window_height > 0,
        );

        // 1. 搜集 Classpath 并解压当前平台的 Natives 罐
        emit_progress(
            "正在按平台规则解析运行库与解压 Natives...",
            40.0,
            None,
            false,
            None,
        );
        let libraries_dir = game_dir.join("libraries");
        let natives_dir = version_dir.join(format!("{}-natives", version_id));
        if !natives_dir.exists() {
            fs::create_dir_all(&natives_dir)
                .map_err(|e| format!("创建 natives 目录失败: {}", e))?;
        }

        let mut cp_map: HashMap<String, String> = HashMap::new();
        let mut cp_order: Vec<String> = Vec::new();

        for lib in &ver_json.libraries {
            if !Self::check_rules_for_target(&lib.rules, target_os, target_arch, None, &active_features)
            {
                continue;
            }

            // 1.1 处理普通 artifact 或 1.19+ 独立 natives artifact
            let has_main_artifact = lib
                .downloads
                .as_ref()
                .and_then(|d| d.artifact.as_ref())
                .is_some()
                || lib.downloads.is_none();

            if has_main_artifact {
                let rel_path = if let Some(path_str) = lib
                    .downloads
                    .as_ref()
                    .and_then(|d| d.artifact.as_ref())
                    .and_then(|a| a.path.as_ref())
                {
                    Self::artifact_path_to_pathbuf(path_str)
                } else {
                    Self::maven_to_path(&lib.name)
                };

                let full_lib_path = libraries_dir.join(&rel_path);
                if full_lib_path.is_file() {
                    if Self::is_modern_native_library(lib) {
                        let excludes = lib.extract.as_ref().and_then(|e| e.exclude.as_deref());
                        let _ = Self::extract_native_jar(&full_lib_path, &natives_dir, excludes);
                    }

                    let pkg_key = Self::get_package_key(&lib.name);
                    let full_str = full_lib_path.to_string_lossy().to_string();

                    if !cp_map.contains_key(&pkg_key) {
                        cp_order.push(pkg_key.clone());
                    }
                    cp_map.insert(pkg_key, full_str);
                }
            }

            // 1.2 处理 1.18 及以前通过 natives 映射表 + classifiers 指定的平台原生库
            if let Some((_classifier_key, native_artifact)) =
                Self::select_native_classifier(lib, target_os, target_arch)
            {
                if let Some(ref path_str) = native_artifact.path {
                    let native_rel_path = Self::artifact_path_to_pathbuf(path_str);
                    let full_native_jar = libraries_dir.join(native_rel_path);
                    if full_native_jar.is_file() {
                        let excludes = lib.extract.as_ref().and_then(|e| e.exclude.as_deref());
                        let _ = Self::extract_native_jar(&full_native_jar, &natives_dir, excludes);
                    }
                }
            }
        }

        let mut cp_entries: Vec<String> = cp_order
            .into_iter()
            .filter_map(|key| cp_map.remove(&key))
            .collect();

        // 添加 client.jar（若子版本自身无 jar 且继承自父版本，则回退查找父版本 jar）
        let local_jar = version_dir.join(format!("{}.jar", version_id));
        if local_jar.is_file() {
            cp_entries.push(local_jar.to_string_lossy().to_string());
        } else if let Some(ref parent_id) = ver_json.inherits_from {
            let parent_jar = game_dir
                .join("versions")
                .join(parent_id)
                .join(format!("{}.jar", parent_id));
            if parent_jar.is_file() {
                cp_entries.push(parent_jar.to_string_lossy().to_string());
            }
        }

        let sep = platform::classpath_separator().to_string();
        let classpath = cp_entries.join(&sep);

        // 2. 准备变量映射表（按当前平台实值替换占位符）
        let effective_game_dir = crate::version::resolve_game_dir(game_dir, &version_dir);
        let assets_dir = game_dir.join("assets");
        let assets_index_name = ver_json
            .asset_index
            .as_ref()
            .map(|a| a.id.clone())
            .or_else(|| ver_json.assets.clone())
            .unwrap_or_else(|| "legacy".to_string());

        let auth_uuid = uuid.unwrap_or("55a5228ca5d74e4fb3bd99aa10d8cd98");
        let token = access_token.unwrap_or("0");
        let user_type = if access_token.is_some() { "msa" } else { "legacy" };

        let mut vars = HashMap::new();
        vars.insert("auth_player_name", player_name.to_string());
        vars.insert("version_name", version_id.to_string());
        vars.insert("game_directory", effective_game_dir.to_string_lossy().to_string());
        vars.insert("assets_root", assets_dir.to_string_lossy().to_string());
        vars.insert("game_assets", assets_dir.to_string_lossy().to_string());
        vars.insert("assets_index_name", assets_index_name);
        vars.insert("auth_uuid", auth_uuid.to_string());
        vars.insert("auth_access_token", token.to_string());
        vars.insert("user_type", user_type.to_string());
        vars.insert("version_type", "PCL".to_string());
        vars.insert("natives_directory", natives_dir.to_string_lossy().to_string());
        vars.insert("library_directory", libraries_dir.to_string_lossy().to_string());
        vars.insert("classpath_separator", sep);
        vars.insert("launcher_name", "PCL".to_string());
        vars.insert("launcher_version", "408".to_string());
        vars.insert("classpath", classpath);
        vars.insert("arch", target_arch.to_string());
        vars.insert("resolution_width", cfg.window_width.to_string());
        vars.insert("resolution_height", cfg.window_height.to_string());
        vars.insert("clientid", "${clientid}".to_string());
        vars.insert("auth_xuid", "${auth_xuid}".to_string());

        // 3. 构造 JVM 参数（保留版本 JSON 规则选出的平台专属参数如 -XstartOnFirstThread，不覆盖 -Dos.name）
        let mut jvm_args = Vec::new();
        jvm_args.push(format!("-Xmx{}m", max_memory_mb));
        jvm_args.push("-XX:+UnlockExperimentalVMOptions".to_string());
        jvm_args.push("-XX:+UseG1GC".to_string());
        jvm_args.push("-XX:-OmitStackTraceInFastThrow".to_string());
        jvm_args.push("-Djdk.lang.Process.allowAmbiguousCommands=True".to_string());
        jvm_args.push("-Dstdout.encoding=UTF-8".to_string());
        jvm_args.push("-Dstderr.encoding=UTF-8".to_string());

        if let Some(args) = &ver_json.arguments {
            if let Some(jvms) = &args.jvm {
                let resolved = Self::resolve_arguments(
                    jvms,
                    target_os,
                    target_arch,
                    None,
                    &active_features,
                    &vars,
                );
                jvm_args.extend(resolved);
            }
        } else if target_os == "osx" {
            // 对于没有 arguments.jvm 字段的旧版（1.12.2 及以前），macOS 同样需要主线程运行 GLFW
            jvm_args.push("-XstartOnFirstThread".to_string());
        }

        if !jvm_args.iter().any(|a| a.contains("java.library.path")) {
            jvm_args.push(format!(
                "-Djava.library.path={}",
                natives_dir.to_string_lossy()
            ));
        }
        if !jvm_args.iter().any(|a| a == "-cp" || a == "-classpath") {
            jvm_args.push("-cp".to_string());
            jvm_args.push(vars.get("classpath").cloned().unwrap_or_default());
        }

        // 4. 构造 Game 参数
        let mut game_args = Vec::new();
        if let Some(args) = &ver_json.arguments {
            if let Some(games) = &args.game {
                let resolved = Self::resolve_arguments(
                    games,
                    target_os,
                    target_arch,
                    None,
                    &active_features,
                    &vars,
                );
                game_args.extend(resolved);
            }
        } else if let Some(mc_args) = &ver_json.minecraft_arguments {
            for token in mc_args.split_whitespace() {
                game_args.push(Self::replace_vars(token, &vars));
            }
            if cfg.window_width > 0 && cfg.window_height > 0 {
                game_args.push("--width".to_string());
                game_args.push(cfg.window_width.to_string());
                game_args.push("--height".to_string());
                game_args.push(cfg.window_height.to_string());
            }
        }

        // 5. 启动实际进程
        emit_progress("正在合成启动参数并唤醒 Java 进程...", 85.0, None, false, None);

        let mut cmd = Command::new(&effective_java_str);
        cmd.current_dir(&effective_game_dir);
        cmd.args(&jvm_args);
        cmd.arg(&ver_json.main_class);
        cmd.args(&game_args);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        match cmd.spawn() {
            Ok(mut child) => {
                let pid = child.id();
                let g_dir = game_dir.to_path_buf();
                let v_dir = version_dir.clone();
                let reporter_handle = reporter.clone();

                emit_progress(
                    &format!("游戏已成功启动 (PID: {})，正在等待游戏窗口...", pid),
                    100.0,
                    Some(pid),
                    true,
                    None,
                );

                // 启动后台监控线程，持续读取日志流并在退出时智能分析崩溃
                std::thread::spawn(move || {
                    use std::io::{BufRead, BufReader};
                    use std::sync::{Arc, Mutex};

                    let log_buffer: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

                    if let Some(stdout) = child.stdout.take() {
                        let buf_clone = log_buffer.clone();
                        std::thread::spawn(move || {
                            let reader = BufReader::new(stdout);
                            for line in reader.lines().flatten() {
                                let mut buf = buf_clone.lock().unwrap();
                                if buf.len() > 100 {
                                    buf.remove(0);
                                }
                                buf.push(line);
                            }
                        });
                    }

                    if let Some(stderr) = child.stderr.take() {
                        let buf_clone = log_buffer.clone();
                        std::thread::spawn(move || {
                            let reader = BufReader::new(stderr);
                            for line in reader.lines().flatten() {
                                let mut buf = buf_clone.lock().unwrap();
                                if buf.len() > 100 {
                                    buf.remove(0);
                                }
                                buf.push(line);
                            }
                        });
                    }

                    match child.wait() {
                        Ok(status) => {
                            let code = status.code();
                            if !status.success() {
                                let tail = {
                                    let buf = log_buffer.lock().unwrap();
                                    buf.join("\n")
                                };
                                let analysis = CrashAnalyzer::analyze(code, &g_dir, &v_dir, &tail);
                                reporter_handle.emit_crash(analysis);
                            } else {
                                reporter_handle.emit_exit(code.unwrap_or(0));
                            }
                        }
                        Err(e) => {
                            eprintln!("监控游戏进程退出失败: {}", e);
                            reporter_handle.emit_exit(-1);
                        }
                    }
                });

                Ok(format!("游戏已成功拉起！进程 PID: {}", pid))
            }
            Err(e) => Err(format!("无法启动 Java 进程: {}", e)),
        }
    }

    /// 读取版本 JSON，若存在 `inheritsFrom` 且未内联父版本支持库，则自动合并父版本 JSON
    pub fn load_version_json_with_inheritance(
        game_dir: &Path,
        version_dir: &Path,
        version_id: &str,
    ) -> Result<LaunchVersionJson, String> {
        let json_path = version_dir.join(format!("{}.json", version_id));
        let json_content = fs::read_to_string(&json_path).map_err(|e| e.to_string())?;
        let mut child: LaunchVersionJson =
            serde_json::from_str(&json_content).map_err(|e| format!("JSON 解析失败: {}", e))?;

        if let Some(ref parent_id) = child.inherits_from.clone() {
            let parent_json_path = game_dir
                .join("versions")
                .join(parent_id)
                .join(format!("{}.json", parent_id));
            if parent_json_path.is_file() {
                if let Ok(parent_content) = fs::read_to_string(&parent_json_path) {
                    if let Ok(parent) = serde_json::from_str::<LaunchVersionJson>(&parent_content) {
                        child = Self::merge_inherited_json(parent, child);
                    }
                }
            }
        }

        Ok(child)
    }

    /// 将父版本（原版）与子版本（Forge/Fabric/NeoForge 等）的 LaunchVersionJson 合并
    pub fn merge_inherited_json(
        mut parent: LaunchVersionJson,
        child: LaunchVersionJson,
    ) -> LaunchVersionJson {
        parent.id = child.id;
        if !child.main_class.is_empty() {
            parent.main_class = child.main_class;
        }
        parent.inherits_from = child.inherits_from;
        if child.asset_index.is_some() {
            parent.asset_index = child.asset_index;
        }
        if child.assets.is_some() {
            parent.assets = child.assets;
        }
        if child.minecraft_arguments.is_some() {
            parent.minecraft_arguments = child.minecraft_arguments;
        }
        if child.java_version.is_some() {
            parent.java_version = child.java_version;
        }

        // 子版本的 libraries 追加在后，以便在 get_package_key 去重时覆盖父版本的同名旧版库（如 ASM）
        parent.libraries.extend(child.libraries);

        if let Some(child_args) = child.arguments {
            let mut merged_args = parent.arguments.unwrap_or(LaunchArguments {
                game: None,
                jvm: None,
            });
            if let Some(c_jvm) = child_args.jvm {
                let mut jvm_list = merged_args.jvm.unwrap_or_default();
                jvm_list.extend(c_jvm);
                merged_args.jvm = Some(jvm_list);
            }
            if let Some(c_game) = child_args.game {
                let mut game_list = merged_args.game.unwrap_or_default();
                game_list.extend(c_game);
                merged_args.game = Some(game_list);
            }
            parent.arguments = Some(merged_args);
        }

        parent
    }

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

    /// 解析 JVM 或 Game 参数数组（支持纯字符串及带 rules 的条件参数对象）
    pub fn resolve_arguments(
        raw_args: &[serde_json::Value],
        target_os: &str,
        target_arch: &str,
        target_os_version: Option<&str>,
        active_features: &HashMap<String, bool>,
        vars: &HashMap<&str, String>,
    ) -> Vec<String> {
        let mut out = Vec::new();
        for item in raw_args {
            if let Some(s) = item.as_str() {
                out.push(Self::replace_vars(s, vars));
            } else if item.is_object() {
                if let Ok(cond) = serde_json::from_value::<ConditionalArgument>(item.clone()) {
                    if Self::check_rules_for_target(
                        &cond.rules,
                        target_os,
                        target_arch,
                        target_os_version,
                        active_features,
                    ) {
                        match cond.value {
                            serde_json::Value::String(s) => {
                                out.push(Self::replace_vars(&s, vars));
                            }
                            serde_json::Value::Array(arr) => {
                                for v in arr {
                                    if let Some(s) = v.as_str() {
                                        out.push(Self::replace_vars(s, vars));
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        out
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

    /// 生成去重用的 package key（不依赖平台分隔符）
    pub fn get_package_key(name: &str) -> String {
        let parts: Vec<&str> = name.split(':').collect();
        if parts.len() >= 4 {
            format!("{}:{}:{}", parts[0], parts[1], parts[3])
        } else if parts.len() >= 2 {
            format!("{}:{}", parts[0], parts[1])
        } else {
            name.to_string()
        }
    }

    /// 将 artifact.path（正斜杠相对路径）按 PathBuf::join 拆解组合（禁止字符串替换拼分隔符）
    pub fn artifact_path_to_pathbuf(path_str: &str) -> PathBuf {
        let mut pb = PathBuf::new();
        for seg in path_str.split('/').filter(|s| !s.is_empty() && *s != "..") {
            pb = pb.join(seg);
        }
        pb
    }

    /// 将 Maven 坐标转换为本地相对路径（全部通过 PathBuf::join 拼接）
    pub fn maven_to_path(name: &str) -> PathBuf {
        let parts: Vec<&str> = name.split(':').collect();
        if parts.len() < 3 {
            return PathBuf::from(name);
        }
        let mut rel_path = PathBuf::new();
        for seg in parts[0].split('.') {
            rel_path = rel_path.join(seg);
        }
        let artifact = parts[1];
        let version = parts[2];
        let classifier = if parts.len() > 3 {
            format!("-{}", parts[3])
        } else {
            String::new()
        };

        let file_name = if classifier.is_empty() {
            format!("{}-{}.jar", artifact, version)
        } else {
            format!("{}-{}{}.jar", artifact, version, classifier)
        };

        rel_path.join(artifact).join(version).join(file_name)
    }

    pub fn replace_vars(template: &str, vars: &HashMap<&str, String>) -> String {
        let mut res = template.to_string();
        for (k, v) in vars {
            let pattern = format!("${{{}}}", k);
            res = res.replace(&pattern, v);
        }
        res
    }

    /// 强制终止指定的 Minecraft 游戏进程（委托给平台层 platform.rs）
    pub fn kill_process(pid: u32) -> Result<(), String> {
        platform::kill_process(pid)
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
                        "value": "-XX:HeapDumpPath=MojangTricksIntelDriversForPerformance_minecraft.heapdump"
                    },
                    {
                        "rules": [{ "action": "allow", "os": { "arch": "x86" } }],
                        "value": "-Xss1M"
                    },
                    "-Djava.library.path=${natives_directory}",
                    "-cp",
                    "${classpath}"
                ]
            },
            "libraries": [
                {
                    "name": "com.mojang:logging:1.1.1",
                    "downloads": {
                        "artifact": { "path": "com/mojang/logging/1.1.1/logging-1.1.1.jar" }
                    }
                },
                {
                    "name": "ca.weblite:java-objc-bridge:1.1",
                    "downloads": {
                        "artifact": { "path": "ca/weblite/java-objc-bridge/1.1/java-objc-bridge-1.1.jar" }
                    },
                    "rules": [
                        { "action": "allow", "os": { "name": "osx" } }
                    ]
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
                    "name": "org.lwjgl:lwjgl-glfw:3.3.2:natives-macos-arm64",
                    "downloads": {
                        "artifact": { "path": "org/lwjgl/lwjgl-glfw/3.3.2/lwjgl-glfw-3.3.2-natives-macos-arm64.jar" }
                    },
                    "rules": [
                        { "action": "allow", "os": { "name": "osx", "arch": "arm64" } }
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
                    "name": "org.lwjgl:lwjgl-glfw:3.3.2:natives-linux",
                    "downloads": {
                        "artifact": { "path": "org/lwjgl/lwjgl-glfw/3.3.2/lwjgl-glfw-3.3.2-natives-linux.jar" }
                    },
                    "rules": [
                        { "action": "allow", "os": { "name": "linux", "arch": "x64" } }
                    ]
                },
                {
                    "name": "net.java.dev.jna:jna:5.13.0",
                    "downloads": {
                        "artifact": { "path": "net/java/dev/jna/jna/5.13.0/jna-5.13.0.jar" }
                    },
                    "rules": [
                        { "action": "allow" },
                        { "action": "disallow", "os": { "name": "osx", "arch": "x86" } }
                    ]
                },
                {
                    "name": "org.lwjgl:lwjgl-legacy:2.9.4-nightly",
                    "downloads": {
                        "artifact": { "path": "org/lwjgl/lwjgl-legacy/2.9.4-nightly/lwjgl-legacy-2.9.4-nightly.jar" },
                        "classifiers": {
                            "natives-windows-64": { "path": "org/lwjgl/lwjgl-legacy/2.9.4-nightly/lwjgl-legacy-2.9.4-nightly-natives-windows-64.jar" },
                            "natives-osx": { "path": "org/lwjgl/lwjgl-legacy/2.9.4-nightly/lwjgl-legacy-2.9.4-nightly-natives-osx.jar" },
                            "natives-linux": { "path": "org/lwjgl/lwjgl-legacy/2.9.4-nightly/lwjgl-legacy-2.9.4-nightly-natives-linux.jar" }
                        }
                    },
                    "natives": {
                        "windows": "natives-windows-${arch}",
                        "osx": "natives-osx",
                        "linux": "natives-linux"
                    }
                }
            ]
        }"#;
        serde_json::from_str(raw).expect("valid sample version json")
    }

    fn collect_for_target(os: &str, arch: &str) -> (Vec<String>, Vec<String>, Vec<String>) {
        let ver = sample_version_json();
        let mut features = HashMap::new();
        features.insert("is_demo_user".to_string(), false);
        features.insert("has_custom_resolution".to_string(), true);

        let mut selected_libs = Vec::new();
        let mut selected_classifiers = Vec::new();

        for lib in &ver.libraries {
            if MinecraftLauncher::check_rules_for_target(&lib.rules, os, arch, None, &features) {
                selected_libs.push(lib.name.clone());
                if let Some((cls_key, _art)) =
                    MinecraftLauncher::select_native_classifier(lib, os, arch)
                {
                    selected_classifiers.push(cls_key);
                }
            }
        }

        let mut vars = HashMap::new();
        vars.insert("natives_directory", "/tmp/natives".to_string());
        vars.insert("classpath", "/tmp/cp.jar".to_string());
        vars.insert("auth_player_name", "Alex".to_string());
        vars.insert("resolution_width", "854".to_string());
        vars.insert("resolution_height", "480".to_string());

        let jvm_args = MinecraftLauncher::resolve_arguments(
            ver.arguments.as_ref().unwrap().jvm.as_ref().unwrap(),
            os,
            arch,
            None,
            &features,
            &vars,
        );

        (selected_libs, selected_classifiers, jvm_args)
    }

    #[test]
    fn test_rules_and_classifiers_across_four_platform_vectors() {
        // 1. (windows, x64) 向量
        let (win_libs, win_cls, win_jvm) = collect_for_target("windows", "x64");
        assert_eq!(
            win_libs,
            vec![
                "com.mojang:logging:1.1.1",
                "org.lwjgl:lwjgl-glfw:3.3.2:natives-windows",
                "net.java.dev.jna:jna:5.13.0",
                "org.lwjgl:lwjgl-legacy:2.9.4-nightly",
            ]
        );
        assert_eq!(win_cls, vec!["natives-windows-64"]);
        assert!(!win_jvm.contains(&"-XstartOnFirstThread".to_string()));
        assert!(win_jvm.iter().any(|a| a.starts_with("-XX:HeapDumpPath=")));

        // 2. (osx, arm64) 向量 —— Apple Silicon Mac
        let (mac_arm_libs, mac_arm_cls, mac_arm_jvm) = collect_for_target("osx", "arm64");
        assert_eq!(
            mac_arm_libs,
            vec![
                "com.mojang:logging:1.1.1",
                "ca.weblite:java-objc-bridge:1.1",
                "org.lwjgl:lwjgl-glfw:3.3.2:natives-macos-arm64",
                "net.java.dev.jna:jna:5.13.0",
                "org.lwjgl:lwjgl-legacy:2.9.4-nightly",
            ]
        );
        assert_eq!(mac_arm_cls, vec!["natives-osx"]);
        assert!(mac_arm_jvm.contains(&"-XstartOnFirstThread".to_string()));
        assert!(!mac_arm_jvm.iter().any(|a| a.starts_with("-XX:HeapDumpPath=")));

        // 3. (osx, x64) 向量 —— Intel Mac
        let (mac_x64_libs, mac_x64_cls, mac_x64_jvm) = collect_for_target("osx", "x64");
        assert_eq!(
            mac_x64_libs,
            vec![
                "com.mojang:logging:1.1.1",
                "ca.weblite:java-objc-bridge:1.1",
                "org.lwjgl:lwjgl-glfw:3.3.2:natives-macos",
                "net.java.dev.jna:jna:5.13.0",
                "org.lwjgl:lwjgl-legacy:2.9.4-nightly",
            ]
        );
        assert_eq!(mac_x64_cls, vec!["natives-osx"]);
        assert!(mac_x64_jvm.contains(&"-XstartOnFirstThread".to_string()));

        // 4. (linux, x64) 向量
        let (linux_libs, linux_cls, linux_jvm) = collect_for_target("linux", "x64");
        assert_eq!(
            linux_libs,
            vec![
                "com.mojang:logging:1.1.1",
                "org.lwjgl:lwjgl-glfw:3.3.2:natives-linux",
                "net.java.dev.jna:jna:5.13.0",
                "org.lwjgl:lwjgl-legacy:2.9.4-nightly",
            ]
        );
        assert_eq!(linux_cls, vec!["natives-linux"]);
        assert!(!linux_jvm.contains(&"-XstartOnFirstThread".to_string()));
        assert!(!linux_jvm.iter().any(|a| a.starts_with("-XX:HeapDumpPath=")));
    }

    #[test]
    fn test_last_matching_rule_wins_and_os_version_regex() {
        let rules = Some(vec![
            LaunchRule {
                action: "allow".to_string(),
                os: None,
                features: None,
            },
            LaunchRule {
                action: "disallow".to_string(),
                os: Some(LaunchOsRule {
                    name: Some("osx".to_string()),
                    arch: None,
                    version: Some("^10\\.5\\.\\d+$".to_string()),
                }),
                features: None,
            },
        ]);

        let feats = HashMap::new();
        assert!(!MinecraftLauncher::check_rules_for_target(
            &rules,
            "osx",
            "x64",
            Some("10.5.8"),
            &feats
        ));
        assert!(MinecraftLauncher::check_rules_for_target(
            &rules,
            "osx",
            "arm64",
            Some("14.2.1"),
            &feats
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
