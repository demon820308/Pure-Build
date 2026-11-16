use super::api::{negotiate_prepare_api, PrepareLaunchApi};
use crate::auth::UserProfile;
use crate::download::DownloadManager;
use crate::installer::server::download_server_jar;
use crate::installer::{DownloadProgress, VersionInstaller};
use crate::java::JavaDownloader;
use crate::platform;
use crate::reporter::ReporterHandle;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use ts_rs::TS;

/// horizon-prepare --check 校验结果 DTO
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
pub struct HorizonCheckResult {
    pub is_ready: bool,
    pub gaps: Vec<String>,
    pub error: Option<String>,
}

/// horizon-prepare --progress json 单行事件
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct PrepareProgressItem {
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub fraction: Option<f32>,
    #[serde(default)]
    pub error: Option<String>,
}

/// 解析 --progress json 单行输出（纯函数，未知字段忽略）
pub fn parse_progress_line(line: &str) -> Option<PrepareProgressItem> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    serde_json::from_str::<PrepareProgressItem>(trimmed).ok()
}

/// 解析 --check 退出码与输出（纯函数，便于测试）
/// 0: 资源完整且为当前版本
/// 1: 存在缺口或待预备（stdout 逐行列出缺口）
/// 其它: 执行异常或选项被拒
pub fn parse_check_exit(
    exit_code: i32,
    stdout: &str,
    stderr: &str,
) -> Result<HorizonCheckResult, String> {
    match exit_code {
        0 => Ok(HorizonCheckResult {
            is_ready: true,
            gaps: Vec::new(),
            error: None,
        }),
        1 => {
            let gaps: Vec<String> = stdout
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .map(|l| l.to_string())
                .collect();
            let err_msg = {
                let s = stderr.trim();
                if s.is_empty() {
                    None
                } else {
                    Some(s.to_string())
                }
            };
            Ok(HorizonCheckResult {
                is_ready: false,
                gaps,
                error: err_msg,
            })
        }
        2 => Err(format!(
            "horizon-prepare --check 命令选项被拒绝: {}",
            stderr.trim()
        )),
        code => Err(format!(
            "horizon-prepare --check 执行异常 (退出码 {}): {}",
            code,
            stderr.trim()
        )),
    }
}

/// 运行 `horizon-prepare --check --output <output_dir>` 校验资源完整性
pub fn run_prepare_check(prepare_bin: &Path, output_dir: &Path) -> Result<HorizonCheckResult, String> {
    if !prepare_bin.exists() {
        return Err(format!("找不到 horizon-prepare 可执行文件: {:?}", prepare_bin));
    }
    let output = Command::new(prepare_bin)
        .arg("--check")
        .arg("--output")
        .arg(output_dir)
        .output()
        .map_err(|e| format!("执行 horizon-prepare --check 失败: {}", e))?;

    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    parse_check_exit(code, &stdout, &stderr)
}

/// 检查指定构建标签的资源准备状态
pub fn check_build_resources(game_dir: &Path, tag: &str) -> Result<HorizonCheckResult, String> {
    let build_dir = game_dir.join("horizon").join("builds").join(tag);
    let prepare_bin = build_dir.join(platform::horizon_prepare_bin_name());
    let resources_dir = game_dir.join("horizon").join("resources").join(tag);
    if !resources_dir.exists() {
        return Ok(HorizonCheckResult {
            is_ready: false,
            gaps: vec!["资源尚未准备".to_string()],
            error: None,
        });
    }
    run_prepare_check(&prepare_bin, &resources_dir)
}

/// 准备指认的输入文件清单
#[derive(Debug, Clone, Default)]
pub struct PrepareInputs {
    pub version_json: Option<PathBuf>,
    pub client_jar: Option<PathBuf>,
    pub libraries_dir: Option<PathBuf>,
    pub assets_dir: Option<PathBuf>,
    pub server_jar: Option<PathBuf>,
    pub java_home: Option<PathBuf>,
}

/// 按协商的可用选项列表安全组装 `horizon-prepare` 命令行参数
pub fn build_prepare_args(
    api: &PrepareLaunchApi,
    output_dir: &Path,
    inputs: &PrepareInputs,
) -> Result<Vec<String>, String> {
    let mut args = Vec::new();

    if !api.is_option_supported("--prepare") || !api.is_option_supported("--output") {
        return Err("当前构建的 horizon-prepare 不支持 --prepare 或 --output 选项".to_string());
    }

    args.push("--prepare".to_string());
    args.push("--output".to_string());
    args.push(output_dir.to_string_lossy().to_string());

    if api.is_option_supported("--complete") {
        args.push("--complete".to_string());
    }

    if api.is_option_supported("--progress") {
        args.push("--progress".to_string());
        args.push("json".to_string());
    }

    if let Some(vj) = &inputs.version_json {
        if api.is_option_supported("--version-json") {
            args.push("--version-json".to_string());
            args.push(vj.to_string_lossy().to_string());
        }
    }

    if let Some(cj) = &inputs.client_jar {
        if api.is_option_supported("--client-jar") {
            args.push("--client-jar".to_string());
            args.push(cj.to_string_lossy().to_string());
        }
    }

    if let Some(ld) = &inputs.libraries_dir {
        if api.is_option_supported("--libraries-dir") {
            args.push("--libraries-dir".to_string());
            args.push(ld.to_string_lossy().to_string());
        }
    }

    if let Some(ad) = &inputs.assets_dir {
        if api.is_option_supported("--assets-dir") {
            args.push("--assets-dir".to_string());
            args.push(ad.to_string_lossy().to_string());
        }
    }

    if let Some(sj) = &inputs.server_jar {
        if api.is_option_supported("--server-jar") {
            args.push("--server-jar".to_string());
            args.push(sj.to_string_lossy().to_string());
        }
    }

    if let Some(jh) = &inputs.java_home {
        if api.is_option_supported("--java-home") {
            args.push("--java-home".to_string());
            args.push(jh.to_string_lossy().to_string());
        }
    }

    Ok(args)
}

/// 执行资源准备管线，解析逐行 JSON 并向前端投递进度事件
pub fn execute_prepare(
    reporter: &ReporterHandle,
    prepare_bin: &Path,
    args: &[String],
    tag: &str,
) -> Result<(), String> {
    let mut cmd = Command::new(prepare_bin);
    cmd.args(args);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("启动 horizon-prepare 进程失败: {}", e))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "未能捕获 horizon-prepare stdout 流".to_string())?;

    let reader = BufReader::new(stdout);
    let mut last_message = "正在准备 Horizon 游戏资源...".to_string();
    let mut last_fraction = 0.0f32;
    let mut failure_reason = None;

    for line_res in reader.lines() {
        let line = match line_res {
            Ok(l) => l,
            Err(_) => continue,
        };

        if let Some(item) = parse_progress_line(&line) {
            if let Some(msg) = item.message {
                last_message = msg;
            }
            if let Some(frac) = item.fraction {
                last_fraction = (frac * 100.0).clamp(0.0, 100.0);
            }
            if let Some(err) = item.error {
                failure_reason = Some(err);
            }

            reporter.emit_download(DownloadProgress {
                version_id: format!("horizon-{}", tag),
                vanilla_version: None,
                step: last_message.clone(),
                percentage: last_fraction,
                speed_text: "".to_string(),
                is_finished: false,
                error: failure_reason.clone(),
            });
        }
    }

    let status = child
        .wait()
        .map_err(|e| format!("等待 horizon-prepare 进程退出失败: {}", e))?;

    if status.success() && failure_reason.is_none() {
        reporter.emit_download(DownloadProgress {
            version_id: format!("horizon-{}", tag),
            vanilla_version: None,
            step: "Horizon 资源准备完毕".to_string(),
            percentage: 100.0,
            speed_text: "".to_string(),
            is_finished: true,
            error: None,
        });
        Ok(())
    } else {
        let err_text = failure_reason.unwrap_or_else(|| {
            format!(
                "horizon-prepare 异常退出 (退出码: {:?})",
                status.code()
            )
        });
        reporter.emit_download(DownloadProgress {
            version_id: format!("horizon-{}", tag),
            vanilla_version: None,
            step: "资源准备失败".to_string(),
            percentage: last_fraction,
            speed_text: "".to_string(),
            is_finished: true,
            error: Some(err_text.clone()),
        });
        Err(err_text)
    }
}

fn require_msa_for_mojang_download(profile: Option<&UserProfile>) -> Result<(), String> {
    match profile {
        Some(p) if p.auth_type == "msa" && !p.access_token.trim().is_empty() => Ok(()),
        _ => Err(
            "准备资源需要下载正版 Minecraft 文件（客户端/服务端），请先使用拥有 Java 版的微软账号登录"
                .to_string(),
        ),
    }
}

/// 完整的 Horizon 构建资源准备管线：
/// 1. 协商 `horizon-prepare --launch-api` 获知所需的 minecraft 钉子与 java_major；
/// 2. 跑前 `--check` 门禁，若已完整直接返回；
/// 3. 优先指认 PBL 已管理的 versions/<mc>/、libraries/、assets/，缺失原版文件时走正版门禁并自动安装；
/// 4. 首次准备且未下载 `server.jar` 时走正版门禁下载至 `<build_dir>/server.jar`；
/// 5. 确保匹配 `java_major` 的 Java 环境并传入 `--java-home`；
/// 6. 运行 `horizon-prepare --prepare` 并实时上报进度，结束后跑后置 `--check` 门禁。
pub async fn prepare_build_resources(
    reporter: &ReporterHandle,
    game_dir: &Path,
    tag: &str,
    java_candidate: &str,
    profile: Option<&UserProfile>,
) -> Result<(), String> {
    let build_dir = game_dir.join("horizon").join("builds").join(tag);
    let prepare_bin = build_dir.join(platform::horizon_prepare_bin_name());
    let resources_dir = game_dir.join("horizon").join("resources").join(tag);

    let api = negotiate_prepare_api(&prepare_bin)?;

    if resources_dir.exists() {
        if let Ok(check) = run_prepare_check(&prepare_bin, &resources_dir) {
            if check.is_ready {
                reporter.emit_download(DownloadProgress {
                    version_id: format!("horizon-{}", tag),
                    vanilla_version: Some(api.minecraft.clone()),
                    step: "Horizon 资源已完整就绪".to_string(),
                    percentage: 100.0,
                    speed_text: "".to_string(),
                    is_finished: true,
                    error: None,
                });
                return Ok(());
            }
        }
    }

    let mc_ver = &api.minecraft;
    let version_dir = game_dir.join("versions").join(mc_ver);
    let version_json = version_dir.join(format!("{}.json", mc_ver));
    let client_jar = version_dir.join(format!("{}.jar", mc_ver));
    let libraries_dir = game_dir.join("libraries");
    let assets_dir = game_dir.join("assets");
    let server_jar = build_dir.join("server.jar");

    let need_vanilla = !version_json.is_file() || !client_jar.is_file();
    let need_server = !server_jar.is_file() && api.is_option_supported("--server-jar");

    let mut cached_meta_url: Option<String> = None;
    if need_vanilla || need_server {
        require_msa_for_mojang_download(profile)?;
        let manifest = DownloadManager::fetch_version_manifest().await?;
        let remote_ver = manifest
            .into_iter()
            .find(|v| v.id == *mc_ver)
            .ok_or_else(|| format!("Mojang 版本清单中找不到构建所需的 Minecraft 版本: {}", mc_ver))?;
        cached_meta_url = Some(remote_ver.url);
    }

    if need_vanilla {
        let meta_url = cached_meta_url.clone().unwrap();
        VersionInstaller::install_version(
            reporter.clone(),
            game_dir.to_string_lossy().to_string(),
            mc_ver.clone(),
            None,
            meta_url,
            None,
            None,
        )
        .await?;
    }

    if !version_json.is_file() {
        return Err(format!("准备资源缺少版本元数据文件: {:?}", version_json));
    }
    if !client_jar.is_file() {
        return Err(format!("准备资源缺少客户端核心文件: {:?}", client_jar));
    }

    if need_server {
        let meta_url = cached_meta_url.unwrap();
        download_server_jar(
            reporter,
            &format!("horizon-{}", tag),
            mc_ver,
            &meta_url,
            &server_jar,
        )
        .await?;
    }

    let java_exe =
        JavaDownloader::ensure_java_for_version(reporter, game_dir, java_candidate, api.java_major)
            .await?;
    let java_home = java_exe
        .parent()
        .and_then(|bin_dir| bin_dir.parent())
        .map(|home| home.to_path_buf());

    let inputs = PrepareInputs {
        version_json: Some(version_json),
        client_jar: Some(client_jar),
        libraries_dir: if libraries_dir.is_dir() {
            Some(libraries_dir)
        } else {
            None
        },
        assets_dir: if assets_dir.is_dir() {
            Some(assets_dir)
        } else {
            None
        },
        server_jar: if server_jar.is_file() {
            Some(server_jar)
        } else {
            None
        },
        java_home,
    };

    let args = build_prepare_args(&api, &resources_dir, &inputs)?;
    let reporter_clone = reporter.clone();
    let bin_clone = prepare_bin.clone();
    let tag_owned = tag.to_string();

    tokio::task::spawn_blocking(move || {
        execute_prepare(&reporter_clone, &bin_clone, &args, &tag_owned)
    })
    .await
    .map_err(|e| format!("资源准备线程异常: {}", e))??;

    // 跑后 --check 门禁验证
    let post_check = run_prepare_check(&prepare_bin, &resources_dir)?;
    if !post_check.is_ready {
        return Err(format!(
            "资源准备完成后完整性校验仍未通过: {}",
            post_check.gaps.join("; ")
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::horizon::api::LaunchOptionDescriptor;

    #[test]
    fn test_parse_progress_line() {
        let line_msg = r#"{"message": "Extracting textures"}"#;
        let item1 = parse_progress_line(line_msg).unwrap();
        assert_eq!(item1.message.as_deref(), Some("Extracting textures"));
        assert_eq!(item1.fraction, None);

        let line_frac = r#"{"fraction": 0.42}"#;
        let item2 = parse_progress_line(line_frac).unwrap();
        assert_eq!(item2.fraction, Some(0.42));

        let line_err = r#"{"error": "Missing research-lock.json"}"#;
        let item3 = parse_progress_line(line_err).unwrap();
        assert_eq!(item3.error.as_deref(), Some("Missing research-lock.json"));

        let line_unknown = r#"{"unknown": 123}"#;
        let item4 = parse_progress_line(line_unknown).unwrap();
        assert_eq!(item4.message, None);
    }

    #[test]
    fn test_parse_check_exit() {
        let ok = parse_check_exit(0, "", "").unwrap();
        assert!(ok.is_ready);
        assert!(ok.gaps.is_empty());

        let gaps = parse_check_exit(1, "missing sounds\nmissing textures\n", "").unwrap();
        assert!(!gaps.is_ready);
        assert_eq!(gaps.gaps, vec!["missing sounds", "missing textures"]);

        assert!(parse_check_exit(2, "", "unknown option").is_err());
    }

    #[test]
    fn test_build_prepare_args_exact_matching() {
        let api = PrepareLaunchApi {
            launch_api: "0.1.1".to_string(),
            horizon: "0.1.1".to_string(),
            minecraft: "26.3".to_string(),
            java_major: 25,
            output_version: Some(1),
            schema: Some(1),
            options: vec![
                LaunchOptionDescriptor {
                    name: "--prepare".to_string(),
                    value: None,
                },
                LaunchOptionDescriptor {
                    name: "--output".to_string(),
                    value: Some("DIR".to_string()),
                },
                LaunchOptionDescriptor {
                    name: "--complete".to_string(),
                    value: None,
                },
                LaunchOptionDescriptor {
                    name: "--progress".to_string(),
                    value: Some("json".to_string()),
                },
                LaunchOptionDescriptor {
                    name: "--version-json".to_string(),
                    value: Some("PATH".to_string()),
                },
                LaunchOptionDescriptor {
                    name: "--client-jar".to_string(),
                    value: Some("PATH".to_string()),
                },
                LaunchOptionDescriptor {
                    name: "--libraries-dir".to_string(),
                    value: Some("DIR".to_string()),
                },
                LaunchOptionDescriptor {
                    name: "--assets-dir".to_string(),
                    value: Some("DIR".to_string()),
                },
                LaunchOptionDescriptor {
                    name: "--java-home".to_string(),
                    value: Some("PATH".to_string()),
                },
            ],
            exit_codes: vec![0, 1, 2],
        };

        let output_dir = Path::new("/path/to/prepared");
        let inputs = PrepareInputs {
            version_json: Some(PathBuf::from("/mc/26.3.json")),
            client_jar: Some(PathBuf::from("/mc/26.3.jar")),
            libraries_dir: Some(PathBuf::from("/mc/libraries")),
            assets_dir: Some(PathBuf::from("/mc/assets")),
            server_jar: Some(PathBuf::from("/mc/server.jar")),
            java_home: Some(PathBuf::from("/java/25")),
        };

        let args = build_prepare_args(&api, output_dir, &inputs).expect("组装应成功");
        assert!(args.contains(&"--prepare".to_string()));
        assert!(args.contains(&"--complete".to_string()));
        assert!(args.contains(&"--progress".to_string()));
        assert!(args.contains(&"json".to_string()));
        assert!(args.contains(&"--version-json".to_string()));
        assert!(args.contains(&"--client-jar".to_string()));
        assert!(args.contains(&"--libraries-dir".to_string()));
        assert!(args.contains(&"--assets-dir".to_string()));
        assert!(args.contains(&"--java-home".to_string()));
        assert!(!args.contains(&"--server-jar".to_string()));
    }
}
