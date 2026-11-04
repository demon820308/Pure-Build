use super::args::{build_launch_arguments, load_version_json_with_inheritance};
use super::LaunchProgress;
use crate::config::ConfigManager;
use crate::crash::CrashAnalyzer;
use crate::platform;
use crate::reporter::ReporterHandle;
use std::path::Path;
use std::process::{Command, Stdio};

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

    let ver_json = load_version_json_with_inheritance(game_dir, &version_dir, version_id)?;

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

    emit_progress(
        "正在按平台规则解析运行库与解压 Natives...",
        40.0,
        None,
        false,
        None,
    );

    let launch_args = build_launch_arguments(
        &ver_json,
        game_dir,
        version_id,
        target_os,
        target_arch,
        None,
        player_name,
        max_memory_mb,
        uuid,
        access_token,
        cfg.window_width,
        cfg.window_height,
        true,
    )?;

    // 5. 启动实际进程
    emit_progress("正在合成启动参数并唤醒 Java 进程...", 85.0, None, false, None);

    let mut cmd = Command::new(&effective_java_str);
    cmd.current_dir(&launch_args.effective_game_dir);
    cmd.args(&launch_args.jvm_args);
    cmd.arg(&launch_args.main_class);
    cmd.args(&launch_args.game_args);
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

/// 强制终止指定的 Minecraft 游戏进程（委托给平台层 platform.rs）
pub fn kill_process(pid: u32) -> Result<(), String> {
    platform::kill_process(pid)
}
