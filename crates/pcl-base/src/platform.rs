//! 全仓唯一的平台抽象层。
//! 所有与操作系统、CPU 架构、默认路径、外部命令（explorer/open/xdg-open/taskkill/kill）、
//! Java 候选目录、可执行文件名、开服脚本生成及权限设置相关的逻辑均唯一收敛于此。

use crate::endpoints::{
    APP_IDENTIFIER, DEFAULT_MAX_MEMORY_MB, DEFAULT_MIN_MEMORY_MB, DEFAULT_MUSIC_DIR_NAME,
};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// 全局缓存由 Tauri 运行时注入的 app_config_dir（若未注入则回退至系统推导目录）
static APP_CONFIG_DIR_OVERRIDE: OnceLock<PathBuf> = OnceLock::new();

/// 前端查询的当前平台元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformInfo {
    pub os: String,
    pub arch: String,
    pub default_game_dir: String,
    pub default_music_dir: String,
    pub server_script_name: String,
    pub is_macos: bool,
}

/// 在 Tauri 启动时注册框架推导的 `app_config_dir`
pub fn set_app_config_dir(dir: PathBuf) {
    let _ = APP_CONFIG_DIR_OVERRIDE.set(dir);
}

/// 获取应用配置目录：优先使用 Tauri `app.path().app_config_dir()`，取不到时按平台环境变量推导回退
pub fn app_config_dir() -> PathBuf {
    if let Some(dir) = APP_CONFIG_DIR_OVERRIDE.get() {
        return dir.clone();
    }
    default_config_dir()
}

/// 获取默认背景音乐目录（位于应用配置目录 `Musics` 子目录下，运行期推导）
pub fn default_music_dir() -> PathBuf {
    app_config_dir().join(DEFAULT_MUSIC_DIR_NAME)
}

/// 将 Rust 标准库 OS 标识映射为 Mojang 版本 JSON 规范的 `os.name`
/// Windows -> `windows`, macOS -> `osx` (注意不是 `macos`), Linux -> `linux`
pub fn map_mc_os_name(raw_os: &str) -> &'static str {
    match raw_os {
        "windows" => "windows",
        "macos" | "osx" => "osx",
        "linux" => "linux",
        _ => "linux",
    }
}

/// 当前运行平台的 Mojang `os.name` 取值
pub fn mc_os_name() -> &'static str {
    map_mc_os_name(env::consts::OS)
}

/// 将 Rust 标准库架构标识映射为 Mojang 规则匹配用架构：`x86` / `x64` / `arm64`
pub fn map_mc_os_arch(raw_arch: &str) -> &'static str {
    match raw_arch {
        "aarch64" | "arm64" => "arm64",
        "x86_64" | "amd64" | "x64" => "x64",
        "x86" | "i686" | "i386" => "x86",
        _ => "x64",
    }
}

/// 当前运行平台的 Mojang `os.arch` 取值
pub fn mc_os_arch() -> &'static str {
    map_mc_os_arch(env::consts::ARCH)
}

/// 将系统/Mojang OS 标识映射为 Adoptium API 的 `os` 取值（纯函数，入参注入便于跨平台测试）
/// Windows -> `windows`, macOS/osx -> `mac`, Linux -> `linux`
pub fn map_adoptium_os_name(raw_os: &str) -> &'static str {
    match raw_os {
        "windows" => "windows",
        "macos" | "osx" => "mac",
        "linux" => "linux",
        _ => "linux",
    }
}

/// 当前平台的 Adoptium API `os` 取值
pub fn adoptium_os_name() -> &'static str {
    map_adoptium_os_name(mc_os_name())
}

/// 将系统/Mojang 架构标识映射为 Adoptium API 的 `architecture` 取值（纯函数，入参注入便于跨平台测试）
/// x64/x86_64 -> `x64`, aarch64/arm64 -> `aarch64`, x86/i386 -> `x32`
pub fn map_adoptium_arch_name(raw_arch: &str) -> &'static str {
    match raw_arch {
        "aarch64" | "arm64" => "aarch64",
        "x86_64" | "amd64" | "x64" => "x64",
        "x86" | "i686" | "i386" => "x32",
        _ => "x64",
    }
}

/// 当前平台的 Adoptium API `architecture` 取值
pub fn adoptium_arch_name() -> &'static str {
    map_adoptium_arch_name(mc_os_arch())
}

/// 判断当前是否为 macOS 平台
pub fn is_macos() -> bool {
    mc_os_name() == "osx"
}

/// 运行期推导当前用户的主目录（禁止写死 `/Users/...` 或盘符）
pub fn user_home_dir() -> Option<PathBuf> {
    if let Some(home) = env::var_os("HOME") {
        let p = PathBuf::from(home);
        if !p.as_os_str().is_empty() {
            return Some(p);
        }
    }
    if let Some(userprofile) = env::var_os("USERPROFILE") {
        let p = PathBuf::from(userprofile);
        if !p.as_os_str().is_empty() {
            return Some(p);
        }
    }
    if let (Some(drive), Some(path)) = (env::var_os("HOMEDRIVE"), env::var_os("HOMEPATH")) {
        let mut p = PathBuf::from(drive);
        p.push(path);
        return Some(p);
    }
    None
}

/// 按给定平台名称推导默认 `.minecraft` 目录（纯函数形式，便于测试与运行期复用）
pub fn derive_default_game_dir(os_name: &str, home: Option<&Path>, appdata: Option<&Path>) -> PathBuf {
    match os_name {
        "windows" => {
            if let Some(ad) = appdata {
                return ad.join(".minecraft");
            }
            if let Some(h) = home {
                return h.join("AppData").join("Roaming").join(".minecraft");
            }
            PathBuf::from(".minecraft")
        }
        "osx" => {
            if let Some(h) = home {
                return h
                    .join("Library")
                    .join("Application Support")
                    .join("minecraft");
            }
            PathBuf::from("minecraft")
        }
        _ => {
            if let Some(h) = home {
                return h.join(".minecraft");
            }
            PathBuf::from(".minecraft")
        }
    }
}

/// 按当前平台推导的默认 `.minecraft` 游戏目录
pub fn default_game_dir() -> PathBuf {
    let home = user_home_dir();
    let appdata = env::var_os("APPDATA").map(PathBuf::from);
    derive_default_game_dir(mc_os_name(), home.as_deref(), appdata.as_deref())
}

/// 按当前平台推导的默认配置目录回退路径（当 Tauri `app_config_dir` 不可用时使用）
pub fn default_config_dir() -> PathBuf {
    let home = user_home_dir();
    match mc_os_name() {
        "windows" => {
            if let Some(appdata) = env::var_os("APPDATA") {
                return PathBuf::from(appdata).join(APP_IDENTIFIER);
            }
            if let Some(h) = home {
                return h.join("AppData").join("Roaming").join(APP_IDENTIFIER);
            }
            PathBuf::from(".")
        }
        "osx" => {
            if let Some(h) = home {
                return h
                    .join("Library")
                    .join("Application Support")
                    .join(APP_IDENTIFIER);
            }
            PathBuf::from(".")
        }
        _ => {
            if let Some(xdg) = env::var_os("XDG_CONFIG_HOME") {
                return PathBuf::from(xdg).join(APP_IDENTIFIER);
            }
            if let Some(h) = home {
                return h.join(".config").join(APP_IDENTIFIER);
            }
            PathBuf::from(".")
        }
    }
}

/// 当前平台的 Java 可执行文件名（Windows -> `javaw.exe`，其余 -> `java`）
pub fn java_executable_name() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "javaw.exe"
    }
    #[cfg(not(target_os = "windows"))]
    {
        "java"
    }
}

/// 根据目标 OS 名称推导 Classpath 分隔符（纯逻辑映射，用于跨平台测试与参数构造）
pub fn classpath_separator_for_os(os: &str) -> char {
    if os == "windows" {
        ';'
    } else {
        ':'
    }
}

/// 当前平台的 Classpath 分隔符（由平台语义推导：Windows 为 `;`，Unix 为 `:`）
pub fn classpath_separator() -> char {
    classpath_separator_for_os(mc_os_name())
}

/// 本平台所有 Java 候选搜索目录
pub fn java_search_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let home = user_home_dir();

    #[cfg(target_os = "windows")]
    {
        let vendor_folders = [
            "Java",
            "Eclipse Adoptium",
            "BellSoft",
            "Zulu",
            "Microsoft",
            "Amazon Corretto",
            "SapMachine",
        ];
        if let Some(pf) = env::var_os("ProgramFiles") {
            let base = PathBuf::from(pf);
            for vendor in vendor_folders {
                dirs.push(base.join(vendor));
            }
        }
        if let Some(pf86) = env::var_os("ProgramFiles(x86)") {
            let base = PathBuf::from(pf86);
            for vendor in vendor_folders {
                dirs.push(base.join(vendor));
            }
        }
        if let Some(local_appdata) = env::var_os("LOCALAPPDATA") {
            let base = PathBuf::from(local_appdata).join("Programs");
            dirs.push(base.join("Eclipse Adoptium"));
            dirs.push(base.join("Java"));
        }
    }

    #[cfg(target_os = "macos")]
    {
        // 1. 解析 /usr/libexec/java_home -V 输出的系统已注册全部 JDK
        let java_home_tool = PathBuf::from("/usr/libexec/java_home");
        if java_home_tool.exists() {
            if let Ok(output) = Command::new(&java_home_tool).arg("-V").output() {
                let combined = format!(
                    "{}\n{}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
                for parsed_home in parse_macos_java_home_output(&combined) {
                    dirs.push(parsed_home);
                }
            }
        }

        // 2. /Library/Java/JavaVirtualMachines 与 ~/Library/Java/JavaVirtualMachines
        let mut jvm_roots = vec![PathBuf::from("/Library/Java/JavaVirtualMachines")];
        if let Some(ref h) = home {
            jvm_roots.push(
                h.join("Library")
                    .join("Java")
                    .join("JavaVirtualMachines"),
            );
        }
        for root in jvm_roots {
            if let Ok(entries) = fs::read_dir(&root) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    let contents_home = p.join("Contents").join("Home");
                    if contents_home.is_dir() {
                        dirs.push(contents_home);
                    } else if p.is_dir() {
                        dirs.push(p);
                    }
                }
            }
            dirs.push(root);
        }

        // 3. Homebrew: /opt/homebrew/opt (Apple Silicon) 与 /usr/local/opt (Intel) 下的 openjdk*
        let brew_opt_roots = [
            PathBuf::from("/opt/homebrew/opt"),
            PathBuf::from("/usr/local/opt"),
        ];
        for brew_root in brew_opt_roots {
            if let Ok(entries) = fs::read_dir(&brew_root) {
                for entry in entries.flatten() {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    if name_str.starts_with("openjdk") {
                        let p = entry.path();
                        let jdk_home = p
                            .join("libexec")
                            .join("openjdk.jdk")
                            .join("Contents")
                            .join("Home");
                        if jdk_home.is_dir() {
                            dirs.push(jdk_home);
                        }
                        dirs.push(p);
                    }
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        dirs.push(PathBuf::from("/usr/lib/jvm"));
        dirs.push(PathBuf::from("/usr/java"));
        dirs.push(PathBuf::from("/opt/java"));
        dirs.push(PathBuf::from("/opt/jdk"));
    }

    // 跨平台通用用户级 JDK 目录 (~/.jdks, ~/.sdkman/candidates/java)
    if let Some(ref h) = home {
        dirs.push(h.join(".jdks"));
        dirs.push(h.join(".sdkman").join("candidates").join("java"));
    }

    // 官方启动器 runtime：<默认游戏目录>/runtime
    dirs.push(default_game_dir().join("runtime"));

    dirs
}

/// 解析 macOS `/usr/libexec/java_home -V` 的文本输出，提取其中注册的 JDK Home 路径
pub fn parse_macos_java_home_output(text: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        // `/usr/libexec/java_home -V` 每条 JDK 行末尾为以 `/` 开头的绝对路径（通常由制表符或空格分隔）
        if let Some(idx) = trimmed.rfind("\t/") {
            let path_str = trimmed[idx + 1..].trim();
            if !path_str.is_empty() {
                paths.push(PathBuf::from(path_str));
            }
        } else if let Some(idx) = trimmed.find(" /Library/") {
            let path_str = trimmed[idx + 1..].trim();
            if !path_str.is_empty() {
                paths.push(PathBuf::from(path_str));
            }
        } else if let Some(idx) = trimmed.find(" /Users/") {
            let path_str = trimmed[idx + 1..].trim();
            if !path_str.is_empty() {
                paths.push(PathBuf::from(path_str));
            }
        } else if let Some(idx) = trimmed.find(" /opt/") {
            let path_str = trimmed[idx + 1..].trim();
            if !path_str.is_empty() {
                paths.push(PathBuf::from(path_str));
            }
        }
    }
    paths
}

/// 按平台打开本地文件夹或定位文件（Windows -> explorer，macOS -> open，Linux -> xdg-open）
pub fn open_folder(path: &Path) -> Result<(), String> {
    let target = if path.is_file() {
        path.parent().unwrap_or(path)
    } else {
        if !path.exists() {
            fs::create_dir_all(path).map_err(|e| format!("创建目录失败: {}", e))?;
        }
        path
    };

    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(target.as_os_str())
            .spawn()
            .map_err(|e| format!("无法打开文件夹: {}", e))?;
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        let status = Command::new("open")
            .arg(target.as_os_str())
            .status()
            .map_err(|e| format!("无法调用 open 打开文件夹: {}", e))?;
        if !status.success() {
            return Err(format!("open 命令返回非零状态码: {}", status));
        }
        return Ok(());
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        Command::new("xdg-open")
            .arg(target.as_os_str())
            .spawn()
            .map_err(|e| format!("无法调用 xdg-open 打开文件夹: {}", e))?;
        Ok(())
    }
}

/// 按平台在系统默认浏览器中打开外部链接（Windows -> explorer/start，macOS -> open，Linux -> xdg-open）
pub fn open_url(url: &str) -> Result<(), String> {
    if url.trim().is_empty() {
        return Err("URL 不能为空".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(url)
            .spawn()
            .map_err(|e| format!("无法调用 explorer 打开外部链接: {}", e))?;
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        let status = Command::new("open")
            .arg(url)
            .status()
            .map_err(|e| format!("无法调用 open 打开链接: {}", e))?;
        if !status.success() {
            return Err(format!("open 打开链接返回非零状态码: {}", status));
        }
        return Ok(());
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map_err(|e| format!("无法调用 xdg-open 打开链接: {}", e))?;
        Ok(())
    }
}

/// 按平台强制终止指定 PID 的进程（Windows -> `taskkill /F`，其余 -> `kill -9`）
pub fn kill_process(pid: u32) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let output = Command::new("taskkill")
            .args(["/F", "/PID", &pid.to_string()])
            .output()
            .map_err(|e| format!("终止进程失败: {}", e))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("taskkill 执行失败: {}", stderr.trim()));
        }
        return Ok(());
    }

    #[cfg(not(target_os = "windows"))]
    {
        let output = Command::new("kill")
            .args(["-9", &pid.to_string()])
            .output()
            .map_err(|e| format!("终止进程失败: {}", e))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("kill -9 执行失败: {}", stderr.trim()));
        }
        Ok(())
    }
}

/// 当前平台生成的开服脚本默认文件名
pub fn server_script_filename() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "启动服务端.bat"
    }
    #[cfg(not(target_os = "windows"))]
    {
        "启动服务端.sh"
    }
}

/// 生成本平台开服脚本并设置可执行权限（自动读取用户配置的 `max_memory_mb`）
pub fn server_script(dir: &Path, version_id: &str, jar_name: &str) -> Result<String, String> {
    let max_mem = crate::config::ConfigManager::load().max_memory_mb;
    server_script_with_memory(dir, version_id, jar_name, max_mem)
}

/// 生成本平台开服脚本并设置可执行权限（内存取用户配置 `max_memory_mb`，Unix 下 `chmod 0o755`）
pub fn server_script_with_memory(
    dir: &Path,
    version_id: &str,
    jar_name: &str,
    max_memory_mb: u32,
) -> Result<String, String> {
    let effective_max = if max_memory_mb == 0 {
        DEFAULT_MAX_MEMORY_MB
    } else {
        max_memory_mb
    };
    let effective_min = DEFAULT_MIN_MEMORY_MB.min(effective_max);
    let script_name = server_script_filename();
    let script_path = dir.join(script_name);

    #[cfg(target_os = "windows")]
    {
        let content = format!(
            "@echo off\r\ntitle Minecraft {version_id} 原版服务端\r\ncd /d \"%~dp0\"\r\necho 正在启动 Minecraft {version_id} 服务端 (最大内存: {effective_max}MB)...\r\necho 如果提示需要同意 EULA，请确认已同意并检查 eula.txt 中的 eula=true\r\necho --------------------------------------------------\r\n\"java\" -server -XX:+UseG1GC -Xmx{effective_max}M -Xms{effective_min}M -jar \"{jar_name}\" nogui\r\necho --------------------------------------------------\r\necho 服务端已退出。\r\npause\r\n"
        );
        fs::write(&script_path, content)
            .map_err(|e| format!("写入服务端启动脚本失败: {}", e))?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        let content = format!(
            "#!/bin/sh\ncd \"$(dirname \"$0\")\" || exit 1\necho \"正在启动 Minecraft {version_id} 服务端 (最大内存: {effective_max}MB)...\"\necho \"如果提示需要同意 EULA，请确认已同意并检查 eula.txt 中的 eula=true\"\necho \"--------------------------------------------------\"\njava -server -XX:+UseG1GC -Xmx{effective_max}M -Xms{effective_min}M -jar \"{jar_name}\" nogui\necho \"--------------------------------------------------\"\necho \"服务端已退出。\"\n"
        );
        fs::write(&script_path, content)
            .map_err(|e| format!("写入服务端启动脚本失败: {}", e))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o755);
            fs::set_permissions(&script_path, perms)
                .map_err(|e| format!("设置脚本可执行权限 (0o755) 失败: {}", e))?;
        }
    }

    Ok(script_name.to_string())
}

/// 设置指定文件为可执行权限（Unix 平台设置 0o755，Windows 平台无操作）
pub fn make_executable(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o755);
        fs::set_permissions(path, perms)
            .map_err(|e| format!("设置可执行权限 (0o755) 失败: {}", e))?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}

/// 为凭据文件设置仅当前用户可读写权限（Unix 下 `0o600`）
pub fn restrict_credentials_permissions(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if path.exists() {
            let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
        }
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
}

/// 获取供前端初始化使用的平台信息汇总
pub fn get_platform_info() -> PlatformInfo {
    PlatformInfo {
        os: mc_os_name().to_string(),
        arch: mc_os_arch().to_string(),
        default_game_dir: default_game_dir().to_string_lossy().to_string(),
        default_music_dir: default_music_dir().to_string_lossy().to_string(),
        server_script_name: server_script_filename().to_string(),
        is_macos: is_macos(),
    }
}

/// 当在缺失 WebView2 等运行环境的电脑上启动失败时，写入错误日志并弹出系统级提示框，避免静默闪退
pub fn report_startup_error(err_msg: &str) {
    let full_msg = format!(
        "Pure Build Launcher 启动失败：{}\n\n若在另一台电脑打不开，请确认系统已安装 Microsoft Edge WebView2 Runtime，或直接使用安装包 (Setup.exe) 安装运行。",
        err_msg
    );
    if let Ok(exe_path) = env::current_exe() {
        if let Some(dir) = exe_path.parent() {
            let _ = fs::write(dir.join("startup_error.log"), &full_msg);
        }
    }
    #[cfg(target_os = "windows")]
    {
        let escaped = full_msg.replace('\'', "''").replace('\n', "`n");
        let ps_cmd = format!(
            "Add-Type -AssemblyName PresentationFramework; [System.Windows.MessageBox]::Show('{}', 'Pure Build Launcher 启动错误', 'OK', 'Error') | Out-Null",
            escaped
        );
        let _ = Command::new("powershell")
            .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &ps_cmd])
            .status();
    }
    #[cfg(not(target_os = "windows"))]
    {
        eprintln!("{}", full_msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mc_os_and_arch_mapping() {
        assert_eq!(map_mc_os_name("windows"), "windows");
        assert_eq!(map_mc_os_name("macos"), "osx");
        assert_eq!(map_mc_os_name("linux"), "linux");

        assert_eq!(map_mc_os_arch("aarch64"), "arm64");
        assert_eq!(map_mc_os_arch("x86_64"), "x64");
        assert_eq!(map_mc_os_arch("x86"), "x86");

        assert_eq!(map_adoptium_os_name("windows"), "windows");
        assert_eq!(map_adoptium_os_name("osx"), "mac");
        assert_eq!(map_adoptium_os_name("linux"), "linux");

        assert_eq!(map_adoptium_arch_name("arm64"), "aarch64");
        assert_eq!(map_adoptium_arch_name("x64"), "x64");
        assert_eq!(map_adoptium_arch_name("x86"), "x32");
    }

    #[test]
    fn test_derive_default_game_dir_across_platforms() {
        let mac_home = Path::new("/Users/steve");
        let mac_dir = derive_default_game_dir("osx", Some(mac_home), None);
        assert_eq!(
            mac_dir,
            mac_home
                .join("Library")
                .join("Application Support")
                .join("minecraft")
        );

        let win_appdata = Path::new("C:\\Users\\steve\\AppData\\Roaming");
        let win_dir = derive_default_game_dir("windows", None, Some(win_appdata));
        assert_eq!(win_dir, win_appdata.join(".minecraft"));

        let linux_home = Path::new("/home/steve");
        let linux_dir = derive_default_game_dir("linux", Some(linux_home), None);
        assert_eq!(linux_dir, linux_home.join(".minecraft"));
    }

    #[test]
    fn test_parse_macos_java_home_v_output() {
        let sample = r#"Matching Java Virtual Machines (2):
    21.0.2 (arm64) "Eclipse Adoptium" - "OpenJDK 21.0.2"	/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home
    17.0.10 (x86_64) "Homebrew" - "OpenJDK 17.0.10"	/opt/homebrew/Cellar/openjdk@17/17.0.10/libexec/openjdk.jdk/Contents/Home
/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home"#;
        let parsed = parse_macos_java_home_output(sample);
        assert_eq!(
            parsed,
            vec![
                PathBuf::from("/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home"),
                PathBuf::from(
                    "/opt/homebrew/Cellar/openjdk@17/17.0.10/libexec/openjdk.jdk/Contents/Home"
                ),
            ]
        );
    }
}

