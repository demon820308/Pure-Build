use crate::auth::UserProfile;
use crate::endpoints::DEFAULT_PLAYER_NAME;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const OFFLINE_LOCAL_DUMMY_TOKEN: &str = "offline_local_test";

/// Horizon 游戏启动会话数据结构（对应 Launch API 契约中的 session JSON）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HorizonSession {
    pub name: String,
    pub uuid: String,
    pub access_token: String,
    pub user_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xuid: Option<String>,
}

fn offline_uuid_for_player(name: &str) -> String {
    let mut hash = Sha256::digest(format!("OfflinePlayer:{}", name).as_bytes());
    hash[6] = (hash[6] & 0x0f) | 0x30;
    hash[8] = (hash[8] & 0x3f) | 0x80;
    hash[..16].iter().map(|b| format!("{:02x}", b)).collect()
}

/// 组装 Horizon 会话 JSON 字节流（只返回内存字节，不落盘）。
/// `require_msa` 为 true 时强制要求微软正版账号与非空令牌；为 false 时允许离线/非正版账号本地测试。
pub fn build_session_bytes(
    profile: Option<&UserProfile>,
    fallback_player_name: &str,
    xuid: Option<&str>,
    require_msa: bool,
) -> Result<Vec<u8>, String> {
    if require_msa {
        let p = profile.ok_or_else(|| "Horizon 需要正版微软账号，请先登录后再启动".to_string())?;
        if p.auth_type != "msa" {
            return Err("Horizon 启动门禁拦截：必须使用拥有正版 Minecraft 的微软账号登录".to_string());
        }
        if p.access_token.trim().is_empty() {
            return Err("无效的微软账号访问令牌：令牌为空".to_string());
        }
        let session = HorizonSession {
            name: p.name.clone(),
            uuid: p.uuid.clone(),
            access_token: p.access_token.clone(),
            user_type: "msa".to_string(),
            xuid: xuid.map(|s| s.to_string()),
        };
        return serde_json::to_vec(&session)
            .map_err(|e| format!("序列化 Horizon 会话数据失败: {}", e));
    }

    if let Some(p) = profile {
        if p.auth_type == "msa" && !p.access_token.trim().is_empty() {
            let session = HorizonSession {
                name: p.name.clone(),
                uuid: p.uuid.clone(),
                access_token: p.access_token.clone(),
                user_type: "msa".to_string(),
                xuid: xuid.map(|s| s.to_string()),
            };
            return serde_json::to_vec(&session)
                .map_err(|e| format!("序列化 Horizon 会话数据失败: {}", e));
        }
    }

    let name = profile
        .map(|p| p.name.trim())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            let trimmed = fallback_player_name.trim();
            (!trimmed.is_empty()).then_some(trimmed)
        })
        .unwrap_or(DEFAULT_PLAYER_NAME)
        .to_string();

    let uuid = profile
        .map(|p| p.uuid.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| offline_uuid_for_player(&name));

    let access_token = profile
        .map(|p| p.access_token.trim())
        .filter(|s| !s.is_empty())
        .unwrap_or(OFFLINE_LOCAL_DUMMY_TOKEN)
        .to_string();

    // Launch API 会话契约要求 user_type 为 "msa"，本地测试沿用该值配合占位令牌
    let session = HorizonSession {
        name,
        uuid,
        access_token,
        user_type: "msa".to_string(),
        xuid: None,
    };

    serde_json::to_vec(&session).map_err(|e| format!("序列化 Horizon 会话数据失败: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_session_bytes_msa_success() {
        let profile = UserProfile {
            name: "Alex".to_string(),
            uuid: "069a79f444e94726a5befca90e38aaf5".to_string(),
            access_token: "secret_token_123".to_string(),
            avatar_url: "".to_string(),
            auth_type: "msa".to_string(),
            auth_server: None,
        };

        let bytes =
            build_session_bytes(Some(&profile), "Steve", Some("253541234567890"), true)
                .expect("组装应成功");
        let parsed: serde_json::Value =
            serde_json::from_slice(&bytes).expect("生成的会话数据应为有效 JSON");

        assert_eq!(parsed["name"], "Alex");
        assert_eq!(parsed["uuid"], "069a79f444e94726a5befca90e38aaf5");
        assert_eq!(parsed["access_token"], "secret_token_123");
        assert_eq!(parsed["user_type"], "msa");
        assert_eq!(parsed["xuid"], "253541234567890");

        let obj = parsed.as_object().unwrap();
        assert_eq!(obj.len(), 5);
    }

    #[test]
    fn test_build_session_bytes_without_xuid() {
        let profile = UserProfile {
            name: "Steve".to_string(),
            uuid: "11111111222233334444555555555555".to_string(),
            access_token: "secret_token_456".to_string(),
            avatar_url: "".to_string(),
            auth_type: "msa".to_string(),
            auth_server: None,
        };

        let bytes = build_session_bytes(Some(&profile), "Steve", None, true).expect("组装应成功");
        let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(parsed["user_type"], "msa");
        assert!(parsed.get("xuid").is_none());

        let obj = parsed.as_object().unwrap();
        assert_eq!(obj.len(), 4);
    }

    #[test]
    fn test_build_session_rejects_non_msa_when_required() {
        let offline_profile = UserProfile {
            name: "Steve".to_string(),
            uuid: "123".to_string(),
            access_token: "dummy".to_string(),
            avatar_url: "".to_string(),
            auth_type: "offline".to_string(),
            auth_server: None,
        };

        let res = build_session_bytes(Some(&offline_profile), "Steve", None, true);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("必须使用拥有正版 Minecraft 的微软账号"));

        let missing = build_session_bytes(None, "Steve", None, true);
        assert!(missing.is_err());
        assert!(missing.unwrap_err().contains("正版微软账号"));
    }

    #[test]
    fn test_build_session_allows_local_offline_when_gate_disabled() {
        let bytes = build_session_bytes(None, "LocalTester", None, false).expect("关闭门禁应放行");
        let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(parsed["name"], "LocalTester");
        assert_eq!(parsed["uuid"].as_str().unwrap().len(), 32);
        assert_eq!(parsed["access_token"], OFFLINE_LOCAL_DUMMY_TOKEN);
        assert_eq!(parsed["user_type"], "msa");
        assert!(parsed.get("xuid").is_none());

        let offline_profile = UserProfile {
            name: "YggUser".to_string(),
            uuid: "abcdef0123456789abcdef0123456789".to_string(),
            access_token: "".to_string(),
            avatar_url: "".to_string(),
            auth_type: "yggdrasil".to_string(),
            auth_server: Some("https://example.com".to_string()),
        };
        let bytes_ygg =
            build_session_bytes(Some(&offline_profile), "Fallback", None, false).unwrap();
        let parsed_ygg: serde_json::Value = serde_json::from_slice(&bytes_ygg).unwrap();
        assert_eq!(parsed_ygg["name"], "YggUser");
        assert_eq!(parsed_ygg["uuid"], "abcdef0123456789abcdef0123456789");
        assert_eq!(parsed_ygg["access_token"], OFFLINE_LOCAL_DUMMY_TOKEN);
    }
}
