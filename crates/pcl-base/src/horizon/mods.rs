use crate::endpoints::{HORIZON_MOD_INDEX_CACHE_TTL_SECS, HORIZON_MOD_INDEX_URL};
use crate::horizon::api::{negotiate_client_api, ClientLaunchApi};
use crate::horizon::build::{resolve_build_root, verify_bytes_sha256};
use crate::horizon::instances::{find_instance, instance_dir, validate_instance_name};
use crate::installer::DownloadProgress;
use crate::platform;
use crate::reporter::ReporterHandle;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, SystemTime};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
pub struct HorizonRegistryFile {
    pub name: String,
    pub url: String,
    #[ts(type = "number")]
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
pub struct HorizonRegistryVersion {
    pub version: String,
    pub published: String,
    pub api: String,
    pub game: String,
    pub file: HorizonRegistryFile,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
pub struct HorizonModItem {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub summary: String,
    pub authors: Vec<String>,
    pub icon: Option<String>,
    pub categories: Vec<String>,
    #[ts(type = "number")]
    pub downloads: u64,
    pub latest_version: Option<HorizonRegistryVersion>,
    pub compatible: bool,
    pub compatibility_reason: Option<String>,
    pub installable: bool,
    pub is_installed: bool,
}

#[derive(Debug, Deserialize)]
struct RawRegistryIndex {
    #[serde(default)]
    projects: Vec<RawRegistryProject>,
}

#[derive(Debug, Deserialize)]
struct RawRegistryProject {
    id: String,
    kind: String,
    name: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    authors: Vec<String>,
    #[serde(default)]
    icon: Option<String>,
    #[serde(default)]
    categories: Vec<String>,
    #[serde(default)]
    downloads: u64,
    #[serde(default)]
    versions: Vec<RawRegistryVersion>,
}

#[derive(Debug, Deserialize)]
struct RawRegistryVersion {
    version: String,
    #[serde(default)]
    published: String,
    #[serde(default)]
    api: String,
    #[serde(default)]
    game: String,
    file: RawRegistryFile,
}

#[derive(Debug, Deserialize)]
struct RawRegistryFile {
    name: String,
    url: String,
    #[serde(default)]
    size: u64,
    #[serde(default)]
    sha256: String,
}

fn validate_single_segment(raw: &str, label: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    let mut comps = Path::new(trimmed).components();
    match (comps.next(), comps.next()) {
        (Some(Component::Normal(_)), None) => {}
        _ => return Err(format!("非法的{}: {}", label, raw)),
    }
    if trimmed
        .chars()
        .any(|c| std::path::is_separator(c) || c == '/' || c == '\\')
    {
        return Err(format!("非法的{}: {}", label, raw));
    }
    Ok(trimmed.to_string())
}

fn registry_cache_path(game_dir: &Path) -> PathBuf {
    game_dir
        .join("horizon")
        .join("cache")
        .join("horizon-mods-index.json")
}

/// 根据当前构建协商所得 `ClientLaunchApi` 判断单个条目版本是否兼容（不硬编码具体版本号）
pub fn check_registry_version_compatibility(
    kind: &str,
    ver: &HorizonRegistryVersion,
    api: &ClientLaunchApi,
) -> (bool, Option<String>) {
    if kind == "native-mod" {
        return (
            false,
            Some("需本地 Rust 源码重编，预编译构建暂不支持直接加载".to_string()),
        );
    }
    if kind == "modpack" {
        return (
            false,
            Some("Horizon 整合包 (.hpack) 暂不支持在此直接解包安装".to_string()),
        );
    }

    if !ver.game.is_empty() && ver.game != api.minecraft {
        return (
            false,
            Some(format!(
                "需要 Minecraft {}（当前构建为 {}）",
                ver.game, api.minecraft
            )),
        );
    }

    let Some(content) = api.content.as_ref().and_then(|v| v.as_object()) else {
        return (true, None);
    };

    match kind {
        "mod" => {
            let Some(supported) = content
                .get("mod")
                .and_then(|m| m.get("api"))
                .and_then(|a| a.as_array())
            else {
                return (false, Some("当前构建未声明支持内容模组".to_string()));
            };
            let matches = ver.api.is_empty()
                || supported
                    .iter()
                    .filter_map(|v| v.as_str())
                    .any(|s| s == ver.api);
            if matches {
                (true, None)
            } else {
                (false, Some(format!("需要 Mod API {}", ver.api)))
            }
        }
        "shader" => {
            let Some(supported) = content
                .get("shader")
                .and_then(|s| s.get("api"))
                .and_then(|a| a.as_array())
            else {
                return (false, Some("当前构建未声明支持原生光影".to_string()));
            };
            let matches = ver.api.is_empty()
                || supported
                    .iter()
                    .filter_map(|v| v.as_str())
                    .any(|s| s == ver.api);
            if matches {
                (true, None)
            } else {
                (false, Some(format!("需要 Shader API {}", ver.api)))
            }
        }
        "resourcepack" => {
            let Some(fmt_arr) = content
                .get("resourcepack")
                .and_then(|r| r.get("format"))
                .and_then(|f| f.as_array())
            else {
                return (false, Some("当前构建未声明支持资源包".to_string()));
            };
            let major = fmt_arr
                .first()
                .and_then(|v| v.as_u64())
                .map(|n| n.to_string());
            let matches = ver.api.is_empty()
                || major.as_deref() == Some(ver.api.as_str())
                || ver
                    .api
                    .split('.')
                    .next()
                    .map(|p| Some(p) == major.as_deref())
                    .unwrap_or(false);
            if matches {
                (true, None)
            } else {
                (false, Some(format!("需要资源包格式 {}", ver.api)))
            }
        }
        _ => (false, Some(format!("暂不支持的类型: {}", kind))),
    }
}

/// 解析官方索引 JSON，并结合可选的构建 `ClientLaunchApi` 计算兼容性与可安装性
pub fn parse_horizon_mod_index(
    json_str: &str,
    client_api: Option<&ClientLaunchApi>,
) -> Result<Vec<HorizonModItem>, String> {
    let raw: RawRegistryIndex = serde_json::from_str(json_str)
        .map_err(|e| format!("解析 Horizon 官方模组索引失败: {}", e))?;

    let mut items = Vec::with_capacity(raw.projects.len());
    for proj in raw.projects {
        let versions: Vec<HorizonRegistryVersion> = proj
            .versions
            .into_iter()
            .map(|v| HorizonRegistryVersion {
                version: v.version,
                published: v.published,
                api: v.api,
                game: v.game,
                file: HorizonRegistryFile {
                    name: v.file.name,
                    url: v.file.url,
                    size: v.file.size,
                    sha256: v.file.sha256,
                },
            })
            .collect();

        let (selected, compatible, reason) = if let Some(api) = client_api {
            let matching = versions
                .iter()
                .find(|v| check_registry_version_compatibility(&proj.kind, v, api).0)
                .cloned();
            if let Some(ver) = matching {
                (Some(ver), true, None)
            } else if let Some(first) = versions.first().cloned() {
                let (ok, r) = check_registry_version_compatibility(&proj.kind, &first, api);
                (Some(first), ok, r)
            } else {
                (None, false, Some("无可用版本文件".to_string()))
            }
        } else {
            let first = versions.first().cloned();
            let (ok, r) = match proj.kind.as_str() {
                "native-mod" => (
                    false,
                    Some("需本地 Rust 源码重编，预编译构建暂不支持直接加载".to_string()),
                ),
                "modpack" => (
                    false,
                    Some("Horizon 整合包 (.hpack) 暂不支持在此直接解包安装".to_string()),
                ),
                "mod" | "shader" | "resourcepack" => (first.is_some(), None),
                _ => (false, Some("暂不支持该类型".to_string())),
            };
            (first, ok, r)
        };

        let installable = matches!(proj.kind.as_str(), "mod" | "shader" | "resourcepack")
            && selected
                .as_ref()
                .map(|v| !v.file.url.is_empty() && !v.file.sha256.is_empty())
                .unwrap_or(false);

        items.push(HorizonModItem {
            id: proj.id,
            kind: proj.kind,
            name: proj.name,
            summary: proj.summary,
            authors: proj.authors,
            icon: proj.icon,
            categories: proj.categories,
            downloads: proj.downloads,
            latest_version: selected,
            compatible,
            compatibility_reason: reason,
            installable,
            is_installed: false,
        });
    }

    items.sort_by(|a, b| b.downloads.cmp(&a.downloads));
    Ok(items)
}

/// 根据目标 Horizon 实例目录标记各项是否已在磁盘安装
pub fn mark_installed_items_in_instance(items: &mut [HorizonModItem], inst_dir: &Path) {
    for item in items.iter_mut() {
        let installed = match item.kind.as_str() {
            "mod" => validate_single_segment(&item.id, "模组 ID")
                .map(|clean_id| {
                    inst_dir
                        .join("horizon-mods")
                        .join(format!("{}.hmod", clean_id))
                        .is_file()
                })
                .unwrap_or(false),
            "shader" => item
                .latest_version
                .as_ref()
                .and_then(|v| validate_single_segment(&v.file.name, "文件名").ok())
                .map(|file_name| inst_dir.join("shaderpacks").join(file_name).is_file())
                .unwrap_or(false),
            "resourcepack" => item
                .latest_version
                .as_ref()
                .and_then(|v| validate_single_segment(&v.file.name, "文件名").ok())
                .map(|file_name| inst_dir.join("resourcepacks").join(file_name).is_file())
                .unwrap_or(false),
            _ => false,
        };
        item.is_installed = installed;
    }
}

/// 按类型与关键词过滤索引结果
pub fn filter_horizon_mod_items(
    items: Vec<HorizonModItem>,
    kind_filter: Option<&str>,
    query: Option<&str>,
    compatible_only: bool,
) -> Vec<HorizonModItem> {
    let q = query.unwrap_or("").trim().to_lowercase();
    items
        .into_iter()
        .filter(|item| {
            match kind_filter.map(str::trim).filter(|s| !s.is_empty()) {
                None | Some("all_usable") => {
                    matches!(item.kind.as_str(), "shader" | "mod" | "resourcepack")
                }
                Some(k) => item.kind == k,
            }
        })
        .filter(|item| {
            if !compatible_only || item.kind == "native-mod" {
                true
            } else {
                item.compatible
            }
        })
        .filter(|item| {
            if q.is_empty() {
                return true;
            }
            item.name.to_lowercase().contains(&q)
                || item.id.to_lowercase().contains(&q)
                || item.summary.to_lowercase().contains(&q)
                || item.authors.iter().any(|a| a.to_lowercase().contains(&q))
                || item.categories.iter().any(|c| c.to_lowercase().contains(&q))
        })
        .collect()
}

/// 拉取并缓存 Horizon 官方仓库索引，按协商能力、筛选条件与目标实例已安装状态返回结果
pub async fn fetch_horizon_mod_registry(
    game_dir: &Path,
    build_tag: Option<&str>,
    instance_name: Option<&str>,
    kind_filter: Option<&str>,
    query: Option<&str>,
    compatible_only: bool,
    force_refresh: bool,
) -> Result<Vec<HorizonModItem>, String> {
    let cache_path = registry_cache_path(game_dir);
    let mut json_str: Option<String> = None;

    if !force_refresh && cache_path.is_file() {
        if let Ok(meta) = fs::metadata(&cache_path) {
            if let Ok(modified) = meta.modified() {
                if let Ok(age) = SystemTime::now().duration_since(modified) {
                    if age <= Duration::from_secs(HORIZON_MOD_INDEX_CACHE_TTL_SECS) {
                        json_str = fs::read_to_string(&cache_path).ok();
                    }
                }
            }
        }
    }

    let raw_json = match json_str {
        Some(cached) => cached,
        None => {
            let client = Client::builder()
                .timeout(Duration::from_secs(20))
                .build()
                .map_err(|e| format!("初始化 HTTP 客户端失败: {}", e))?;
            let resp = client
                .get(HORIZON_MOD_INDEX_URL)
                .header("User-Agent", "PureBuildLauncher")
                .send()
                .await
                .map_err(|e| format!("拉取 Horizon 官方仓库索引失败: {}", e))?;
            if !resp.status().is_success() {
                return Err(format!("Horizon 官方仓库返回异常状态: {}", resp.status()));
            }
            let text = resp
                .text()
                .await
                .map_err(|e| format!("读取 Horizon 官方仓库索引失败: {}", e))?;
            // 写缓存前先校验结构合法，避免损坏缓存污染后续读取
            let _: RawRegistryIndex = serde_json::from_str(&text)
                .map_err(|e| format!("解析 Horizon 官方仓库索引失败: {}", e))?;
            if let Some(parent) = cache_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::write(&cache_path, &text);
            text
        }
    };

    let client_api = match build_tag.map(str::trim).filter(|t| !t.is_empty()) {
        Some(tag) => {
            let clean_tag = validate_instance_name(tag)?;
            let build_dir = game_dir.join("horizon").join("builds").join(clean_tag);
            let build_root = resolve_build_root(&build_dir);
            let client_bin = build_root.join(platform::horizon_client_bin_name());
            if client_bin.is_file() {
                negotiate_client_api(&client_bin).ok()
            } else {
                None
            }
        }
        None => None,
    };

    let mut parsed = parse_horizon_mod_index(&raw_json, client_api.as_ref())?;
    if let Some(inst) = instance_name.map(str::trim).filter(|s| !s.is_empty()) {
        if let Ok(clean_inst) = validate_instance_name(inst) {
            if find_instance(game_dir, &clean_inst).is_some() {
                mark_installed_items_in_instance(&mut parsed, &instance_dir(game_dir, &clean_inst));
            }
        }
    }
    Ok(filter_horizon_mod_items(
        parsed,
        kind_filter,
        query,
        compatible_only,
    ))
}

/// 将指定 `.hmod` 在实例根目录 `horizon-mods.txt` 中标记为启用（保留原有条目）
pub fn enable_content_mod_in_file(inst_dir: &Path, mod_id: &str) -> Result<(), String> {
    let clean_id = validate_single_segment(mod_id, "模组 ID")?;
    let cfg_path = inst_dir.join("horizon-mods.txt");
    let key = format!("mod.{}", clean_id);
    let enabled_line = format!("{}:true", key);

    let mut lines: Vec<String> = if cfg_path.is_file() {
        fs::read_to_string(&cfg_path)
            .map_err(|e| format!("读取 horizon-mods.txt 失败: {}", e))?
            .lines()
            .map(|l| l.trim_end_matches('\r').to_string())
            .filter(|l| !l.trim().is_empty())
            .collect()
    } else {
        Vec::new()
    };

    let prefix = format!("{}:", key);
    if let Some(pos) = lines.iter().position(|l| l.starts_with(&prefix)) {
        lines[pos] = enabled_line;
    } else {
        lines.push(enabled_line);
    }

    let mut content = lines.join("\n");
    content.push('\n');
    fs::write(&cfg_path, content).map_err(|e| format!("写入 horizon-mods.txt 失败: {}", e))
}

/// 下载并校验 Horizon 官方仓库扩展文件，安装至指定 Horizon 实例目录
pub async fn install_horizon_registry_item(
    reporter: &ReporterHandle,
    game_dir: &Path,
    instance_name: &str,
    project_id: &str,
    kind: &str,
    file_name: &str,
    download_url: &str,
    expected_sha256: &str,
) -> Result<String, String> {
    let clean_inst = validate_instance_name(instance_name)?;
    let clean_id = validate_single_segment(project_id, "项目 ID")?;
    let clean_file = validate_single_segment(file_name, "文件名")?;

    if kind == "native-mod" {
        return Err("原生重编类 (.hnmod) 需本地 Rust 源码重编，预编译构建暂不支持直接加载".to_string());
    }
    if !matches!(kind, "mod" | "shader" | "resourcepack") {
        return Err(format!("不支持直接安装该扩展类型: {}", kind));
    }
    if find_instance(game_dir, &clean_inst).is_none() {
        return Err(format!("找不到 Horizon 实例「{}」", clean_inst));
    }
    if !download_url.starts_with("https://") {
        return Err("仅允许从 HTTPS 官方源下载扩展文件".to_string());
    }
    let sha_trimmed = expected_sha256.trim();
    if sha_trimmed.len() != 64 || !sha_trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("扩展文件缺少有效的 SHA-256 校验值，已拒绝安装".to_string());
    }

    let inst_dir = instance_dir(game_dir, &clean_inst);
    let target_path = match kind {
        "mod" => inst_dir
            .join("horizon-mods")
            .join(format!("{}.hmod", clean_id)),
        "shader" => inst_dir.join("shaderpacks").join(&clean_file),
        "resourcepack" => inst_dir.join("resourcepacks").join(&clean_file),
        _ => unreachable!(),
    };

    let task_id = format!("horizon-ext-{}", clean_id);
    let emit = |step: &str, pct: f32, finished: bool, err: Option<String>| {
        reporter.emit_download(DownloadProgress {
            version_id: task_id.clone(),
            vanilla_version: None,
            step: step.to_string(),
            percentage: pct,
            speed_text: String::new(),
            is_finished: finished,
            error: err,
        });
    };

    emit(&format!("正在下载 {}...", clean_file), 20.0, false, None);

    let client = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| format!("初始化 HTTP 客户端失败: {}", e))?;
    let resp = client
        .get(download_url)
        .header("User-Agent", "PureBuildLauncher")
        .send()
        .await
        .map_err(|e| {
            let msg = format!("下载失败: {}", e);
            emit("下载失败", 0.0, true, Some(msg.clone()));
            msg
        })?;
    if !resp.status().is_success() {
        let msg = format!("下载服务器返回异常状态: {}", resp.status());
        emit("下载失败", 0.0, true, Some(msg.clone()));
        return Err(msg);
    }

    let bytes = resp.bytes().await.map_err(|e| {
        let msg = format!("读取下载内容失败: {}", e);
        emit("下载失败", 0.0, true, Some(msg.clone()));
        msg
    })?;

    emit("正在校验 SHA-256 完整性...", 75.0, false, None);
    if !verify_bytes_sha256(&bytes, sha_trimmed) {
        let msg = "SHA-256 校验失败，文件可能已损坏或被篡改，已中止安装".to_string();
        emit("校验失败", 0.0, true, Some(msg.clone()));
        return Err(msg);
    }

    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建目标目录失败: {}", e))?;
    }
    fs::write(&target_path, &bytes).map_err(|e| format!("写入目标文件失败: {}", e))?;

    if kind == "mod" {
        enable_content_mod_in_file(&inst_dir, &clean_id)?;
    }

    emit("安装完成", 100.0, true, None);
    Ok(target_path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reporter::NoopProgressReporter;
    use serde_json::json;
    use std::sync::Arc;

    fn sample_api() -> ClientLaunchApi {
        ClientLaunchApi {
            launch_api: "0.1.0".to_string(),
            horizon: "Nightly 2026-11-13".to_string(),
            minecraft: "26.3".to_string(),
            options: vec![],
            exit_codes: vec![0],
            crash_report_line: None,
            content: Some(json!({
                "mod": { "api": ["0.2", "0.3"] },
                "shader": { "api": ["0.1"] },
                "resourcepack": { "format": [97, 1] }
            })),
        }
    }

    #[test]
    fn test_parse_and_filter_horizon_mod_index_by_negotiated_api() {
        let raw = json!({
            "schema": 1,
            "projects": [
                {
                    "id": "aurora",
                    "kind": "shader",
                    "name": "Aurora Shaders",
                    "summary": "Sky and lighting",
                    "downloads": 500,
                    "versions": [{
                        "version": "5.9.3",
                        "published": "2026-11-10",
                        "api": "0.1",
                        "game": "26.3",
                        "file": { "name": "aurora.hshader", "url": "https://example.com/aurora.hshader", "size": 100, "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" }
                    }]
                },
                {
                    "id": "old_shader",
                    "kind": "shader",
                    "name": "Old Shader",
                    "downloads": 100,
                    "versions": [{
                        "version": "1.0.0",
                        "published": "2026-01-01",
                        "api": "9.9",
                        "game": "26.3",
                        "file": { "name": "old.hshader", "url": "https://example.com/old.hshader", "size": 50, "sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb" }
                    }]
                },
                {
                    "id": "additional_lights",
                    "kind": "mod",
                    "name": "Additional Lights",
                    "downloads": 400,
                    "versions": [{
                        "version": "3.1.4",
                        "published": "2026-11-08",
                        "api": "0.2",
                        "game": "26.3",
                        "file": { "name": "additional_lights.hmod", "url": "https://example.com/al.hmod", "size": 200, "sha256": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc" }
                    }]
                },
                {
                    "id": "abelha_skull",
                    "kind": "resourcepack",
                    "name": "Abelha skull",
                    "downloads": 300,
                    "versions": [{
                        "version": "1.0.0",
                        "published": "2026-11-09",
                        "api": "97",
                        "game": "26.3",
                        "file": { "name": "abelha.zip", "url": "https://example.com/abelha.zip", "size": 300, "sha256": "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd" }
                    }]
                },
                {
                    "id": "appleskin",
                    "kind": "native-mod",
                    "name": "AppleSkin",
                    "downloads": 900,
                    "versions": [{
                        "version": "1.0.0",
                        "published": "2026-11-10",
                        "api": "native-1/abc/def",
                        "game": "26.3",
                        "file": { "name": "appleskin.hnmod", "url": "https://example.com/appleskin.hnmod", "size": 400, "sha256": "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee" }
                    }]
                }
            ]
        })
        .to_string();

        let api = sample_api();
        let parsed = parse_horizon_mod_index(&raw, Some(&api)).unwrap();
        let usable = filter_horizon_mod_items(parsed.clone(), Some("all_usable"), None, true);
        let ids: Vec<&str> = usable.iter().map(|i| i.id.as_str()).collect();
        assert_eq!(ids, vec!["aurora", "additional_lights", "abelha_skull"]);
        assert!(usable.iter().all(|i| i.installable));

        let native = filter_horizon_mod_items(parsed, Some("native-mod"), None, true);
        assert_eq!(native.len(), 1);
        assert_eq!(native[0].id, "appleskin");
        assert!(!native[0].installable);
    }

    #[test]
    fn test_mark_installed_items_per_horizon_instance() {
        let base = std::env::temp_dir().join(format!("pcl_hmod_inst_mark_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let inst_a = base.join("inst_a");
        let inst_b = base.join("inst_b");
        fs::create_dir_all(inst_a.join("shaderpacks")).unwrap();
        fs::create_dir_all(inst_a.join("horizon-mods")).unwrap();
        fs::create_dir_all(&inst_b).unwrap();
        fs::write(inst_a.join("shaderpacks").join("aurora.hshader"), "ok").unwrap();
        fs::write(inst_a.join("horizon-mods").join("additional_lights.hmod"), "ok").unwrap();

        let raw = json!({
            "schema": 1,
            "projects": [
                {
                    "id": "aurora",
                    "kind": "shader",
                    "name": "Aurora Shaders",
                    "downloads": 500,
                    "versions": [{
                        "version": "5.9.3",
                        "api": "0.1",
                        "game": "26.3",
                        "file": { "name": "aurora.hshader", "url": "https://example.com/aurora.hshader", "size": 10, "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" }
                    }]
                },
                {
                    "id": "additional_lights",
                    "kind": "mod",
                    "name": "Additional Lights",
                    "downloads": 400,
                    "versions": [{
                        "version": "3.1.4",
                        "api": "0.2",
                        "game": "26.3",
                        "file": { "name": "additional_lights.hmod", "url": "https://example.com/al.hmod", "size": 10, "sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb" }
                    }]
                }
            ]
        })
        .to_string();

        let mut items_a = parse_horizon_mod_index(&raw, None).unwrap();
        mark_installed_items_in_instance(&mut items_a, &inst_a);
        assert!(items_a.iter().all(|i| i.is_installed));

        let mut items_b = parse_horizon_mod_index(&raw, None).unwrap();
        mark_installed_items_in_instance(&mut items_b, &inst_b);
        assert!(items_b.iter().all(|i| !i.is_installed));

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn test_enable_content_mod_in_file_preserves_existing_lines() {
        let base = std::env::temp_dir().join(format!("pcl_hmod_cfg_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();
        fs::write(
            base.join("horizon-mods.txt"),
            "mod.other_mod:false\nmod.additional_lights:false\n",
        )
        .unwrap();

        enable_content_mod_in_file(&base, "additional_lights").unwrap();
        enable_content_mod_in_file(&base, "brand_new").unwrap();

        let content = fs::read_to_string(base.join("horizon-mods.txt")).unwrap();
        assert!(content.contains("mod.other_mod:false"));
        assert!(content.contains("mod.additional_lights:true"));
        assert!(content.contains("mod.brand_new:true"));
        assert!(!content.contains("mod.additional_lights:false"));

        let _ = fs::remove_dir_all(&base);
    }

    #[tokio::test]
    async fn test_install_rejects_native_mod_and_traversal() {
        let base = std::env::temp_dir().join(format!("pcl_hmod_inst_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let reporter: ReporterHandle = Arc::new(NoopProgressReporter);

        let err_native = install_horizon_registry_item(
            &reporter,
            &base,
            "Horizon-nightly",
            "appleskin",
            "native-mod",
            "appleskin.hnmod",
            "https://example.com/a.hnmod",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        )
        .await
        .unwrap_err();
        assert!(err_native.contains("源码重编"));

        let err_trav = install_horizon_registry_item(
            &reporter,
            &base,
            "Horizon-nightly",
            "../evil",
            "shader",
            "ok.hshader",
            "https://example.com/ok.hshader",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        )
        .await
        .unwrap_err();
        assert!(err_trav.contains("非法"));

        let _ = fs::remove_dir_all(&base);
    }
}
