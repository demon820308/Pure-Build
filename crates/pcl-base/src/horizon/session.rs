use crate::auth::UserProfile;
use serde::{Deserialize, Serialize};

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

/// 从 UserProfile 组装会话 JSON 字节流
/// 契约约束：
/// 1. 只许为拥有正版 Java 版的微软账号启动（auth_type 必须为 "msa"）；
/// 2. user_type 固定为 "msa"；xuid 若无则省略；
/// 3. 只返回内存字节流，严禁提供任何落盘写入方法。
pub fn build_session_bytes(profile: &UserProfile, xuid: Option<&str>) -> Result<Vec<u8>, String> {
    if profile.auth_type != "msa" {
        return Err("Horizon 启动门禁拦截：必须使用拥有正版 Minecraft 的微软账号登录".to_string());
    }
    if profile.access_token.trim().is_empty() {
        return Err("无效的微软账号访问令牌：令牌为空".to_string());
    }

    let session = HorizonSession {
        name: profile.name.clone(),
        uuid: profile.uuid.clone(),
        access_token: profile.access_token.clone(),
        user_type: "msa".to_string(),
        xuid: xuid.map(|s| s.to_string()),
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

        let bytes = build_session_bytes(&profile, Some("253541234567890")).expect("组装应成功");
        let parsed: serde_json::Value =
            serde_json::from_slice(&bytes).expect("生成的会话数据应为有效 JSON");

        assert_eq!(parsed["name"], "Alex");
        assert_eq!(parsed["uuid"], "069a79f444e94726a5befca90e38aaf5");
        assert_eq!(parsed["access_token"], "secret_token_123");
        assert_eq!(parsed["user_type"], "msa");
        assert_eq!(parsed["xuid"], "253541234567890");

        // 验证没有多余字段
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

        let bytes = build_session_bytes(&profile, None).expect("组装应成功");
        let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(parsed["user_type"], "msa");
        assert!(parsed.get("xuid").is_none());

        let obj = parsed.as_object().unwrap();
        assert_eq!(obj.len(), 4);
    }

    #[test]
    fn test_build_session_rejects_non_msa() {
        let offline_profile = UserProfile {
            name: "Steve".to_string(),
            uuid: "123".to_string(),
            access_token: "dummy".to_string(),
            avatar_url: "".to_string(),
            auth_type: "offline".to_string(),
            auth_server: None,
        };

        let res = build_session_bytes(&offline_profile, None);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("必须使用拥有正版 Minecraft 的微软账号"));
    }
}
