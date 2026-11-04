use crate::endpoints::{
    CRAFATAR_AVATAR_BASE, DEFAULT_YGGDRASIL_API_ROOT, MC_LOGIN_XBOX_URL, MC_PROFILE_URL,
    MS_DEFAULT_VERIFICATION_URI, MS_DEVICE_CODE_URL, MS_OAUTH_CLIENT_ID, MS_TOKEN_URL,
    XBL_AUTH_URL, XSTS_AUTH_URL,
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

fn default_verification_uri() -> String {
    MS_DEFAULT_VERIFICATION_URI.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct DeviceCodeResponse {
    pub user_code: String,
    pub device_code: String,
    #[serde(alias = "verification_url")]
    #[serde(default = "default_verification_uri")]
    pub verification_uri: String,
    #[ts(type = "number")]
    pub expires_in: u64,
    #[ts(type = "number")]
    pub interval: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct UserProfile {
    pub name: String,
    pub uuid: String,
    pub access_token: String,
    pub avatar_url: String,
    #[serde(default = "default_auth_type")]
    pub auth_type: String, // "msa" | "yggdrasil"
    #[serde(default)]
    pub auth_server: Option<String>,
}

fn default_auth_type() -> String {
    "msa".to_string()
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct XboxLiveAuthRequest {
    #[serde(rename = "Properties")]
    properties: XboxLiveAuthProperties,
    #[serde(rename = "RelyingParty")]
    relying_party: String,
    #[serde(rename = "TokenType")]
    token_type: String,
}

#[derive(Debug, Serialize)]
struct XboxLiveAuthProperties {
    #[serde(rename = "AuthMethod")]
    auth_method: String,
    #[serde(rename = "SiteName")]
    site_name: String,
    #[serde(rename = "RpsTicket")]
    rps_ticket: String,
}

#[derive(Debug, Deserialize)]
struct XboxLiveAuthResponse {
    #[serde(rename = "Token")]
    token: Option<String>,
    #[serde(rename = "DisplayClaims")]
    display_claims: Option<XboxDisplayClaims>,
    #[serde(rename = "XErr")]
    xerr: Option<u64>,
    #[serde(rename = "Message")]
    message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenPollState {
    Success(String),
    Pending,
    DeclinedOrExpired(String),
}

#[derive(Debug, Deserialize)]
struct XboxDisplayClaims {
    xui: Option<Vec<XboxXui>>,
}

#[derive(Debug, Deserialize)]
struct XboxXui {
    uhs: Option<String>,
}

#[derive(Debug, Serialize)]
struct XstsAuthRequest {
    #[serde(rename = "Properties")]
    properties: XstsAuthProperties,
    #[serde(rename = "RelyingParty")]
    relying_party: String,
    #[serde(rename = "TokenType")]
    token_type: String,
}

#[derive(Debug, Serialize)]
struct XstsAuthProperties {
    #[serde(rename = "SandboxId")]
    sandbox_id: String,
    #[serde(rename = "UserTokens")]
    user_tokens: Vec<String>,
}

#[derive(Debug, Serialize)]
struct McLoginWithXboxRequest {
    #[serde(rename = "identityToken")]
    identity_token: String,
}

#[derive(Debug, Deserialize)]
struct McLoginResponse {
    access_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct McProfileResponse {
    id: Option<String>,
    name: Option<String>,
}

#[derive(Debug, Serialize)]
struct YggdrasilAuthRequest<'a> {
    agent: YggdrasilAgent<'a>,
    username: &'a str,
    password: &'a str,
    #[serde(rename = "requestUser")]
    request_user: bool,
}

#[derive(Debug, Serialize)]
struct YggdrasilAgent<'a> {
    name: &'a str,
    version: u32,
}

#[derive(Debug, Deserialize)]
struct YggdrasilAuthResponse {
    #[serde(rename = "accessToken")]
    access_token: Option<String>,
    #[serde(rename = "selectedProfile")]
    selected_profile: Option<McProfileResponse>,
    #[serde(rename = "availableProfiles")]
    available_profiles: Option<Vec<McProfileResponse>>,
    #[serde(rename = "errorMessage")]
    error_message: Option<String>,
}

pub struct MicrosoftAuth;

impl MicrosoftAuth {
    /// 解析微软设备代码响应 JSON
    pub fn parse_device_code_response(json: &str) -> Result<DeviceCodeResponse, String> {
        serde_json::from_str(json).map_err(|e| format!("解析设备码响应失败: {}", e))
    }

    /// 解析 OAuth Token 轮询响应 JSON
    pub fn parse_token_poll_response(json: &str) -> Result<TokenPollState, String> {
        let token_resp: TokenResponse =
            serde_json::from_str(json).map_err(|e| format!("解析 Token 响应失败: {}", e))?;

        if let Some(token) = token_resp.access_token {
            return Ok(TokenPollState::Success(token));
        }

        if let Some(err) = token_resp.error {
            if err == "authorization_declined"
                || err == "bad_verification_code"
                || err == "expired_token"
            {
                return Ok(TokenPollState::DeclinedOrExpired(err));
            }
            if err == "authorization_pending" {
                return Ok(TokenPollState::Pending);
            }
            return Err(format!("登录认证异常: {}", err));
        }

        Ok(TokenPollState::Pending)
    }

    /// 解析 Xbox Live 响应 JSON，提取 token 与 userhash
    pub fn parse_xbl_response(json: &str) -> Result<(String, String), String> {
        let resp: XboxLiveAuthResponse =
            serde_json::from_str(json).map_err(|e| format!("解析 Xbox Live 响应失败: {}", e))?;

        let token = resp.token.ok_or_else(|| "缺失 Xbox Live 凭据".to_string())?;
        let userhash = resp
            .display_claims
            .and_then(|c| c.xui)
            .and_then(|x| x.into_iter().next())
            .and_then(|u| u.uhs)
            .ok_or_else(|| "缺失 UserHash".to_string())?;

        Ok((token, userhash))
    }

    /// 解析 XSTS 响应 JSON，提取 xsts_token 并映射 XErr 错误码为可读说明
    pub fn parse_xsts_response(json: &str) -> Result<String, String> {
        let resp: XboxLiveAuthResponse =
            serde_json::from_str(json).map_err(|e| format!("解析 XSTS 响应失败: {}", e))?;

        if let Some(xerr) = resp.xerr {
            let err_desc = match xerr {
                2148916233 => "此微软账号属于未成年人（18岁以下），需要家长将其加入微软家庭组后方可游玩",
                2148916235 => "您所在的国家或地区不支持 Xbox Live",
                2148916236 | 2148916237 => "此账号需要前往 xbox.com 完成成人身份验证",
                2148916238 => "此微软账号尚未创建 Xbox 角色档案，请先前往 xbox.com 注册 Xbox 档案",
                code => {
                    if let Some(ref msg) = resp.message {
                        return Err(format!("XSTS 认证失败 (代码: {}): {}", code, msg));
                    }
                    return Err(format!("XSTS 认证失败，错误代码 (XErr: {})", code));
                }
            };
            return Err(format!("XSTS 认证失败: {} (代码: {})", err_desc, xerr));
        }

        resp.token.ok_or_else(|| "缺失 XSTS 凭据".to_string())
    }

    /// 解析 Minecraft 登录响应 JSON，提取 mc access_token
    pub fn parse_mc_login_response(json: &str) -> Result<String, String> {
        let resp: McLoginResponse =
            serde_json::from_str(json).map_err(|e| format!("解析 Minecraft 凭据失败: {}", e))?;
        resp.access_token
            .ok_or_else(|| "未获取到 Minecraft 令牌".to_string())
    }

    /// 解析 Minecraft Profile 响应 JSON，提取 (name, uuid)
    pub fn parse_mc_profile_response(json: &str) -> Result<(String, String), String> {
        let resp: McProfileResponse =
            serde_json::from_str(json).map_err(|e| format!("解析游戏角色失败: {}", e))?;
        let name = resp
            .name
            .unwrap_or_else(|| "MinecraftPlayer".to_string());
        let uuid = resp
            .id
            .unwrap_or_else(|| "00000000000000000000000000000000".to_string());
        Ok((name, uuid))
    }

    /// 解析 Yggdrasil 响应 JSON，提取 (access_token, name, uuid)
    pub fn parse_yggdrasil_response(
        json: &str,
        fallback_username: &str,
    ) -> Result<(String, String, String), String> {
        let resp: YggdrasilAuthResponse =
            serde_json::from_str(json).map_err(|e| format!("解析外置登录响应失败: {}", e))?;

        if let Some(err_msg) = resp.error_message {
            return Err(format!("外置登录认证失败: {}", err_msg));
        }

        let token = resp
            .access_token
            .ok_or_else(|| "未返回有效的 accessToken".to_string())?;
        let chosen_profile = resp
            .selected_profile
            .or_else(|| resp.available_profiles.and_then(|mut v| v.pop()))
            .ok_or_else(|| "该外置账号下尚未创建任何游戏角色".to_string())?;

        let name = chosen_profile
            .name
            .unwrap_or_else(|| fallback_username.to_string());
        let uuid = chosen_profile
            .id
            .unwrap_or_else(|| "00000000000000000000000000000000".to_string());

        Ok((token, name, uuid))
    }

    /// 1. 获取设备代码 (Device Code)，让用户在网页打开并输入配对码
    pub async fn get_device_code() -> Result<DeviceCodeResponse, String> {
        let client = reqwest::Client::new();
        let params = [
            ("client_id", MS_OAUTH_CLIENT_ID),
            ("scope", "service::user.auth.xboxlive.com::MBI_SSL"),
            ("response_type", "device_code"),
        ];

        let resp = client
            .post(MS_DEVICE_CODE_URL)
            .form(&params)
            .send()
            .await
            .map_err(|e| format!("请求设备码失败: {}", e))?;

        let text = resp
            .text()
            .await
            .map_err(|e| format!("读取设备码响应失败: {}", e))?;
        Self::parse_device_code_response(&text)
    }

    /// 2. 轮询微软授权状态，并在授权成功后走完全套 Minecraft Profile 获取链路
    pub async fn poll_for_token(device_code: String) -> Result<UserProfile, String> {
        let client = reqwest::Client::new();

        // 轮询 OAuth Token
        let mut ms_access_token = String::new();
        for _ in 0..60 {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;

            let params = [
                ("client_id", MS_OAUTH_CLIENT_ID),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("device_code", &device_code),
            ];

            let resp = client
                .post(MS_TOKEN_URL)
                .form(&params)
                .send()
                .await
                .map_err(|e| format!("轮询请求失败: {}", e))?;

            if let Ok(text) = resp.text().await {
                match Self::parse_token_poll_response(&text) {
                    Ok(TokenPollState::Success(token)) => {
                        ms_access_token = token;
                        break;
                    }
                    Ok(TokenPollState::DeclinedOrExpired(err)) => {
                        return Err(format!("登录已取消或超时: {}", err));
                    }
                    Ok(TokenPollState::Pending) => {}
                    Err(e) => {
                        return Err(e);
                    }
                }
            }
        }

        if ms_access_token.is_empty() {
            return Err("等待用户授权超时，请重试".to_string());
        }

        // 3. 交换 Xbox Live Token
        let xbl_req = XboxLiveAuthRequest {
            properties: XboxLiveAuthProperties {
                auth_method: "RPS".to_string(),
                site_name: "user.auth.xboxlive.com".to_string(),
                rps_ticket: format!("d={}", ms_access_token),
            },
            relying_party: "http://auth.xboxlive.com".to_string(),
            token_type: "JWT".to_string(),
        };

        let xbl_text = client
            .post(XBL_AUTH_URL)
            .json(&xbl_req)
            .send()
            .await
            .map_err(|e| format!("Xbox Live 认证失败: {}", e))?
            .text()
            .await
            .map_err(|e| format!("读取 Xbox Live 响应失败: {}", e))?;

        let (xbl_token, userhash) = Self::parse_xbl_response(&xbl_text)?;

        // 4. 交换 XSTS Token
        let xsts_req = XstsAuthRequest {
            properties: XstsAuthProperties {
                sandbox_id: "RETAIL".to_string(),
                user_tokens: vec![xbl_token],
            },
            relying_party: "rp://api.minecraftservices.com/".to_string(),
            token_type: "JWT".to_string(),
        };

        let xsts_text = client
            .post(XSTS_AUTH_URL)
            .json(&xsts_req)
            .send()
            .await
            .map_err(|e| format!("XSTS 认证失败: {}", e))?
            .text()
            .await
            .map_err(|e| format!("读取 XSTS 响应失败: {}", e))?;

        let xsts_token = Self::parse_xsts_response(&xsts_text)?;

        // 5. 换取 Minecraft 访问令牌
        let mc_req = McLoginWithXboxRequest {
            identity_token: format!("XBL3.0 x={};{}", userhash, xsts_token),
        };

        let mc_text = client
            .post(MC_LOGIN_XBOX_URL)
            .json(&mc_req)
            .send()
            .await
            .map_err(|e| format!("Minecraft 登录接口调用失败: {}", e))?
            .text()
            .await
            .map_err(|e| format!("读取 Minecraft 凭据失败: {}", e))?;

        let mc_token = Self::parse_mc_login_response(&mc_text)?;

        // 6. 获取正版角色信息 (UUID 与游戏名称)
        let profile_text = client
            .get(MC_PROFILE_URL)
            .bearer_auth(&mc_token)
            .send()
            .await
            .map_err(|e| format!("获取游戏档案失败: {}", e))?
            .text()
            .await
            .map_err(|e| format!("读取游戏角色失败: {}", e))?;

        let (name, uuid) = Self::parse_mc_profile_response(&profile_text)?;
        let avatar_url = format!("{}/{}?size=64&overlay", CRAFATAR_AVATAR_BASE, uuid);

        let profile = UserProfile {
            name,
            uuid,
            access_token: mc_token,
            avatar_url,
            auth_type: "msa".to_string(),
            auth_server: None,
        };

        let _ = crate::config::ConfigManager::save_profile(Some(&profile));

        Ok(profile)
    }

    /// 3. 外置登录 (Authlib-Injector / LittleSkin 等 Yggdrasil 验证服务器)
    pub async fn login_yggdrasil(
        api_root: &str,
        username: &str,
        password: &str,
    ) -> Result<UserProfile, String> {
        let raw_root = if api_root.trim().is_empty() {
            DEFAULT_YGGDRASIL_API_ROOT
        } else {
            api_root.trim()
        };
        let trimmed_root = raw_root.trim_end_matches('/');

        let auth_url = format!("{}/authserver/authenticate", trimmed_root);
        let req_body = YggdrasilAuthRequest {
            agent: YggdrasilAgent {
                name: "Minecraft",
                version: 1,
            },
            username,
            password,
            request_user: true,
        };

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| e.to_string())?;

        let resp = client
            .post(&auth_url)
            .json(&req_body)
            .send()
            .await
            .map_err(|e| format!("连接外置验证服务器失败: {}", e))?;

        let text = resp
            .text()
            .await
            .map_err(|e| format!("读取外置登录响应失败: {}", e))?;

        let (token, name, uuid) = Self::parse_yggdrasil_response(&text, username)?;
        let avatar_url = format!("{}/{}?size=64&overlay", CRAFATAR_AVATAR_BASE, uuid);

        let profile = UserProfile {
            name,
            uuid,
            access_token: token,
            avatar_url,
            auth_type: "yggdrasil".to_string(),
            auth_server: Some(trimmed_root.to_string()),
        };

        let _ = crate::config::ConfigManager::save_profile(Some(&profile));

        Ok(profile)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_device_code_response_success() {
        let json = r#"{
            "user_code": "ABCD-EFGH",
            "device_code": "fake-device-code-123456",
            "verification_uri": "https://www.microsoft.com/link",
            "expires_in": 900,
            "interval": 5
        }"#;

        let parsed = MicrosoftAuth::parse_device_code_response(json).unwrap();
        assert_eq!(parsed.user_code, "ABCD-EFGH");
        assert_eq!(parsed.device_code, "fake-device-code-123456");
        assert_eq!(parsed.verification_uri, "https://www.microsoft.com/link");
        assert_eq!(parsed.expires_in, 900);
        assert_eq!(parsed.interval, 5);
    }

    #[test]
    fn test_parse_token_poll_response_states() {
        // 成功获取 access_token
        let success_json = r#"{"access_token": "mock-ms-access-token-xyz"}"#;
        assert_eq!(
            MicrosoftAuth::parse_token_poll_response(success_json).unwrap(),
            TokenPollState::Success("mock-ms-access-token-xyz".to_string())
        );

        // 用户等待中
        let pending_json = r#"{"error": "authorization_pending"}"#;
        assert_eq!(
            MicrosoftAuth::parse_token_poll_response(pending_json).unwrap(),
            TokenPollState::Pending
        );

        // 用户拒绝或超时
        let declined_json = r#"{"error": "authorization_declined"}"#;
        assert_eq!(
            MicrosoftAuth::parse_token_poll_response(declined_json).unwrap(),
            TokenPollState::DeclinedOrExpired("authorization_declined".to_string())
        );

        let expired_json = r#"{"error": "expired_token"}"#;
        assert_eq!(
            MicrosoftAuth::parse_token_poll_response(expired_json).unwrap(),
            TokenPollState::DeclinedOrExpired("expired_token".to_string())
        );
    }

    #[test]
    fn test_parse_xbl_response_success_and_missing_claims() {
        let valid_json = r#"{
            "IssueInstant": "2026-11-04T00:00:00Z",
            "NotAfter": "2026-11-05T00:00:00Z",
            "Token": "mock-xbl-token-111",
            "DisplayClaims": {
                "xui": [
                    { "uhs": "mock-uhs-222" }
                ]
            }
        }"#;

        let (token, uhs) = MicrosoftAuth::parse_xbl_response(valid_json).unwrap();
        assert_eq!(token, "mock-xbl-token-111");
        assert_eq!(uhs, "mock-uhs-222");

        let missing_token = r#"{"DisplayClaims": {"xui": [{"uhs": "mock-uhs"}]}}"#;
        assert!(MicrosoftAuth::parse_xbl_response(missing_token).is_err());

        let missing_xui = r#"{"Token": "token-only"}"#;
        assert!(MicrosoftAuth::parse_xbl_response(missing_xui).is_err());
    }

    #[test]
    fn test_parse_xsts_response_success() {
        let valid_json = r#"{
            "IssueInstant": "2026-11-04T00:00:00Z",
            "NotAfter": "2026-11-05T00:00:00Z",
            "Token": "mock-xsts-token-333",
            "DisplayClaims": {
                "xui": [{ "uhs": "mock-uhs" }]
            }
        }"#;

        let token = MicrosoftAuth::parse_xsts_response(valid_json).unwrap();
        assert_eq!(token, "mock-xsts-token-333");
    }

    #[test]
    fn test_parse_xsts_response_xerr_friendly_errors() {
        // 2148916238: 未创建 Xbox 档案
        let no_profile_json = r#"{
            "Identity": "0",
            "XErr": 2148916238,
            "Message": "No Xbox account found"
        }"#;
        let err = MicrosoftAuth::parse_xsts_response(no_profile_json).unwrap_err();
        assert!(err.contains("尚未创建 Xbox 角色档案"));
        assert!(err.contains("2148916238"));

        // 2148916233: 未成年需加入家庭组
        let child_json = r#"{"XErr": 2148916233}"#;
        let err = MicrosoftAuth::parse_xsts_response(child_json).unwrap_err();
        assert!(err.contains("未成年人"));

        // 未知错误码带 Message
        let custom_err_json = r#"{"XErr": 9999999999, "Message": "Custom server error"}"#;
        let err = MicrosoftAuth::parse_xsts_response(custom_err_json).unwrap_err();
        assert!(err.contains("Custom server error"));
    }

    #[test]
    fn test_parse_mc_login_and_profile_response() {
        let login_json = r#"{"access_token": "mock-mc-bearer-token", "expires_in": 86400}"#;
        let token = MicrosoftAuth::parse_mc_login_response(login_json).unwrap();
        assert_eq!(token, "mock-mc-bearer-token");

        let profile_json = r#"{
            "id": "4566e69fc90748eeab86458279f21b8b",
            "name": "SteveDeveloper",
            "skins": [],
            "capes": []
        }"#;
        let (name, uuid) = MicrosoftAuth::parse_mc_profile_response(profile_json).unwrap();
        assert_eq!(name, "SteveDeveloper");
        assert_eq!(uuid, "4566e69fc90748eeab86458279f21b8b");
    }

    #[test]
    fn test_parse_yggdrasil_response_success_and_error() {
        let success_json = r#"{
            "accessToken": "mock-ygg-token-999",
            "clientToken": "mock-client-token",
            "selectedProfile": {
                "id": "11112222333344445555666677778888",
                "name": "AlexPlayer"
            }
        }"#;
        let (token, name, uuid) =
            MicrosoftAuth::parse_yggdrasil_response(success_json, "fallback").unwrap();
        assert_eq!(token, "mock-ygg-token-999");
        assert_eq!(name, "AlexPlayer");
        assert_eq!(uuid, "11112222333344445555666677778888");

        let error_json = r#"{
            "error": "ForbiddenOperationException",
            "errorMessage": "Invalid credentials. Invalid username or password."
        }"#;
        let err = MicrosoftAuth::parse_yggdrasil_response(error_json, "fallback").unwrap_err();
        assert!(err.contains("Invalid username or password"));
    }
}
