use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

/// horizon-client --launch-api 响应数据契约
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientLaunchApi {
    pub launch_api: String,
    pub horizon: String,
    pub minecraft: String,
    #[serde(default)]
    pub options: Vec<LaunchOptionDescriptor>,
    #[serde(default)]
    pub exit_codes: Vec<i32>,
    #[serde(default)]
    pub crash_report_line: Option<String>,
    #[serde(default)]
    pub content: Option<serde_json::Value>,
}

impl ClientLaunchApi {
    /// 检查指定选项名是否在协商所得的可用选项列表中
    pub fn is_option_supported(&self, option_name: &str) -> bool {
        self.options.iter().any(|opt| opt.name == option_name)
    }
}

/// 命令行选项描述（如 {"name": "--width", "value": "N"}）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LaunchOptionDescriptor {
    pub name: String,
    #[serde(default)]
    pub value: Option<String>,
}

/// horizon-prepare --launch-api 响应数据契约
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrepareLaunchApi {
    pub launch_api: String,
    pub horizon: String,
    pub minecraft: String,
    pub java_major: u32,
    #[serde(default)]
    pub output_version: Option<u32>,
    #[serde(default)]
    pub schema: Option<u32>,
    #[serde(default)]
    pub options: Vec<LaunchOptionDescriptor>,
    #[serde(default)]
    pub exit_codes: Vec<i32>,
}

impl PrepareLaunchApi {
    /// 检查指定选项名是否在协商所得的可用选项列表中
    pub fn is_option_supported(&self, option_name: &str) -> bool {
        self.options.iter().any(|opt| opt.name == option_name)
    }
}

/// 校验 Launch API 版本大版本是否兼容
/// 当前上游契约为 0.1.x（experimental），大版本非 0 即判定不兼容
pub fn check_launch_api_compatibility(version_str: &str) -> Result<(), String> {
    let mut parts = version_str.trim().split('.');
    let major = parts
        .next()
        .and_then(|m| m.parse::<u64>().ok())
        .ok_or_else(|| format!("无效的 Launch API 版本号格式: {}", version_str))?;

    if major != 0 {
        return Err(format!(
            "Launch API 大版本不兼容：期望大版本 0.x，实际构建为 {}",
            version_str
        ));
    }
    Ok(())
}

/// 从 JSON 字符串解析 ClientLaunchApi（纯函数，忽略未知字段并校验 API 版本）
pub fn parse_client_launch_api(json_str: &str) -> Result<ClientLaunchApi, String> {
    let api: ClientLaunchApi = serde_json::from_str(json_str)
        .map_err(|e| format!("解析 horizon-client --launch-api 失败: {}", e))?;
    check_launch_api_compatibility(&api.launch_api)?;
    Ok(api)
}

/// 从 JSON 字符串解析 PrepareLaunchApi（纯函数，忽略未知字段并校验 API 版本）
pub fn parse_prepare_launch_api(json_str: &str) -> Result<PrepareLaunchApi, String> {
    let api: PrepareLaunchApi = serde_json::from_str(json_str)
        .map_err(|e| format!("解析 horizon-prepare --launch-api 失败: {}", e))?;
    check_launch_api_compatibility(&api.launch_api)?;
    Ok(api)
}

/// 运行 `horizon-client --launch-api` 进行选项协商
pub fn negotiate_client_api(bin_path: &Path) -> Result<ClientLaunchApi, String> {
    if !bin_path.exists() {
        return Err(format!("找不到 horizon-client 可执行文件: {:?}", bin_path));
    }
    let output = Command::new(bin_path)
        .arg("--launch-api")
        .output()
        .map_err(|e| format!("执行 horizon-client --launch-api 失败: {}", e))?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "horizon-client --launch-api 返回错误码 {:?}: {}",
            output.status.code(),
            err_msg.trim()
        ));
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    parse_client_launch_api(&stdout_str)
}

/// 运行 `horizon-prepare --launch-api` 进行选项与钉子协商
pub fn negotiate_prepare_api(bin_path: &Path) -> Result<PrepareLaunchApi, String> {
    if !bin_path.exists() {
        return Err(format!("找不到 horizon-prepare 可执行文件: {:?}", bin_path));
    }
    let output = Command::new(bin_path)
        .arg("--launch-api")
        .output()
        .map_err(|e| format!("执行 horizon-prepare --launch-api 失败: {}", e))?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "horizon-prepare --launch-api 返回错误码 {:?}: {}",
            output.status.code(),
            err_msg.trim()
        ));
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    parse_prepare_launch_api(&stdout_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_client_launch_api_real_structure() {
        let sample = r#"{
          "content": {
            "datapack": { "format": [121, 0] },
            "index": { "schema": 1 },
            "mod": { "api": ["0.2"] },
            "modpack": { "format": 1 },
            "native_mod": { "exact_build": true, "rebuild": true },
            "resourcepack": { "format": [97, 1] },
            "shader": { "api": ["0.2"], "schema": 1 }
          },
          "crash_report_line": "Horizon: crash report saved to ",
          "exit_codes": [0, 1, 2, 101],
          "horizon": "0.1.1",
          "launch_api": "0.1.1",
          "minecraft": "26.3",
          "options": [
            { "name": "--resources", "value": "DIR" },
            { "name": "--game-dir", "value": "DIR" },
            { "name": "--home", "value": "DIR" },
            { "name": "--width", "value": "N" },
            { "name": "--height", "value": "N" },
            { "name": "--fullscreen", "value": null },
            { "name": "--session-stdin", "value": null },
            { "name": "--launch-api", "value": null }
          ],
          "unknown_future_field": "readers must ignore"
        }"#;

        let api = parse_client_launch_api(sample).expect("应成功解析并忽略未知字段");
        assert_eq!(api.launch_api, "0.1.1");
        assert_eq!(api.horizon, "0.1.1");
        assert_eq!(api.minecraft, "26.3");
        assert_eq!(
            api.crash_report_line.as_deref(),
            Some("Horizon: crash report saved to ")
        );
        assert!(api.is_option_supported("--resources"));
        assert!(api.is_option_supported("--session-stdin"));
        assert!(api.is_option_supported("--width"));
        assert!(!api.is_option_supported("--unsupported-option"));
    }

    #[test]
    fn test_parse_prepare_launch_api_real_structure() {
        let sample = r#"{
          "launch_api": "0.1.1",
          "horizon": "0.1.1",
          "minecraft": "26.3",
          "java_major": 25,
          "output_version": 1,
          "schema": 1,
          "options": [
            { "name": "--prepare", "value": null },
            { "name": "--output", "value": "DIR" },
            { "name": "--complete", "value": null },
            { "name": "--progress", "value": "json" },
            { "name": "--check", "value": null },
            { "name": "--version-json", "value": "PATH" },
            { "name": "--client-jar", "value": "PATH" },
            { "name": "--libraries-dir", "value": "DIR" },
            { "name": "--assets-dir", "value": "DIR" },
            { "name": "--server-jar", "value": "PATH" },
            { "name": "--java-home", "value": "PATH" }
          ],
          "exit_codes": [0, 1, 2]
        }"#;

        let api = parse_prepare_launch_api(sample).expect("应成功解析");
        assert_eq!(api.java_major, 25);
        assert_eq!(api.minecraft, "26.3");
        assert!(api.is_option_supported("--prepare"));
        assert!(api.is_option_supported("--java-home"));
        assert!(api.is_option_supported("--check"));
    }

    #[test]
    fn test_launch_api_incompatible_version() {
        let bad_sample = r#"{
          "launch_api": "2.0.0",
          "horizon": "2.0.0",
          "minecraft": "27.0",
          "options": []
        }"#;
        let res = parse_client_launch_api(bad_sample);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("大版本不兼容"));
    }
}
