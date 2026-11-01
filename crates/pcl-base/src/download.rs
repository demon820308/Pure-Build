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
    /// 从配置的下载源（镜像/官方双源容灾）获取 Minecraft 全量官方版本清单
    pub async fn fetch_version_manifest() -> Result<Vec<RemoteVersion>, String> {
        let prefer_mirror = ConfigManager::load().prefer_mirror;
        let urls = endpoints::version_manifest_urls(prefer_mirror);

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| e.to_string())?;

        for url in urls {
            if let Ok(res) = client.get(url).send().await {
                if let Ok(manifest) = res.json::<VersionManifest>().await {
                    return Ok(manifest.versions);
                }
            }
        }

        Err("无法从下载源获取版本列表，请检查网络连接".to_string())
    }

    /// 获取 Fabric 整体元数据（支持的游戏版本清单 + 全部 Loader 列表）
    pub async fn fetch_fabric_meta() -> Result<FabricMetaSummary, String> {
        let prefer_mirror = ConfigManager::load().prefer_mirror;
        let urls = endpoints::fabric_versions_urls(prefer_mirror);

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| e.to_string())?;

        for url in &urls {
            if let Ok(res) = client.get(url).send().await {
                if let Ok(raw) = res.json::<FabricRawMeta>().await {
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

                    return Ok(FabricMetaSummary {
                        supported_games,
                        loaders,
                    });
                }
            }
        }

        Err("获取 Fabric 版本元数据失败".to_string())
    }

    /// 获取指定原版版本的 Forge 列表
    pub async fn fetch_forge_loaders(game_version: &str) -> Result<Vec<LoaderVersionItem>, String> {
        let url = endpoints::forge_list_url(game_version);
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| e.to_string())?;

        if let Ok(res) = client.get(&url).send().await {
            if let Ok(items) = res.json::<Vec<ForgeRawItem>>().await {
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
        }
        Ok(Vec::new())
    }

    /// 获取指定原版版本的 NeoForge 列表
    pub async fn fetch_neoforge_loaders(
        game_version: &str,
    ) -> Result<Vec<LoaderVersionItem>, String> {
        let url = endpoints::neoforge_list_url(game_version);
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| e.to_string())?;

        if let Ok(res) = client.get(&url).send().await {
            if let Ok(items) = res.json::<Vec<NeoForgeRawItem>>().await {
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
        }
        Ok(Vec::new())
    }

    /// 获取支持 OptiFine 的原版游戏版本列表
    pub async fn fetch_optifine_games() -> Result<Vec<String>, String> {
        let url = endpoints::optifine_version_list_url();
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| e.to_string())?;

        if let Ok(res) = client.get(&url).send().await {
            if let Ok(items) = res.json::<Vec<OptiFineRawItem>>().await {
                let mut set = std::collections::HashSet::new();
                for item in items {
                    set.insert(item.mcversion);
                }
                return Ok(set.into_iter().collect());
            }
        }
        Ok(Vec::new())
    }

    /// 获取指定原版版本的 OptiFine 加载器版本列表
    pub async fn fetch_optifine_loaders(
        game_version: &str,
    ) -> Result<Vec<LoaderVersionItem>, String> {
        let url = endpoints::optifine_list_url(game_version);
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| e.to_string())?;

        if let Ok(res) = client.get(&url).send().await {
            if let Ok(items) = res.json::<Vec<OptiFineRawItem>>().await {
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
        }
        Ok(Vec::new())
    }
}
