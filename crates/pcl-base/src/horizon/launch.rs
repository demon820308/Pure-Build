use super::api::{negotiate_client_api, ClientLaunchApi};
use super::instances::{find_instance, instance_dir};
use super::prepare::{prepare_build_resources, run_prepare_check};
use super::session::build_session_bytes;
use crate::auth::UserProfile;
use crate::config::AppConfig;
use crate::crash::CrashAnalysisResult;
use crate::installer::DownloadProgress;
use crate::launch::LaunchProgress;
use crate::platform;
use crate::reporter::{ProgressReporter, ReporterHandle};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

/// Horizon 启动选项参数载荷
#[derive(Debug, Clone)]
pub struct HorizonLaunchOptions {
    pub window_width: u32,
    pub window_height: u32,
    pub fullscreen: bool,
}

/// 进程退出与崩溃判定结果（纯数据，便于测试）
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HorizonExitOutcome {
    Success,
    OptionRejected { error_message: String },
    Crashed {
        exit_code: i32,
        report_path: Option<String>,
        log_snippet: String,
    },
    Unexpected { exit_code: i32, log_snippet: String },
}

/// 按 ClientLaunchApi 协商结果组装 `horizon-client` 命令行参数
pub fn build_client_launch_args(
    api: &ClientLaunchApi,
    resources_dir: &Path,
    instance_dir: &Path,
    options: &HorizonLaunchOptions,
) -> Result<Vec<String>, String> {
    if !api.is_option_supported("--resources")
        || !api.is_option_supported("--game-dir")
        || !api.is_option_supported("--home")
        || !api.is_option_supported("--session-stdin")
    {
        return Err("当前 Horizon 构建缺少核心必须选项 (--resources, --game-dir, --home 或 --session-stdin)".to_string());
    }

    let mut args = Vec::new();
    args.push("--resources".to_string());
    args.push(resources_dir.to_string_lossy().to_string());

    args.push("--game-dir".to_string());
    args.push(instance_dir.to_string_lossy().to_string());

    args.push("--home".to_string());
    args.push(instance_dir.to_string_lossy().to_string());

    args.push("--session-stdin".to_string());

    if options.window_width > 0 && api.is_option_supported("--width") {
        args.push("--width".to_string());
        args.push(options.window_width.to_string());
    }

    if options.window_height > 0 && api.is_option_supported("--height") {
        args.push("--height".to_string());
        args.push(options.window_height.to_string());
    }

    if options.fullscreen && api.is_option_supported("--fullscreen") {
        args.push("--fullscreen".to_string());
    }

    Ok(args)
}

/// 解析进程退出状态与错误/崩溃特征
pub fn parse_client_exit_outcome(
    exit_code: i32,
    stderr_lines: &[String],
    crash_line_prefix: &str,
) -> HorizonExitOutcome {
    match exit_code {
        0 => HorizonExitOutcome::Success,
        2 => {
            let tail = stderr_lines.join("\n");
            HorizonExitOutcome::OptionRejected {
                error_message: format!(
                    "构建与启动器不兼容：Horizon 拒绝了传递的选项参数。\n{}",
                    tail.trim()
                ),
            }
        }
        1 | 101 => {
            let mut report_path = None;
            for line in stderr_lines.iter().rev() {
                let trimmed = line.trim();
                if trimmed.starts_with(crash_line_prefix) {
                    report_path = Some(
                        trimmed[crash_line_prefix.len()..]
                            .trim()
                            .to_string(),
                    );
                    break;
                }
            }
            HorizonExitOutcome::Crashed {
                exit_code,
                report_path,
                log_snippet: stderr_lines.join("\n"),
            }
        }
        code => HorizonExitOutcome::Unexpected {
            exit_code: code,
            log_snippet: stderr_lines.join("\n"),
        },
    }
}

/// 拉起原生 Horizon 游戏进程并进行安全会话管道交接与异步生命周期监控
pub fn launch_client_process(
    reporter: ReporterHandle,
    client_bin: &Path,
    args: &[String],
    instance_dir: &Path,
    session_bytes: &[u8],
    crash_line_prefix: &str,
) -> Result<u32, String> {
    if !client_bin.exists() {
        return Err(format!("找不到 horizon-client 可执行文件: {:?}", client_bin));
    }

    let mut cmd = Command::new(client_bin);
    cmd.current_dir(instance_dir);
    cmd.args(args);
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("拉起 horizon-client 进程失败: {}", e))?;

    let pid = child.id();

    // 经 stdin 写入会话字节并立即关闭管道，不落盘、不进命令行、不写日志
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(session_bytes)
            .map_err(|e| format!("向游戏进程传递会话数据失败: {}", e))?;
        let _ = stdin.flush();
        drop(stdin);
    } else {
        return Err("无法打开游戏进程的 stdin 管道".to_string());
    }

    let prefix_str = crash_line_prefix.to_string();
    let inst_dir_buf = instance_dir.to_path_buf();
    let reporter_handle = reporter.clone();

    // 后台监控线程
    std::thread::spawn(move || {
        let stderr_buffer: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

        if let Some(stderr) = child.stderr.take() {
            let buf_clone = stderr_buffer.clone();
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

        let code = match child.wait() {
            Ok(status) => status.code().unwrap_or(-1),
            Err(e) => {
                eprintln!("监控 Horizon 进程退出异常: {}", e);
                -1
            }
        };

        let captured_lines = {
            let buf = stderr_buffer.lock().unwrap();
            buf.clone()
        };

        let outcome = parse_client_exit_outcome(code, &captured_lines, &prefix_str);

        match outcome {
            HorizonExitOutcome::Success => {
                reporter_handle.emit_exit(0);
            }
            HorizonExitOutcome::OptionRejected { error_message } => {
                reporter_handle.emit_crash(CrashAnalysisResult {
                    exit_code: Some(2),
                    title: "Horizon 参数选项被拒绝".to_string(),
                    reason: "构建与启动器不兼容".to_string(),
                    suggestion: "请检查游戏版本与启动器版本是否兼容，或更新 Horizon 构建。"
                        .to_string(),
                    log_snippet: error_message,
                    report_file: None,
                });
                reporter_handle.emit_exit(2);
            }
            HorizonExitOutcome::Crashed {
                exit_code,
                report_path,
                log_snippet,
            } => {
                let resolved_report = report_path.map(|rp| {
                    let p = Path::new(&rp);
                    if p.is_absolute() {
                        rp
                    } else {
                        inst_dir_buf.join(p).to_string_lossy().to_string()
                    }
                });
                reporter_handle.emit_crash(CrashAnalysisResult {
                    exit_code: Some(exit_code),
                    title: if exit_code == 101 {
                        "Horizon 游戏运行时 Panic 崩溃".to_string()
                    } else {
                        "Horizon 游戏异常退出".to_string()
                    },
                    reason: "原生游戏进程终止".to_string(),
                    suggestion: "请查看崩溃日志获取具体堆栈。".to_string(),
                    log_snippet,
                    report_file: resolved_report,
                });
                reporter_handle.emit_exit(exit_code);
            }
            HorizonExitOutcome::Unexpected {
                exit_code,
                log_snippet,
            } => {
                reporter_handle.emit_crash(CrashAnalysisResult {
                    exit_code: Some(exit_code),
                    title: "Horizon 进程非正常退出".to_string(),
                    reason: format!("进程终止，退出码: {}", exit_code),
                    suggestion: "请查看 stderr 日志片段。".to_string(),
                    log_snippet,
                    report_file: None,
                });
                reporter_handle.emit_exit(exit_code);
            }
        }
    });

    Ok(pid)
}

/// 将资源准备阶段的下载进度同步镜像到 LaunchProgress，保证启动重备时进度在启动页可见
struct LaunchPrepareBridgeReporter(ReporterHandle);

impl ProgressReporter for LaunchPrepareBridgeReporter {
    fn emit_download(&self, progress: DownloadProgress) {
        self.0.emit_launch(LaunchProgress {
            step: format!("准备资源: {}", progress.step),
            percentage: 50.0 + (progress.percentage.clamp(0.0, 100.0) * 0.35),
            pid: None,
            is_finished: false,
            error: progress.error.clone(),
        });
        self.0.emit_download(progress);
    }

    fn emit_launch(&self, progress: LaunchProgress) {
        self.0.emit_launch(progress);
    }

    fn emit_crash(&self, analysis: CrashAnalysisResult) {
        self.0.emit_crash(analysis);
    }

    fn emit_exit(&self, exit_code: i32) {
        self.0.emit_exit(exit_code);
    }
}

/// 完整的 Horizon 实例启动编排：
/// 1. 微软正版账号门禁校验并生成内存会话 JSON（不落盘、不进日志/参数）；
/// 2. 读取实例注册表与构建协商结果（协商失败直接报不兼容/损坏，不硬启）；
/// 3. 启动前 `--check` 门禁，若未就绪则自动转入资源准备并再次 `--check`；
/// 4. 按协商选项表组装参数并启动原生游戏进程，上报 PID。
pub async fn launch_horizon_instance(
    reporter: ReporterHandle,
    game_dir: &Path,
    instance_name: &str,
    profile: Option<&UserProfile>,
    cfg: &AppConfig,
) -> Result<String, String> {
    let emit_progress = |step: &str, pct: f32, pid: Option<u32>, finished: bool, err: Option<String>| {
        reporter.emit_launch(LaunchProgress {
            step: step.to_string(),
            percentage: pct,
            pid,
            is_finished: finished,
            error: err,
        });
    };

    emit_progress("正在校验正版微软账号门禁...", 10.0, None, false, None);

    let active_profile = profile.ok_or_else(|| {
        let msg = "Horizon 需要正版微软账号，请先登录后再启动".to_string();
        emit_progress("启动受阻", 0.0, None, true, Some(msg.clone()));
        msg
    })?;
    let session_bytes = build_session_bytes(active_profile, None).map_err(|e| {
        emit_progress("启动受阻", 0.0, None, true, Some(e.clone()));
        e
    })?;

    let instance = find_instance(game_dir, instance_name).ok_or_else(|| {
        let msg = format!("找不到 Horizon 实例「{}」", instance_name);
        emit_progress("启动失败", 0.0, None, true, Some(msg.clone()));
        msg
    })?;

    let build_dir = game_dir
        .join("horizon")
        .join("builds")
        .join(&instance.build_tag);
    let client_bin = build_dir.join(platform::horizon_client_bin_name());
    let prepare_bin = build_dir.join(platform::horizon_prepare_bin_name());
    let resources_dir = game_dir
        .join("horizon")
        .join("resources")
        .join(&instance.build_tag);
    let inst_dir = instance_dir(game_dir, &instance.name);
    if !inst_dir.exists() {
        let _ = std::fs::create_dir_all(&inst_dir);
    }

    emit_progress("正在与 Horizon 构建协商 Launch API...", 25.0, None, false, None);
    let client_api = negotiate_client_api(&client_bin).map_err(|e| {
        let msg = format!("构建不可启动（协商失败或版本不兼容）: {}", e);
        emit_progress("启动失败", 0.0, None, true, Some(msg.clone()));
        msg
    })?;

    emit_progress("正在检查 Horizon 游戏资源完整性...", 40.0, None, false, None);
    let pre_check_ready = if resources_dir.exists() {
        let chk = run_prepare_check(&prepare_bin, &resources_dir).map_err(|e| {
            let msg = format!("启动前资源检查失败: {}", e);
            emit_progress("启动失败", 0.0, None, true, Some(msg.clone()));
            msg
        })?;
        chk.is_ready
    } else {
        false
    };

    if !pre_check_ready {
        emit_progress("资源未就绪，正在自动准备 Horizon 资源...", 50.0, None, false, None);
        let java_cand = cfg.java_path.as_deref().unwrap_or("");
        let bridge_reporter: ReporterHandle = Arc::new(LaunchPrepareBridgeReporter(reporter.clone()));
        prepare_build_resources(
            &bridge_reporter,
            game_dir,
            &instance.build_tag,
            java_cand,
            Some(active_profile),
        )
        .await
        .map_err(|e| {
            let msg = format!("启动前自动准备资源失败: {}", e);
            emit_progress("启动失败", 0.0, None, true, Some(msg.clone()));
            msg
        })?;

        let post_check = run_prepare_check(&prepare_bin, &resources_dir).map_err(|e| {
            emit_progress("启动失败", 0.0, None, true, Some(e.clone()));
            e
        })?;
        if !post_check.is_ready {
            let msg = format!(
                "资源完整性校验仍未通过，拒绝启动: {}",
                post_check.gaps.join("; ")
            );
            emit_progress("启动失败", 0.0, None, true, Some(msg.clone()));
            return Err(msg);
        }
    }

    emit_progress("正在启动 Horizon 原生游戏进程...", 85.0, None, false, None);

    let opts = HorizonLaunchOptions {
        window_width: cfg.window_width,
        window_height: cfg.window_height,
        fullscreen: false,
    };
    let args = build_client_launch_args(&client_api, &resources_dir, &inst_dir, &opts).map_err(|e| {
        emit_progress("启动失败", 0.0, None, true, Some(e.clone()));
        e
    })?;
    let crash_prefix = client_api
        .crash_report_line
        .as_deref()
        .unwrap_or("Horizon: crash report saved to ");

    let pid = launch_client_process(
        reporter.clone(),
        &client_bin,
        &args,
        &inst_dir,
        &session_bytes,
        crash_prefix,
    )
    .map_err(|e| {
        emit_progress("启动失败", 0.0, None, true, Some(e.clone()));
        e
    })?;

    emit_progress(
        &format!("Horizon 已启动 (PID: {})，正在等待游戏窗口...", pid),
        100.0,
        Some(pid),
        true,
        None,
    );

    Ok(format!("Horizon 已启动！进程 PID: {}", pid))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::horizon::api::LaunchOptionDescriptor;

    #[test]
    fn test_build_client_launch_args() {
        let api = ClientLaunchApi {
            launch_api: "0.1.1".to_string(),
            horizon: "0.1.1".to_string(),
            minecraft: "26.3".to_string(),
            options: vec![
                LaunchOptionDescriptor {
                    name: "--resources".to_string(),
                    value: Some("DIR".to_string()),
                },
                LaunchOptionDescriptor {
                    name: "--game-dir".to_string(),
                    value: Some("DIR".to_string()),
                },
                LaunchOptionDescriptor {
                    name: "--home".to_string(),
                    value: Some("DIR".to_string()),
                },
                LaunchOptionDescriptor {
                    name: "--session-stdin".to_string(),
                    value: None,
                },
                LaunchOptionDescriptor {
                    name: "--width".to_string(),
                    value: Some("N".to_string()),
                },
                LaunchOptionDescriptor {
                    name: "--height".to_string(),
                    value: Some("N".to_string()),
                },
                LaunchOptionDescriptor {
                    name: "--fullscreen".to_string(),
                    value: None,
                },
            ],
            exit_codes: vec![0, 1, 2, 101],
            crash_report_line: Some("Horizon: crash report saved to ".to_string()),
        };

        let resources_dir = Path::new("/path/to/res");
        let instance_dir = Path::new("/path/to/inst");
        let opts = HorizonLaunchOptions {
            window_width: 1280,
            window_height: 720,
            fullscreen: true,
        };

        let args = build_client_launch_args(&api, resources_dir, instance_dir, &opts)
            .expect("组装应成功");

        assert!(args.contains(&"--resources".to_string()));
        assert!(args.contains(&"--game-dir".to_string()));
        assert!(args.contains(&"--home".to_string()));
        assert!(args.contains(&"--session-stdin".to_string()));
        assert!(args.contains(&"--width".to_string()));
        assert!(args.contains(&"1280".to_string()));
        assert!(args.contains(&"--height".to_string()));
        assert!(args.contains(&"720".to_string()));
        assert!(args.contains(&"--fullscreen".to_string()));
    }

    #[test]
    fn test_parse_client_exit_code_mapping() {
        let prefix = "Horizon: crash report saved to ";

        // 退出码 0: 正常
        let out0 = parse_client_exit_outcome(0, &[], prefix);
        assert_eq!(out0, HorizonExitOutcome::Success);

        // 退出码 2: 选项被拒
        let out2 = parse_client_exit_outcome(2, &["Horizon: --bogus: unknown".to_string()], prefix);
        match out2 {
            HorizonExitOutcome::OptionRejected { error_message } => {
                assert!(error_message.contains("构建与启动器不兼容"));
                assert!(error_message.contains("--bogus"));
            }
            _ => panic!("应为 OptionRejected"),
        }

        // 退出码 1: 崩溃且捕获崩溃报告路径
        let stderr1 = vec![
            "Error details...".to_string(),
            "Horizon: crash report saved to /path/to/crash-2026.txt".to_string(),
        ];
        let out1 = parse_client_exit_outcome(1, &stderr1, prefix);
        assert_eq!(
            out1,
            HorizonExitOutcome::Crashed {
                exit_code: 1,
                report_path: Some("/path/to/crash-2026.txt".to_string()),
                log_snippet: stderr1.join("\n"),
            }
        );

        // 退出码 101: Panic
        let stderr101 = vec![
            "thread panicked at ...".to_string(),
            "Horizon: crash report saved to /path/to/panic-report.txt".to_string(),
        ];
        let out101 = parse_client_exit_outcome(101, &stderr101, prefix);
        assert_eq!(
            out101,
            HorizonExitOutcome::Crashed {
                exit_code: 101,
                report_path: Some("/path/to/panic-report.txt".to_string()),
                log_snippet: stderr101.join("\n"),
            }
        );
    }

    #[tokio::test]
    async fn test_launch_horizon_rejects_broken_negotiation_without_fallback() {
        let tmp = std::env::temp_dir().join(format!("pbl_launch_neg_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let build_dir = tmp.join("horizon").join("builds").join("nightly");
        std::fs::create_dir_all(&build_dir).unwrap();
        let meta = crate::horizon::build::PblBuildMeta {
            tag: "nightly".to_string(),
            package_name: "horizon-windows-x64.zip".to_string(),
            sha256: "0".repeat(64),
            horizon_version: "0.1.1".to_string(),
            minecraft_version: "26.3".to_string(),
            launch_api_version: "0.1.1".to_string(),
            java_major: 25,
            installed_at: "2026-11-16T00:00:00Z".to_string(),
        };
        std::fs::write(
            build_dir.join("pbl-build.json"),
            serde_json::to_string(&meta).unwrap(),
        )
        .unwrap();
        crate::horizon::instances::create_instance(&tmp, "BrokenInst", "nightly").unwrap();

        let reporter: ReporterHandle = Arc::new(crate::reporter::NoopProgressReporter);
        let cfg = AppConfig::default();
        let msa = UserProfile {
            name: "Alex".to_string(),
            uuid: "069a79f444e94726a5befca90e38aaf5".to_string(),
            access_token: "tok".to_string(),
            avatar_url: "".to_string(),
            auth_type: "msa".to_string(),
            auth_server: None,
        };

        // 缺少真实 horizon-client 二进制时协商必败，禁止回退按默认选项硬启
        let err = launch_horizon_instance(reporter, &tmp, "BrokenInst", Some(&msa), &cfg)
            .await
            .unwrap_err();
        assert!(err.contains("构建不可启动"));

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
