use crate::config::ConfigManager;
use crate::endpoints;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteVersion {
    pub id: String,
    #[serde(rename = "type")]
    pub version_type: String, // release, snapshot, old_beta, old_alpha
    pub url: String,
    pub time: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
    pub sha1: String,
}

#[derive(Debug, Deserialize)]
struct VersionManifest {
    #[allow(dead_code)]
    latest: LatestVersions,
    versions: Vec<RemoteVersion>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct LatestVersions {
    release: String,
    snapshot: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoaderVersionItem {
    pub version: String,
    pub stable: bool,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FabricMetaSummary {
    pub supported_games: Vec<String>,
    pub loaders: Vec<LoaderVersionItem>,
}

#[derive(Debug, Deserialize)]
struct FabricRawMeta {
    game: Option<Vec<FabricRawGame>>,
    loader: Option<Vec<FabricRawLoader>>,
}

#[derive(Debug, Deserialize)]
struct FabricRawGame {
    version: String,
}

#[derive(Debug, Deserialize)]
struct FabricRawLoader {
    version: String,
    stable: bool,
}

#[derive(Debug, Deserialize)]
struct ForgeRawItem {
    version: String,
}

#[derive(Debug, Deserialize)]
struct NeoForgeRawItem {
    version: String,
}

#[derive(Debug, Deserialize)]
struct OptiFineRawItem {
    mcversion: String,
    #[serde(default)]
    r#type: String,
    #[serde(default)]
    patch: String,
    #[serde(default)]
    filename: String,
    #[serde(default)]
    forge: Option<String>,
}

pub struct DownloadManager;

impl DownloadManager {
    /// 对齐 PCL2 `NetRequestByClientRetry`（ModNet.vb 第 23~88 行）：
    /// 1. 复用带标准 UA/Referer 的全局 `shared_http_client`；
    /// 2. 对 BMCLAPI 自动执行建连节流；
    /// 3. 严格检查 HTTP 状态码（遇到 429 冷却退避重试，非 2xx 绝不当作正文解析）；
    /// 4. 强制执行 `RequireJson` 首尾括号完整性校验（防网关报错页或 GFW 截断）；
    /// 5. 多源轮询 + 最多 3 轮退避重试。
    pub async fn fetch_json_with_retry<T: serde::de::DeserializeOwned>(
        urls: &[String],
    ) -> Result<T, String> {
        let client = endpoints::shared_http_client();
        let mut last_err = String::from("未提供有效的请求地址");

        for attempt in 0..3usize {
            if attempt > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(500 * attempt as u64)).await;
            }
            for url in urls {
                endpoints::pace_mirror_request(url).await;
                match client.get(url).send().await {
                    Ok(resp) => {
                        let status = resp.status();
                        if status.as_u16() == 429 || status.as_u16() == 503 {
                            last_err = format!("服务器繁忙 (HTTP {})", status.as_u16());
                            // 对齐 ModNet.vb 第 73 行：遇到 429 冷却等待后重试
                            tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
                            continue;
                        }
                        if !status.is_success() {
                            last_err = format!("HTTP {} ({})", status, url);
                            continue;
                        }
                        match resp.text().await {
                            Ok(raw_text) => {
                                let trimmed = raw_text.trim();
                                let is_valid_json_shape = (trimmed.starts_with('{')
                                    && trimmed.ends_with('}'))
                                    || (trimmed.starts_with('[') && trimmed.ends_with(']'));
                                if !is_valid_json_shape {
                                    last_err = format!("返回结果并非完整 JSON 格式 ({})", url);
                                    continue;
                                }
                                match serde_json::from_str::<T>(trimmed) {
                                    Ok(parsed) => return Ok(parsed),
                                    Err(e) => {
                                        last_err = format!("解析 JSON 失败: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                last_err = format!("读取响应流失败: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        last_err = format!("网络请求失败: {}", e);
                    }
                }
            }
        }

        Err(last_err)
    }

    /// 从配置的下载源（镜像/官方双源容灾）获取 Minecraft 全量官方版本清单
    pub async fn fetch_version_manifest() -> Result<Vec<RemoteVersion>, String> {
        let prefer_mirror = ConfigManager::load().prefer_mirror;
        let urls: Vec<String> = endpoints::version_manifest_urls(prefer_mirror)
            .into_iter()
            .map(|s| s.to_string())
            .collect();

        let manifest: VersionManifest = Self::fetch_json_with_retry(&urls)
            .await
            .map_err(|e| format!("无法从下载源获取版本列表，请检查网络连接 ({})", e))?;

        Ok(manifest.versions)
    }

    /// 获取 Fabric 整体元数据（支持的游戏版本清单 + 全部 Loader 列表）
    pub async fn fetch_fabric_meta() -> Result<FabricMetaSummary, String> {
        let prefer_mirror = ConfigManager::load().prefer_mirror;
        let urls = endpoints::fabric_versions_urls(prefer_mirror);

        let raw: FabricRawMeta = Self::fetch_json_with_retry(&urls)
            .await
            .map_err(|e| format!("获取 Fabric 版本元数据失败: {}", e))?;

        let supported_games = raw
            .game
            .unwrap_or_default()
            .into_iter()
            .map(|g| g.version)
            .collect();

        let loaders = raw
            .loader
            .unwrap_or_default()
            .into_iter()
            .map(|l| LoaderVersionItem {
                version: l.version,
                stable: l.stable,
                description: Some(if l.stable {
                    "稳定版".into()
                } else {
                    "测试版".into()
                }),
            })
            .collect();

        Ok(FabricMetaSummary {
            supported_games,
            loaders,
        })
    }

    /// 获取指定原版版本的 Forge 列表
    pub async fn fetch_forge_loaders(game_version: &str) -> Result<Vec<LoaderVersionItem>, String> {
        let urls = vec![endpoints::forge_list_url(game_version)];
        if let Ok(items) = Self::fetch_json_with_retry::<Vec<ForgeRawItem>>(&urls).await {
            let mut list: Vec<LoaderVersionItem> = items
                .into_iter()
                .map(|i| LoaderVersionItem {
                    version: i.version,
                    stable: true,
                    description: Some("正式版".into()),
                })
                .collect();
            list.reverse();
            return Ok(list);
        }
        Ok(Vec::new())
    }

    /// 获取指定原版版本的 NeoForge 列表
    pub async fn fetch_neoforge_loaders(
        game_version: &str,
    ) -> Result<Vec<LoaderVersionItem>, String> {
        let urls = vec![endpoints::neoforge_list_url(game_version)];
        if let Ok(items) = Self::fetch_json_with_retry::<Vec<NeoForgeRawItem>>(&urls).await {
            let mut list: Vec<LoaderVersionItem> = items
                .into_iter()
                .map(|i| {
                    let is_beta = i.version.contains("beta");
                    LoaderVersionItem {
                        version: i.version,
                        stable: !is_beta,
                        description: Some(if is_beta {
                            "测试版".into()
                        } else {
                            "稳定版".into()
                        }),
                    }
                })
                .collect();
            list.reverse();
            return Ok(list);
        }
        Ok(Vec::new())
    }

    /// 获取支持 OptiFine 的原版游戏版本列表
    pub async fn fetch_optifine_games() -> Result<Vec<String>, String> {
        let urls = vec![endpoints::optifine_version_list_url()];
        if let Ok(items) = Self::fetch_json_with_retry::<Vec<OptiFineRawItem>>(&urls).await {
            let mut set = std::collections::HashSet::new();
            for item in items {
                set.insert(item.mcversion);
            }
            return Ok(set.into_iter().collect());
        }
        Ok(Vec::new())
    }

    /// 获取指定原版版本的 OptiFine 加载器版本列表
    pub async fn fetch_optifine_loaders(
        game_version: &str,
    ) -> Result<Vec<LoaderVersionItem>, String> {
        let urls = vec![endpoints::optifine_list_url(game_version)];
        if let Ok(items) = Self::fetch_json_with_retry::<Vec<OptiFineRawItem>>(&urls).await {
            let mut list: Vec<LoaderVersionItem> = items
                .into_iter()
                .filter(|i| !i.r#type.is_empty() && !i.patch.is_empty())
                .map(|i| {
                    let stable = !i.patch.to_lowercase().starts_with("pre")
                        && !i.filename.to_lowercase().starts_with("preview");
                    let base_desc = if stable { "正式版" } else { "预览版" };
                    let desc = match i.forge.as_deref() {
                        Some(f) if !f.is_empty() && f != "Forge N/A" => {
                            format!("{} · 兼容 {}", base_desc, f)
                        }
                        _ => base_desc.to_string(),
                    };
                    LoaderVersionItem {
                        version: format!("{}_{}", i.r#type, i.patch),
                        stable,
                        description: Some(desc),
                    }
                })
                .collect();
            // 正式版优先，同类中新版在前
            list.reverse();
            list.sort_by_key(|item| !item.stable);
            return Ok(list);
        }
        Ok(Vec::new())
    }
}
