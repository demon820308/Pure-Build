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

struct CrashRule {
    keywords: &'static [&'static str],
    title: &'static str,
    reason: &'static str,
    suggestion: &'static str,
}

/// 移植自原版 PCL2 `ModCrash.vb` 的结构化崩溃特征知识库
const CRASH_RULES: &[CrashRule] = &[
    CrashRule {
        keywords: &[
            "java.lang.OutOfMemoryError",
            "an out of memory error",
            "Could not reserve enough space for",
            "GC overhead limit exceeded",
        ],
        title: "内存不足 (Out Of Memory)",
        reason: "游戏在运行过程中消耗了所有分配的内存，触发了 Java 虚拟机内存溢出。",
        suggestion: "请前往启动器【设置】页面，将【最大内存分配】调大（推荐 4096MB 或 8192MB），或排查是否有过多高清材质包/大型模组。",
    },
    CrashRule {
        keywords: &[
            "The driver does not appear to support OpenGL",
            "GLFW error 65542: WGL",
            "Couldn't set pixel format",
            "GLXBadFBConfig",
        ],
        title: "显卡不支持或 OpenGL 驱动缺失",
        reason: "Minecraft 无法创建 OpenGL 渲染上下文，通常是由于显卡驱动未正确安装或版本过旧。",
        suggestion: "请前往显卡官网（NVIDIA / AMD / Intel）更新最新的显卡驱动程序；若为双显卡笔记本，请在系统图形设置中将 Java 设为“高性能独立显卡”。",
    },
    CrashRule {
        keywords: &[
            "nvoglv64.dll",
            "nvoglv32.dll",
            "atio6axx.dll",
            "ig7icd64.dll",
            "ig9icd64.dll",
            "libGL.dylib",
        ],
        title: "显卡驱动底层崩溃 (Native Driver Crash)",
        reason: "OpenGL 显卡驱动动态库在渲染过程中发生内存访问违例 (EXCEPTION_ACCESS_VIOLATION)。",
        suggestion: "1. 更新或清洁安装显卡驱动；2. 若安装了 Sodium (钠) / Iris / OptiFine，尝试暂时禁用光影或更新优化模组版本。",
    },
    CrashRule {
        keywords: &[
            "has been compiled by a more recent version of the Java Runtime",
            "Unsupported class file major version",
            "Unsupported major.minor version",
        ],
        title: "Java 版本过低 (Class Version Mismatch)",
        reason: "当前游戏核心或某个 Mod 是使用更高版本的 Java 编译的，当前选中的 Java 无法加载它。",
        suggestion: "Minecraft 1.17~1.20.4 推荐使用 Java 17；1.20.5 及以上版本必须使用 Java 21。请在启动器首页切换对应的高版本 Java。",
    },
    CrashRule {
        keywords: &[
            "Unable to make protected final java.lang.Class java.lang.ClassLoader.defineClass",
            "java.lang.NoSuchFieldException: ucp",
            "because module java.base does not export",
            "java.lang.ClassCastException: class jdk.internal.loader.ClassLoaders$AppClassLoader cannot be cast to class java.net.URLClassLoader",
        ],
        title: "Java 版本过高 (旧版 Forge 反射限制)",
        reason: "旧版本 Forge (1.12.2 / 1.16.5 等) 尝试反射访问 Java 8 内部类，但 Java 16+ 启用了严格模块封装。",
        suggestion: "运行 Minecraft 1.16.5 及更早的 Forge 版本时，请在启动器首页切换使用 Java 8 (1.8.0)。",
    },
    CrashRule {
        keywords: &[
            "Cannot findlaunchtarget",
            "ClassNotFoundException: net.fabricmc.loader.impl.launch.knot.KnotClient",
            "ClassNotFoundException: cpw.mods.bootstraplauncher.BootstrapLauncher",
            "ClassNotFoundException: net.minecraft.client.main.Main",
        ],
        title: "核心支持库缺失或主类未找到",
        reason: "启动器未能找到游戏或加载器（Fabric/Forge）的入口主类，通常是支持库未完整下载。",
        suggestion: "请前往【下载】页面重新安装该版本以自动校验并补全所有缺失的运行支持库。",
    },
    CrashRule {
        keywords: &[
            "Incompatible mods found!",
            "net.fabricmc.loader.impl.FormattedException",
            "Missing or unsupported mandatory dependencies",
            "ModResolutionException",
        ],
        title: "模组前置缺失或版本不兼容",
        reason: "加载器在进行模组依赖解析时发现缺少必需的前置 Mod（例如 Fabric API、Architectury、Cloth Config 等）或版本不匹配。",
        suggestion: "请阅读下方日志截取中提示缺失的 Mod ID，前往【下载 -> 社区模组】搜索并一键安装对应前置 Mod。",
    },
    CrashRule {
        keywords: &[
            "DuplicateModsFoundException",
            "Found duplicate mods",
            "Mod ID: 'rubidium' from mod files",
        ],
        title: "检测到重复安装的模组 (Duplicate Mods)",
        reason: "mods 文件夹中存在两个或多个相同 Mod ID 的文件（或者是同一模组的不同版本、又或是 Rubidium 与 Embeddium 等同源移植模组共存）。",
        suggestion: "请前往【版本 -> 模组 (Mods)】页面，禁用或删除重复安装的模组文件。",
    },
    CrashRule {
        keywords: &[
            "optifine.OptiFineTransformationService",
            "me.jellysquid.mods.sodium",
            "net.coderbot.iris",
        ],
        title: "渲染优化模组冲突 (OptiFine / Sodium / Iris)",
        reason: "OptiFine 与现代渲染优化模组（如 Sodium 钠、Iris 光影、Embeddium）修改了相同的底层渲染管线，不能同时启用。",
        suggestion: "请在【版本 -> 模组】中移除 OptiFine，改用 Sodium + Iris 组合（或保留 OptiFine 并禁用冲突模组）。",
    },
    CrashRule {
        keywords: &[
            "org.spongepowered.asm.mixin.transformer.throwables.MixinTransformerError",
            "org.spongepowered.asm.mixin.throwables.MixinApplyError",
            "Critical injection failure",
        ],
        title: "Mixin 字节码注入失败 (Mod 与当前版本不兼容)",
        reason: "某个 Mod 尝试通过 Mixin 注入修改游戏底层代码时失败，通常因为该 Mod 针对的是其它小版本，或与其他核心修改 Mod 冲突。",
        suggestion: "请查看下方日志中报错的 mixin 配置文件名（如 xxx.mixins.json），定位对应 Mod 并更新或暂时停用它。",
    },
    CrashRule {
        keywords: &[
            "java.util.zip.ZipException: zip END header not found",
            "Invalid module descriptor",
            "error in opening zip file",
        ],
        title: "Mod 或支持库 JAR 文件损坏",
        reason: "某个 `.jar` 模组或支持库在下载或复制过程中未完整写入，导致归档文件头损坏。",
        suggestion: "请检查最近新下载的 Mod 文件大小是否异常（如 0KB），将其删除后重新下载。",
    },
    CrashRule {
        keywords: &[
            "java.nio.file.FileSystemException",
            "The process cannot access the file because it is being used by another process",
            "Failed to lock",
            "SessionLock",
        ],
        title: "游戏文件或存档被其他进程锁定",
        reason: "另一个未完全退出的 Minecraft 实例或外部程序正在占用当前存档 (session.lock) 或日志文件。",
        suggestion: "请先关闭后台残留的 Java 游戏进程后再重新启动游戏。",
    },
    CrashRule {
        keywords: &[
            "java.lang.NullPointerException: Rendering overlay",
            "Shaders Mod detected",
            "1282: Invalid operation",
        ],
        title: "光影或材质资源加载崩溃",
        reason: "游戏在初始化渲染覆盖层（Overlay）或编译着色器时遇到空指针异常，通常由不兼容的材质包或光影包引起。",
        suggestion: "请前往【版本 -> 光影/材质】暂时停用刚添加的光影包或材质包。",
    },
];

pub struct CrashAnalyzer;

impl CrashAnalyzer {
    /// 分析游戏崩溃（结合退出代码、控制台尾部日志、latest.log 与 crash-reports）
    pub fn analyze(
        exit_code: Option<i32>,
        game_dir: &Path,
        version_dir: &Path,
        console_tail: &str,
    ) -> CrashAnalysisResult {
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

        let full_text = format!("{}\n{}\n{}", console_tail, latest_log, report_content);

        let mut snippet_lines: Vec<&str> = full_text
            .lines()
            .filter(|l| !l.trim().is_empty())
            .collect();
        if snippet_lines.len() > 35 {
            snippet_lines = snippet_lines[snippet_lines.len() - 35..].to_vec();
        }
        let log_snippet = snippet_lines.join("\n");

        // 遍历规则库匹配
        for rule in CRASH_RULES {
            if rule.keywords.iter().any(|kw| full_text.contains(kw)) {
                return CrashAnalysisResult {
                    exit_code,
                    title: rule.title.to_string(),
                    reason: rule.reason.to_string(),
                    suggestion: rule.suggestion.to_string(),
                    log_snippet: if log_snippet.is_empty() {
                        "暂无控制台日志输出".to_string()
                    } else {
                        log_snippet
                    },
                    report_file: report_file_path,
                };
            }
        }

        let code_str = exit_code
            .map(|c| c.to_string())
            .unwrap_or_else(|| "未知".to_string());
        CrashAnalysisResult {
            exit_code,
            title: format!("游戏异常退出 (代码: {})", code_str),
            reason: "游戏进程在非正常状态下突然终止退出。".to_string(),
            suggestion: "请查看下方报错日志摘要。如果生成了崩溃日志文件，可点击下方按钮直接打开排查。".to_string(),
            log_snippet: if log_snippet.is_empty() {
                "暂无控制台日志输出".to_string()
            } else {
                log_snippet
            },
            report_file: report_file_path,
        }
    }
}
