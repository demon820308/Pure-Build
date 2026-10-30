use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrashAnalysisResult {
    pub exit_code: Option<i32>,
    pub title: String,
    pub reason: String,
    pub suggestion: String,
    pub log_snippet: String,
    pub report_file: Option<String>,
}

pub struct CrashAnalyzer;

impl CrashAnalyzer {
    /// 分析游戏崩溃（结合退出代码、控制台尾部日志、latest.log 与 crash-reports）
    pub fn analyze(
        exit_code: Option<i32>,
        game_dir: &Path,
        version_dir: &Path,
        console_tail: &str,
    ) -> CrashAnalysisResult {
        // 1. 寻找最新的 crash-reports 文件
        let mut report_content = String::new();
        let mut report_file_path = None;

        let crash_dirs = vec![
            version_dir.join("crash-reports"),
            game_dir.join("crash-reports"),
        ];

        for c_dir in crash_dirs {
            if let Ok(entries) = fs::read_dir(c_dir) {
                let mut latest_file: Option<(PathBuf, std::time::SystemTime)> = None;
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("txt") {
                        if let Ok(meta) = entry.metadata() {
                            if let Ok(modified) = meta.modified() {
                                if latest_file.as_ref().map_or(true, |(_, t)| modified > *t) {
                                    latest_file = Some((path, modified));
                                }
                            }
                        }
                    }
                }

                if let Some((path, _)) = latest_file {
                    if let Ok(content) = fs::read_to_string(&path) {
                        report_file_path = Some(path.to_string_lossy().to_string());
                        report_content = content;
                        break;
                    }
                }
            }
        }

        // 2. 读取 logs/latest.log
        let mut latest_log = String::new();
        let log_candidates = vec![
            version_dir.join("logs").join("latest.log"),
            game_dir.join("logs").join("latest.log"),
        ];
        for l_path in log_candidates {
            if l_path.is_file() {
                if let Ok(content) = fs::read_to_string(l_path) {
                    latest_log = content;
                    break;
                }
            }
        }

        // 合并所有日志源用于规则匹配
        let full_text = format!("{}\n{}\n{}", console_tail, latest_log, report_content);

        // 提取用于展示的最后 35 行
        let mut snippet_lines: Vec<&str> = full_text
            .lines()
            .filter(|l| !l.trim().is_empty())
            .collect();
        if snippet_lines.len() > 35 {
            snippet_lines = snippet_lines[snippet_lines.len() - 35..].to_vec();
        }
        let log_snippet = snippet_lines.join("\n");

        // 3. 经典诊断规则匹配 (对标 PCL2 ModCrash 规则库)
        if full_text.contains("java.lang.OutOfMemoryError")
            || full_text.contains("an out of memory error")
            || full_text.contains("Could not reserve enough space for")
        {
            return CrashAnalysisResult {
                exit_code,
                title: "内存不足 (Out Of Memory)".to_string(),
                reason: "游戏在运行过程中消耗了所有分配的内存，触发了 Java 虚拟机内存溢出。".to_string(),
                suggestion: "请前往启动器【设置】页面，将【最大内存分配】调大（推荐 4096MB 或 8192MB），或排查是否有过多高清材质包/大型模组。".to_string(),
                log_snippet,
                report_file: report_file_path,
            };
        }

        if full_text.contains("The driver does not appear to support OpenGL")
            || full_text.contains("GLFW error 65542: WGL")
            || full_text.contains("Couldn't set pixel format")
        {
            return CrashAnalysisResult {
                exit_code,
                title: "显卡不支持或 OpenGL 驱动缺失".to_string(),
                reason: "Minecraft 无法创建 OpenGL 渲染上下文，通常是由于显卡驱动未正确安装或版本过旧。".to_string(),
                suggestion: "请前往显卡官网（NVIDIA / AMD / Intel）下载并安装最新的显卡驱动程序；若为双显卡笔记本，请在显卡控制面板中将 Java 设为使用“高性能独显”。".to_string(),
                log_snippet,
                report_file: report_file_path,
            };
        }

        if full_text.contains("has been compiled by a more recent version of the Java Runtime")
            || full_text.contains("Unsupported class file major version")
            || full_text.contains("Unsupported major.minor version")
        {
            return CrashAnalysisResult {
                exit_code,
                title: "Java 版本不匹配 (版本过低)".to_string(),
                reason: "当前游戏核心或某个 Mod 是使用更高版本的 Java 编译的，你当前使用的 Java 无法运行它。".to_string(),
                suggestion: "1.17~1.20.4 游戏推荐使用 Java 17；1.20.5 及以上版本推荐使用 Java 21。请在启动器首页选择对应版本的 Java。".to_string(),
                log_snippet,
                report_file: report_file_path,
            };
        }

        if full_text.contains("Unable to make protected final java.lang.Class java.lang.ClassLoader.defineClass")
            || full_text.contains("java.lang.NoSuchFieldException: ucp")
            || full_text.contains("because module java.base does not export")
        {
            return CrashAnalysisResult {
                exit_code,
                title: "Java 版本过高 (模块反射限制)".to_string(),
                reason: "旧版本的 Forge 或 Mod 试图非法反射 Java 内部类，但高版本 Java (Java 16+) 启用了严格模块隔离。".to_string(),
                suggestion: "对于 Minecraft 1.12.2 或 1.16.5 等旧版本 Forge，推荐在启动器首页选用官方标准 Java 8 (1.8.0)。".to_string(),
                log_snippet,
                report_file: report_file_path,
            };
        }

        if full_text.contains("Incompatible mods found!")
            || full_text.contains("net.fabricmc.loader.impl.FormattedException")
            || full_text.contains("Missing or unsupported mandatory dependencies")
            || full_text.contains("DuplicateModsFoundException")
            || full_text.contains("ModResolutionException: Duplicate")
        {
            return CrashAnalysisResult {
                exit_code,
                title: "模组兼容性冲突或前置缺失".to_string(),
                reason: "加载器在检查模组列表时发现存在冲突模组、重复安装的模组，或者缺失了某些必要的前置 Mod（如 Fabric API）。".to_string(),
                suggestion: "请仔细阅读下方日志中的黄色/红色提示行，补齐缺失的前置 Mod，或者将重复/冲突的 Mod 文件移除。".to_string(),
                log_snippet,
                report_file: report_file_path,
            };
        }

        if full_text.contains("org.spongepowered.asm.mixin.transformer.throwables.MixinTransformerError")
            || full_text.contains("org.spongepowered.asm.mixin.throwables.MixinApplyError")
        {
            return CrashAnalysisResult {
                exit_code,
                title: "Mixin 注入失败 (Mod 版本与游戏版本不符)".to_string(),
                reason: "某个 Mod 尝试通过 Mixin 修改游戏底层字节码时失败，通常是由于该 Mod 的目标 Minecraft 小版本与当前版本不一致。".to_string(),
                suggestion: "请查看日志截图中报错的包名前缀，找到对应的 Mod，并在下载中心或 Mod 官网更新到适用于当前游戏版本的对应构建。".to_string(),
                log_snippet,
                report_file: report_file_path,
            };
        }

        if full_text.contains("java.lang.NullPointerException: Rendering overlay")
            || full_text.contains("Shaders Mod detected")
            || full_text.contains("1282: Invalid operation")
        {
            return CrashAnalysisResult {
                exit_code,
                title: "光影渲染或 OptiFine 冲突".to_string(),
                reason: "着色器管线加载异常或光影包配置文件损坏。".to_string(),
                suggestion: "建议在【版本管理 -> 光影包】中暂时关闭当前光影，或使用更兼容现代版本的 Sodium + Iris 光影加载器。".to_string(),
                log_snippet,
                report_file: report_file_path,
            };
        }

        // 默认兜底诊断
        let code_str = exit_code.map(|c| c.to_string()).unwrap_or_else(|| "未知".to_string());
        CrashAnalysisResult {
            exit_code,
            title: format!("游戏异常退出 (代码: {})", code_str),
            reason: "游戏进程在非正常状态下突然终止退出。".to_string(),
            suggestion: "请查看下方报错日志摘要。如果生成了崩溃日志文件，可点击下方按钮直接打开排查。".to_string(),
            log_snippet: if log_snippet.is_empty() { "暂无控制台日志输出".to_string() } else { log_snippet },
            report_file: report_file_path,
        }
    }
}
