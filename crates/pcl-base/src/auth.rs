use crate::endpoints::{
    CRAFATAR_AVATAR_BASE, DEFAULT_YGGDRASIL_API_ROOT, MC_LOGIN_XBOX_URL, MC_PROFILE_URL,
    MS_DEFAULT_VERIFICATION_URI, MS_DEVICE_CODE_URL, MS_OAUTH_CLIENT_ID, MS_TOKEN_URL,
    XBL_AUTH_URL, XSTS_AUTH_URL,
};
use serde::{Deserialize, Serialize};

fn default_verification_uri() -> String {
    MS_DEFAULT_VERIFICATION_URI.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCodeResponse {
    pub user_code: String,
    pub device_code: String,
    #[serde(alias = "verification_url", default = "default_verification_uri")]
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

        resp.json::<DeviceCodeResponse>()
            .await
            .map_err(|e| format!("解析设备码响应失败: {}", e))
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

            if let Ok(token_resp) = resp.json::<TokenResponse>().await {
                if let Some(token) = token_resp.access_token {
                    ms_access_token = token;
                    break;
                }
                if let Some(err) = token_resp.error {
                    if err == "authorization_declined"
                        || err == "bad_verification_code"
                        || err == "expired_token"
                    {
                        return Err(format!("登录已取消或超时: {}", err));
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

        let xbl_resp = client
            .post(XBL_AUTH_URL)
            .json(&xbl_req)
            .send()
            .await
            .map_err(|e| format!("Xbox Live 认证失败: {}", e))?
            .json::<XboxLiveAuthResponse>()
            .await
            .map_err(|e| format!("解析 Xbox Live 响应失败: {}", e))?;

        let xbl_token = xbl_resp.token.ok_or("缺失 Xbox Live 凭据")?;
        let userhash = xbl_resp
            .display_claims
            .and_then(|c| c.xui)
            .and_then(|x| x.into_iter().next())
            .and_then(|u| u.uhs)
            .ok_or("缺失 UserHash")?;

        // 4. 交换 XSTS Token
        let xsts_req = XstsAuthRequest {
            properties: XstsAuthProperties {
                sandbox_id: "RETAIL".to_string(),
                user_tokens: vec![xbl_token],
            },
            relying_party: "rp://api.minecraftservices.com/".to_string(),
            token_type: "JWT".to_string(),
        };

        let xsts_resp = client
            .post(XSTS_AUTH_URL)
            .json(&xsts_req)
            .send()
            .await
            .map_err(|e| format!("XSTS 认证失败: {}", e))?
            .json::<XboxLiveAuthResponse>()
            .await
            .map_err(|e| format!("解析 XSTS 响应失败: {}", e))?;

        let xsts_token = xsts_resp.token.ok_or("缺失 XSTS 凭据")?;

        // 5. 换取 Minecraft 访问令牌
        let mc_req = McLoginWithXboxRequest {
            identity_token: format!("XBL3.0 x={};{}", userhash, xsts_token),
        };

        let mc_resp = client
            .post(MC_LOGIN_XBOX_URL)
            .json(&mc_req)
            .send()
            .await
            .map_err(|e| format!("Minecraft 登录接口调用失败: {}", e))?
            .json::<McLoginResponse>()
            .await
            .map_err(|e| format!("解析 Minecraft 凭据失败: {}", e))?;

        let mc_token = mc_resp.access_token.ok_or("未获取到 Minecraft 令牌")?;

        // 6. 获取正版角色信息 (UUID 与游戏名称)
        let profile_resp = client
            .get(MC_PROFILE_URL)
            .bearer_auth(&mc_token)
            .send()
            .await
            .map_err(|e| format!("获取游戏档案失败: {}", e))?
            .json::<McProfileResponse>()
            .await
            .map_err(|e| format!("解析游戏角色失败: {}", e))?;

        let name = profile_resp
            .name
            .unwrap_or_else(|| "MinecraftPlayer".to_string());
        let uuid = profile_resp
            .id
            .unwrap_or_else(|| "00000000000000000000000000000000".to_string());
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

        let ygg_resp: YggdrasilAuthResponse = resp
            .json()
            .await
            .map_err(|e| format!("解析外置登录响应失败: {}", e))?;

        if let Some(err_msg) = ygg_resp.error_message {
            return Err(format!("外置登录认证失败: {}", err_msg));
        }

        let token = ygg_resp.access_token.ok_or("未返回有效的 accessToken")?;
        let chosen_profile = ygg_resp
            .selected_profile
            .or_else(|| ygg_resp.available_profiles.and_then(|mut v| v.pop()))
            .ok_or("该外置账号下尚未创建任何游戏角色")?;

        let name = chosen_profile.name.unwrap_or_else(|| username.to_string());
        let uuid = chosen_profile
            .id
            .unwrap_or_else(|| "00000000000000000000000000000000".to_string());
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
