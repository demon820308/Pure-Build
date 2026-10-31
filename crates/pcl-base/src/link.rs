use crate::endpoints::{
    BMCLAPI_FABRIC_META, BMCLAPI_VERSION_MANIFEST_V2, FABRIC_META_OFFICIAL, MC_LAN_BIND_ADDR,
    MC_LAN_MULTICAST_ADDR, MC_LAN_MULTICAST_PORT, MC_LAN_SEND_BIND_ADDR, MODRINTH_API_BASE,
    MOJANG_VERSION_MANIFEST_V2, QUILT_META_OFFICIAL,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::net::Ipv4Addr;
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LanRoomInfo {
    pub motd: String,
    pub host_ip: String,
    pub port: u16,
    pub server_address: String,
    pub invite_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointPingResult {
    pub name: String,
    pub category: String,
    pub latency_ms: Option<u64>,
    pub status_ok: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheCleanResult {
    pub removed_items: usize,
    pub freed_bytes: u64,
}

pub struct LinkManager;

impl LinkManager {
    /// 解析标准 Minecraft 局域网广播报文：`[MOTD]房间名[/MOTD][AD]端口号[/AD]`
    pub fn parse_lan_broadcast(packet: &str, sender_ip: &str) -> Option<LanRoomInfo> {
        let motd_start = packet.find("[MOTD]")? + 6;
        let motd_end = packet[motd_start..].find("[/MOTD]")? + motd_start;
        let motd = packet[motd_start..motd_end].trim().to_string();

        let ad_start = packet.find("[AD]")? + 4;
        let ad_end = packet[ad_start..].find("[/AD]")? + ad_start;
        let port_str = packet[ad_start..ad_end].trim();
        let port: u16 = port_str.parse().ok()?;

        let server_address = format!("{}:{}", sender_ip, port);
        let invite_code = Self::encode_invite_code(sender_ip, port);

        Some(LanRoomInfo {
            motd,
            host_ip: sender_ip.to_string(),
            port,
            server_address,
            invite_code,
        })
    }

    /// 将 IP 与端口编码为 PBL 联机邀请码（如 `PBL-LINK-C0A8010A-63DD`）
    pub fn encode_invite_code(ip: &str, port: u16) -> String {
        if let Ok(ipv4) = ip.parse::<Ipv4Addr>() {
            let octets = ipv4.octets();
            let ip_hex = format!(
                "{:02X}{:02X}{:02X}{:02X}",
                octets[0], octets[1], octets[2], octets[3]
            );
            format!("PBL-LINK-{}-{:04X}", ip_hex, port)
        } else {
            let hex_bytes: String = ip.bytes().map(|b| format!("{:02X}", b)).collect();
            format!("PBL-HOST-{}-{:04X}", hex_bytes, port)
        }
    }

    /// 将 PBL / PCL2 联机邀请码反解为 `(host, port)`
    pub fn decode_invite_code(code: &str) -> Result<(String, u16), String> {
        let trimmed = code.trim().to_uppercase();
        if let Some(rest) = trimmed
            .strip_prefix("PBL-LINK-")
            .or_else(|| trimmed.strip_prefix("PCL2-LINK-"))
        {
            let parts: Vec<&str> = rest.split('-').collect();
            if parts.len() != 2 || parts[0].len() != 8 {
                return Err("邀请码格式无效（应为 PBL-LINK-XXXXXXXX-XXXX）".to_string());
            }
            let ip_u32 = u32::from_str_radix(parts[0], 16)
                .map_err(|_| "邀请码 IP 段解析失败".to_string())?;
            let port = u16::from_str_radix(parts[1], 16)
                .map_err(|_| "邀请码端口段解析失败".to_string())?;
            let ipv4 = Ipv4Addr::from(ip_u32);
            Ok((ipv4.to_string(), port))
        } else if let Some(rest) = trimmed
            .strip_prefix("PBL-HOST-")
            .or_else(|| trimmed.strip_prefix("PCL2-HOST-"))
        {
            let parts: Vec<&str> = rest.split('-').collect();
            if parts.len() != 2 || parts[0].len() % 2 != 0 {
                return Err("自定义域名邀请码格式无效".to_string());
            }
            let mut bytes = Vec::with_capacity(parts[0].len() / 2);
            for i in (0..parts[0].len()).step_by(2) {
                let b = u8::from_str_radix(&parts[0][i..i + 2], 16)
                    .map_err(|_| "域名十六进制解码失败".to_string())?;
                bytes.push(b);
            }
            let host = String::from_utf8(bytes)
                .map_err(|_| "域名字符编码无效".to_string())?
                .to_lowercase();
            let port = u16::from_str_radix(parts[1], 16)
                .map_err(|_| "邀请码端口段解析失败".to_string())?;
            Ok((host, port))
        } else {
            Err("未识别的邀请码前缀（请使用 PBL-LINK-... 格式）".to_string())
        }
    }

    /// 扫描局域网内的 Minecraft 开放世界广播（监听 UDP 多播 `224.0.2.60:4445`）
    pub async fn scan_lan_rooms(timeout_ms: u64) -> Result<Vec<LanRoomInfo>, String> {
        let mut rooms_by_addr: HashMap<String, LanRoomInfo> = HashMap::new();

        // 尝试绑定 4445 端口并加入多播组；若端口已被游戏进程占用则优雅返回空列表
        let Ok(socket) = UdpSocket::bind(MC_LAN_BIND_ADDR).await else {
            return Ok(Vec::new());
        };

        if let Ok(multi_ip) = MC_LAN_MULTICAST_ADDR.parse::<Ipv4Addr>() {
            let _ = socket.join_multicast_v4(multi_ip, Ipv4Addr::UNSPECIFIED);
        }

        let deadline = Instant::now() + Duration::from_millis(timeout_ms.clamp(300, 3000));
        let mut buf = [0u8; 1024];

        while Instant::now() < deadline {
            let remaining = deadline.saturating_duration_since(Instant::now());
            match tokio::time::timeout(remaining, socket.recv_from(&mut buf)).await {
                Ok(Ok((len, addr))) => {
                    let msg = String::from_utf8_lossy(&buf[..len]);
                    let sender_ip = addr.ip().to_string();
                    if let Some(info) = Self::parse_lan_broadcast(&msg, &sender_ip) {
                        rooms_by_addr.insert(info.server_address.clone(), info);
                    }
                }
                _ => break,
            }
        }

        let mut list: Vec<LanRoomInfo> = rooms_by_addr.into_values().collect();
        list.sort_by(|a, b| a.motd.cmp(&b.motd));
        Ok(list)
    }

    /// 向当前局域网/虚拟局域网发送 Minecraft 房间广播包，并返回对应的 PCL2 联机邀请码
    pub async fn broadcast_lan_room(
        motd: &str,
        host_ip: &str,
        port: u16,
    ) -> Result<LanRoomInfo, String> {
        let clean_motd = if motd.trim().is_empty() {
            "PBL 共享世界"
        } else {
            motd.trim()
        };
        let clean_ip = if host_ip.trim().is_empty() {
            "127.0.0.1"
        } else {
            host_ip.trim()
        };

        let payload = format!("[MOTD]{}[/MOTD][AD]{}[/AD]", clean_motd, port);
        let target = format!("{}:{}", MC_LAN_MULTICAST_ADDR, MC_LAN_MULTICAST_PORT);

        if let Ok(socket) = UdpSocket::bind(MC_LAN_SEND_BIND_ADDR).await {
            let _ = socket.set_multicast_ttl_v4(4);
            let _ = socket.send_to(payload.as_bytes(), &target).await;
        }

        let invite_code = Self::encode_invite_code(clean_ip, port);
        Ok(LanRoomInfo {
            motd: clean_motd.to_string(),
            host_ip: clean_ip.to_string(),
            port,
            server_address: format!("{}:{}", clean_ip, port),
            invite_code,
        })
    }

    /// 百宝箱：并发检测各大官方源与镜像源的 HTTP 响应延迟 (ms)
    pub async fn ping_download_endpoints() -> Vec<EndpointPingResult> {
        let targets = vec![
            (
                "Mojang 官方版本源",
                "官方源",
                MOJANG_VERSION_MANIFEST_V2.to_string(),
            ),
            (
                "BMCLAPI 高速镜像源",
                "国内镜像",
                BMCLAPI_VERSION_MANIFEST_V2.to_string(),
            ),
            (
                "Fabric 官方 Meta 源",
                "加载器源",
                format!("{}/v2/versions/game", FABRIC_META_OFFICIAL),
            ),
            (
                "BMCLAPI Fabric 镜像",
                "国内镜像",
                format!("{}/v2/versions/game", BMCLAPI_FABRIC_META),
            ),
            (
                "Quilt 官方 Meta 源",
                "加载器源",
                format!("{}/v3/versions/game", QUILT_META_OFFICIAL),
            ),
            (
                "Modrinth 社区 API",
                "社区生态",
                format!("{}/tag/category", MODRINTH_API_BASE),
            ),
        ];

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(6))
            .build()
            .unwrap_or_default();

        let mut results = Vec::with_capacity(targets.len());
        for (name, category, url) in targets {
            let start = Instant::now();
            let resp = client.head(&url).send().await;
            match resp {
                Ok(r) if r.status().is_success() || r.status().as_u16() == 405 => {
                    results.push(EndpointPingResult {
                        name: name.to_string(),
                        category: category.to_string(),
                        latency_ms: Some(start.elapsed().as_millis() as u64),
                        status_ok: true,
                    });
                }
                _ => {
                    // 部分节点不支持 HEAD，回退轻量 GET
                    let start_get = Instant::now();
                    if let Ok(r2) = client.get(&url).send().await {
                        let ok = r2.status().is_success();
                        results.push(EndpointPingResult {
                            name: name.to_string(),
                            category: category.to_string(),
                            latency_ms: if ok {
                                Some(start_get.elapsed().as_millis() as u64)
                            } else {
                                None
                            },
                            status_ok: ok,
                        });
                    } else {
                        results.push(EndpointPingResult {
                            name: name.to_string(),
                            category: category.to_string(),
                            latency_ms: None,
                            status_ok: false,
                        });
                    }
                }
            }
        }

        results
    }

    /// 百宝箱：清理指定游戏目录下的临时 Natives 解压目录与历史归档日志
    pub fn clean_game_temp_cache(game_dir_str: &str) -> Result<CacheCleanResult, String> {
        let game_dir = Path::new(game_dir_str);
        if !game_dir.is_dir() {
            return Ok(CacheCleanResult {
                removed_items: 0,
                freed_bytes: 0,
            });
        }

        let mut removed_items = 0usize;
        let mut freed_bytes = 0u64;

        // 1. 清理 versions/<id>/<id>-natives 临时解压目录（下次启动时会自动重新解压）
        let versions_dir = game_dir.join("versions");
        if versions_dir.is_dir() {
            if let Ok(entries) = fs::read_dir(&versions_dir) {
                for entry in entries.flatten() {
                    let v_path = entry.path();
                    if v_path.is_dir() {
                        if let Ok(sub_entries) = fs::read_dir(&v_path) {
                            for sub in sub_entries.flatten() {
                                let sub_path = sub.path();
                                let sub_name = sub.file_name().to_string_lossy().to_string();
                                if sub_path.is_dir() && sub_name.ends_with("-natives") {
                                    freed_bytes += Self::dir_size(&sub_path);
                                    if fs::remove_dir_all(&sub_path).is_ok() {
                                        removed_items += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 2. 清理 logs 目录下归档的 .log.gz 历史压缩日志
        let logs_dir = game_dir.join("logs");
        if logs_dir.is_dir() {
            if let Ok(entries) = fs::read_dir(&logs_dir) {
                for entry in entries.flatten() {
                    let l_path = entry.path();
                    let l_name = entry.file_name().to_string_lossy().to_string();
                    if l_path.is_file() && l_name.ends_with(".log.gz") {
                        let sz = fs::metadata(&l_path).map(|m| m.len()).unwrap_or(0);
                        if fs::remove_file(&l_path).is_ok() {
                            freed_bytes += sz;
                            removed_items += 1;
                        }
                    }
                }
            }
        }

        Ok(CacheCleanResult {
            removed_items,
            freed_bytes,
        })
    }

    fn dir_size(dir: &Path) -> u64 {
        let mut total = 0u64;
        for entry in walkdir::WalkDir::new(dir).into_iter().flatten() {
            if entry.path().is_file() {
                if let Ok(meta) = entry.metadata() {
                    total += meta.len();
                }
            }
        }
        total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_lan_broadcast_and_invite_code_roundtrip() {
        let raw_packet = "[MOTD]Steve - 生存世界[/MOTD][AD]25565[/AD]";
        let parsed = LinkManager::parse_lan_broadcast(raw_packet, "192.168.1.10").unwrap();
        assert_eq!(parsed.motd, "Steve - 生存世界");
        assert_eq!(parsed.host_ip, "192.168.1.10");
        assert_eq!(parsed.port, 25565);
        assert_eq!(parsed.invite_code, "PBL-LINK-C0A8010A-63DD");

        let (decoded_ip, decoded_port) =
            LinkManager::decode_invite_code(&parsed.invite_code).unwrap();
        assert_eq!(decoded_ip, "192.168.1.10");
        assert_eq!(decoded_port, 25565);
    }
}
