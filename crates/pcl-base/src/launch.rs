use crate::crash::CrashAnalyzer;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchProgress {
    pub step: String,
    pub percentage: f32,
    pub pid: Option<u32>,
    pub is_finished: bool,
    pub error: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct LaunchVersionJson {
    id: String,
    #[serde(rename = "mainClass")]
    main_class: String,
    arguments: Option<LaunchArguments>,
    #[serde(rename = "minecraftArguments")]
    minecraft_arguments: Option<String>,
    libraries: Vec<LaunchLibrary>,
    #[serde(rename = "inheritsFrom")]
    inherits_from: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LaunchArguments {
    game: Option<Vec<serde_json::Value>>,
    jvm: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
struct LaunchLibrary {
    name: String,
    downloads: Option<LaunchLibraryDownloads>,
    rules: Option<Vec<LaunchRule>>,
}

#[derive(Debug, Deserialize)]
struct LaunchLibraryDownloads {
    artifact: Option<LaunchArtifact>,
}

#[derive(Debug, Deserialize)]
struct LaunchArtifact {
    path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LaunchRule {
    action: String,
    os: Option<LaunchOsRule>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct LaunchOsRule {
    name: Option<String>,
    arch: Option<String>,
}

pub struct MinecraftLauncher;

impl MinecraftLauncher {
    /// 执行拉起游戏进程 (带正版/离线身份，并异步监控崩溃日志)
    pub fn launch_with_auth(
        app: AppHandle,
        game_dir_str: &str,
        version_id: &str,
        java_path_str: &str,
        player_name: &str,
        max_memory_mb: u32,
        uuid: Option<&str>,
        access_token: Option<&str>,
    ) -> Result<String, String> {
        let emit_progress = |step: &str, pct: f32, pid: Option<u32>, finished: bool, err: Option<String>| {
            let _ = app.emit(
                "launch_progress",
                LaunchProgress {
                    step: step.to_string(),
                    percentage: pct,
                    pid,
                    is_finished: finished,
                    error: err,
                },
            );
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

        let json_content = fs::read_to_string(&json_path).map_err(|e| e.to_string())?;
        let ver_json: LaunchVersionJson =
            serde_json::from_str(&json_content).map_err(|e| format!("JSON 解析失败: {}", e))?;

        // 1. 搜集 Classpath (去重：只针对重复库，区分 classifier 如 natives)
        emit_progress("正在解析运行依赖库并构建 Classpath...", 40.0, None, false, None);
        let libraries_dir = game_dir.join("libraries");
        let mut cp_map: HashMap<String, String> = HashMap::new();
        let mut cp_order: Vec<String> = Vec::new();

        for lib in &ver_json.libraries {
            if !Self::check_rules(&lib.rules) {
                continue;
            }

            let rel_path = if let Some(path) = lib
                .downloads
                .as_ref()
                .and_then(|d| d.artifact.as_ref())
                .and_then(|a| a.path.as_ref())
            {
                PathBuf::from(path.replace('/', std::path::MAIN_SEPARATOR_STR))
            } else {
                Self::maven_to_path(&lib.name)
            };

            let full_lib_path = libraries_dir.join(&rel_path);
            if full_lib_path.is_file() {
                let pkg_key = Self::get_package_key(&lib.name);
                let full_str = full_lib_path.to_string_lossy().to_string();

                if !cp_map.contains_key(&pkg_key) {
                    cp_order.push(pkg_key.clone());
                }
                cp_map.insert(pkg_key, full_str);
            }
        }

        let mut cp_entries: Vec<String> = cp_order
            .into_iter()
            .filter_map(|key| cp_map.remove(&key))
            .collect();

        // 添加 client.jar
        let local_jar = version_dir.join(format!("{}.jar", version_id));
        if local_jar.is_file() {
            cp_entries.push(local_jar.to_string_lossy().to_string());
        }

        let classpath = cp_entries.join(if cfg!(windows) { ";" } else { ":" });

        // 2. 准备变量映射表
        let natives_dir = version_dir.join(format!("{}-natives", version_id));
        let assets_dir = game_dir.join("assets");

        let auth_uuid = uuid.unwrap_or("55a5228ca5d74e4fb3bd99aa10d8cd98");
        let token = access_token.unwrap_or("0");
        let user_type = if access_token.is_some() { "msa" } else { "legacy" };

        let mut vars = HashMap::new();
        vars.insert("auth_player_name", player_name.to_string());
        vars.insert("version_name", version_id.to_string());
        vars.insert("game_directory", version_dir.to_string_lossy().to_string());
        vars.insert("assets_root", assets_dir.to_string_lossy().to_string());
        vars.insert("assets_index_name", "19".to_string());
        vars.insert("auth_uuid", auth_uuid.to_string());
        vars.insert("auth_access_token", token.to_string());
        vars.insert("user_type", user_type.to_string());
        vars.insert("version_type", "PCL".to_string());
        vars.insert("natives_directory", natives_dir.to_string_lossy().to_string());
        vars.insert("launcher_name", "PCL".to_string());
        vars.insert("launcher_version", "408".to_string());
        vars.insert("classpath", classpath);
        vars.insert("resolution_width", "854".to_string());
        vars.insert("resolution_height", "480".to_string());
        vars.insert("clientid", "${clientid}".to_string());
        vars.insert("auth_xuid", "${auth_xuid}".to_string());

        // 3. 构造 JVM 参数
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
                for j in jvms {
                    if let Some(s) = j.as_str() {
                        jvm_args.push(Self::replace_vars(s, &vars));
                    }
                }
            }
        }

        if !jvm_args.iter().any(|a| a.contains("java.library.path")) {
            jvm_args.push(format!("-Djava.library.path={}", natives_dir.to_string_lossy()));
        }
        if !jvm_args.iter().any(|a| a == "-cp" || a == "-classpath") {
            jvm_args.push("-cp".to_string());
            jvm_args.push(vars.get("classpath").cloned().unwrap_or_default());
        }

        // 4. 构造 Game 参数
        let mut game_args = Vec::new();
        if let Some(args) = &ver_json.arguments {
            if let Some(games) = &args.game {
                for g in games {
                    if let Some(s) = g.as_str() {
                        game_args.push(Self::replace_vars(s, &vars));
                    }
                }
            }
        } else if let Some(mc_args) = &ver_json.minecraft_arguments {
            for token in mc_args.split_whitespace() {
                game_args.push(Self::replace_vars(token, &vars));
            }
        }

        // 5. 启动实际进程
        emit_progress("正在合成启动参数并唤醒 Java 进程...", 85.0, None, false, None);

        let mut cmd = Command::new(java_path_str);
        cmd.current_dir(&version_dir);
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
                let app_handle = app.clone();

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

                    // 异步读取 stdout
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

                    // 异步读取 stderr
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

                    // 等待子进程退出
                    match child.wait() {
                        Ok(status) => {
                            let code = status.code();
                            // 如果退出码非 0，判定为异常退出或崩溃
                            if !status.success() {
                                let tail = {
                                    let buf = log_buffer.lock().unwrap();
                                    buf.join("\n")
                                };
                                let analysis = CrashAnalyzer::analyze(code, &g_dir, &v_dir, &tail);
                                let _ = app_handle.emit("game_crash", analysis);
                            } else {
                                let _ = app_handle.emit("game_exit_normal", ());
                            }
                        }
                        Err(e) => {
                            eprintln!("监控游戏进程退出失败: {}", e);
                        }
                    }
                });

                Ok(format!("游戏已成功拉起！进程 PID: {}", pid))
            }
            Err(e) => Err(format!("无法启动 Java 进程: {}", e)),
        }
    }

    fn check_rules(rules: &Option<Vec<LaunchRule>>) -> bool {
        if let Some(rule_list) = rules {
            let mut allow = false;
            for r in rule_list {
                let action = r.action == "allow";
                if let Some(os) = &r.os {
                    let mut os_match = true;
                    if let Some(name) = &os.name {
                        if name != "windows" {
                            os_match = false;
                        }
                    }
                    if os_match {
                        allow = action;
                    }
                } else {
                    allow = action;
                }
            }
            allow
        } else {
            true
        }
    }

    fn get_package_key(name: &str) -> String {
        let parts: Vec<&str> = name.split(':').collect();
        if parts.len() >= 4 {
            // groupId:artifactId:classifier (如 com.mojang:jtracy:natives-windows)
            format!("{}:{}:{}", parts[0], parts[1], parts[3])
        } else if parts.len() >= 2 {
            // groupId:artifactId (如 org.ow2.asm:asm)
            format!("{}:{}", parts[0], parts[1])
        } else {
            name.to_string()
        }
    }

    fn maven_to_path(name: &str) -> PathBuf {
        let parts: Vec<&str> = name.split(':').collect();
        if parts.len() < 3 {
            return PathBuf::from(name);
        }
        let group = parts[0].replace('.', std::path::MAIN_SEPARATOR_STR);
        let artifact = parts[1];
        let version = parts[2];
        let classifier = if parts.len() > 3 { format!("-{}", parts[3]) } else { "".to_string() };

        let file_name = if classifier.is_empty() {
            format!("{}-{}.jar", artifact, version)
        } else {
            format!("{}-{}{}.jar", artifact, version, classifier)
        };

        PathBuf::from(format!(
            "{}{}{}{}{}{}",
            group,
            std::path::MAIN_SEPARATOR,
            artifact,
            std::path::MAIN_SEPARATOR,
            version,
            std::path::MAIN_SEPARATOR
        ))
        .join(file_name)
    }

    fn replace_vars(template: &str, vars: &HashMap<&str, String>) -> String {
        let mut res = template.to_string();
        for (k, v) in vars {
            let pattern = format!("${{{}}}", k);
            res = res.replace(&pattern, v);
        }
        res
    }

    /// 强制终止指定的 Minecraft 游戏进程
    pub fn kill_process(pid: u32) -> Result<(), String> {
        #[cfg(target_os = "windows")]
        {
            Command::new("taskkill")
                .args(["/F", "/PID", &pid.to_string()])
                .output()
                .map_err(|e| format!("终止进程失败: {}", e))?;
        }
        #[cfg(not(target_os = "windows"))]
        {
            Command::new("kill")
                .args(["-9", &pid.to_string()])
                .output()
                .map_err(|e| format!("终止进程失败: {}", e))?;
        }
        Ok(())
    }
}
