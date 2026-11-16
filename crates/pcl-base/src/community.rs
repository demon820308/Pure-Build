use crate::config::ConfigManager;
use crate::endpoints;
use crate::installer::DownloadProgress;
use crate::mods::ModScanner;
use crate::reporter::ReporterHandle;
use crate::resource::ResourceScanner;
use crate::version::{is_version_isolated, resolve_game_dir};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use ts_rs::TS;

const MODRINTH_MANIFEST_FILE: &str = ".pcl_modrinth_installed.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
pub struct InstalledCommunitySnapshot {
    pub effective_dir: String,
    pub is_isolated: bool,
    pub project_ids: Vec<String>,
    pub mod_ids: Vec<String>,
    pub file_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModrinthProject {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub categories: Vec<String>,
    pub downloads: u64,
    pub follows: u64,
    pub icon_url: Option<String>,
    pub author: String,
    pub versions: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ModrinthSearchResponse {
    hits: Vec<ModrinthHit>,
}

#[derive(Debug, Deserialize)]
struct ModrinthHit {
    project_id: String,
    slug: String,
    title: String,
    description: String,
    categories: Option<Vec<String>>,
    downloads: Option<u64>,
    follows: Option<u64>,
    icon_url: Option<String>,
    author: Option<String>,
    versions: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct ModrinthVersion {
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    version_number: String,
    files: Vec<ModrinthFile>,
}

#[derive(Debug, Deserialize)]
struct ModrinthFile {
    url: String,
    filename: String,
    primary: bool,
    size: u64,
    hashes: Option<HashMap<String, String>>,
}

pub struct CommunityManager;

impl CommunityManager {
    /// 搜索 Modrinth 社区资源 (Mods, Shaders, Resourcepacks)，支持 MCIM 国内镜像与官方源自动故障转移（对齐 PCL2 DlModRequest）
    pub async fn search_resources(
        query: &str,
        project_type: &str,
        game_version: Option<&str>,
        loader: Option<&str>,
    ) -> Result<Vec<ModrinthProject>, String> {
        let client = endpoints::shared_http_client();
        let prefer_mirror = ConfigManager::load().prefer_mirror;

        // 构造 facets 过滤条件: [["project_type:<type>"], ["versions:<mc_ver>"], ["categories:<loader>"]]
        let mut facets_arr = vec![format!("[\"project_type:{}\"]", project_type)];

        if let Some(gv) = game_version {
            if !gv.is_empty() {
                facets_arr.push(format!("[\"versions:{}\"]", gv));
            }
        }

        if let Some(ld) = loader {
            if !ld.is_empty() && ld.to_lowercase() != "all" && ld.to_lowercase() != "vanilla" {
                facets_arr.push(format!("[\"categories:{}\"]", ld.to_lowercase()));
            }
        }

        let facets_param = format!("[{}]", facets_arr.join(","));
        let search_urls = endpoints::resolve_modrinth_api_urls("search", prefer_mirror);

        let mut last_err = String::from("请求 Modrinth API 失败");
        let mut search_data_opt: Option<ModrinthSearchResponse> = None;

        for url in &search_urls {
            let mut req = client
                .get(url)
                .query(&[("limit", "25"), ("facets", &facets_param)]);

            if !query.trim().is_empty() {
                req = req.query(&[("query", query.trim())]);
            } else {
                req = req.query(&[("index", "downloads")]);
            }

            match req.send().await {
                Ok(resp) if resp.status().is_success() => {
                    match resp.json::<ModrinthSearchResponse>().await {
                        Ok(data) => {
                            search_data_opt = Some(data);
                            break;
                        }
                        Err(e) => last_err = format!("解析 Modrinth 数据失败: {}", e),
                    }
                }
                Ok(resp) => last_err = format!("Modrinth API HTTP {}", resp.status()),
                Err(e) => last_err = format!("请求 Modrinth API 失败: {}", e),
            }
        }

        let search_data = search_data_opt.ok_or(last_err)?;

        let results = search_data
            .hits
            .into_iter()
            .map(|h| ModrinthProject {
                project_id: h.project_id,
                slug: h.slug,
                title: h.title,
                description: h.description,
                categories: h.categories.unwrap_or_default(),
                downloads: h.downloads.unwrap_or(0),
                follows: h.follows.unwrap_or(0),
                icon_url: h.icon_url,
                author: h.author.unwrap_or_else(|| "Unknown".to_string()),
                versions: h.versions.unwrap_or_default(),
            })
            .collect();

        Ok(results)
    }

    /// 下载并安装指定的 Modrinth 资源到指定文件夹（支持 MCIM 镜像加速 + 临时文件原子校验）
    pub async fn install_resource(
        reporter: ReporterHandle,
        game_dir_str: String,
        version_path_str: Option<String>,
        project_id: String,
        project_type: String,
        game_version: Option<String>,
        loader: Option<String>,
    ) -> Result<String, String> {
        let emit_progress =
            |step: &str, pct: f32, speed: &str, finished: bool, err: Option<String>| {
                reporter.emit_download(DownloadProgress {
                    version_id: project_id.clone(),
                    vanilla_version: game_version.clone(),
                    step: step.to_string(),
                    percentage: pct,
                    speed_text: speed.to_string(),
                    is_finished: finished,
                    error: err,
                });
            };

        emit_progress("正在获取该资源的匹配版本...", 10.0, "解析版本", false, None);

        let client = endpoints::shared_http_client();
        let prefer_mirror = ConfigManager::load().prefer_mirror;

        let ver_urls = endpoints::resolve_modrinth_api_urls(
            &format!("project/{}/version", project_id),
            prefer_mirror,
        );
        let mut query_params: Vec<(&str, String)> = Vec::new();

        if let Some(ref gv) = game_version {
            if !gv.is_empty() {
                query_params.push(("game_versions", format!("[\"{}\"]", gv)));
            }
        }

        if let Some(ref ld) = loader {
            let l_lower = ld.to_lowercase();
            if !l_lower.is_empty() && l_lower != "all" && l_lower != "vanilla" {
                query_params.push(("loaders", format!("[\"{}\"]", l_lower)));
            }
        }

        let mut versions: Vec<ModrinthVersion> = Vec::new();
        let mut last_ver_err = String::from("查询版本失败");

        for ver_url in &ver_urls {
            match client.get(ver_url).query(&query_params).send().await {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(parsed) = resp.json::<Vec<ModrinthVersion>>().await {
                        versions = parsed;
                        break;
                    }
                }
                Ok(resp) => last_ver_err = format!("HTTP {}", resp.status()),
                Err(e) => last_ver_err = format!("查询版本失败: {}", e),
            }
        }

        if versions.is_empty() {
            for ver_url in &ver_urls {
                if let Ok(fallback_resp) = client.get(ver_url).send().await {
                    if fallback_resp.status().is_success() {
                        if let Ok(fallback_vers) =
                            fallback_resp.json::<Vec<ModrinthVersion>>().await
                        {
                            versions = fallback_vers;
                            break;
                        }
                    }
                }
            }
        }

        if versions.is_empty() {
            let err = format!(
                "未找到与当前游戏版本或加载器兼容的资源文件 ({})",
                last_ver_err
            );
            emit_progress("安装失败", 0.0, "失败", true, Some(err.clone()));
            return Err(err);
        }

        let target_ver = &versions[0];
        let file_info = target_ver
            .files
            .iter()
            .find(|f| f.primary)
            .or_else(|| target_ver.files.first())
            .ok_or_else(|| "该版本没有任何可下载文件".to_string())?;

        // 防御路径逃逸：只取纯文件名组件
        let safe_filename = Path::new(&file_info.filename)
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| "非法的文件名".to_string())?;

        let (base_dir, target_folder, _) = resolve_community_target_folder(
            Path::new(&game_dir_str),
            version_path_str.as_deref(),
            &project_type,
        );

        if !target_folder.exists() {
            fs::create_dir_all(&target_folder)
                .map_err(|e| format!("创建目标目录失败: {}", e))?;
        }

        let dest_path = target_folder.join(safe_filename);

        emit_progress(
            &format!("正在下载 {}...", safe_filename),
            20.0,
            "开始下载",
            false,
            None,
        );

        let candidate_urls = endpoints::resolve_modrinth_file_urls(&file_info.url, prefer_mirror);
        let mut last_emit_pct = 0.0f32;
        let exp_sha1 = file_info
            .hashes
            .as_ref()
            .and_then(|h| h.get("sha1"))
            .map(|s| s.as_str());
        let exp_sha512 = file_info
            .hashes
            .as_ref()
            .and_then(|h| h.get("sha512"))
            .map(|s| s.as_str());

        crate::installer::download::download_file_with_retry_and_hashes(
            client,
            &candidate_urls,
            &dest_path,
            Some(file_info.size),
            exp_sha1,
            exp_sha512,
            |downloaded: u64, total: u64| {
                let total_f = total.max(1) as f32;
                let dl_f = downloaded as f32;
                let pct = 20.0 + (dl_f / total_f) * 78.0;
                if (pct - last_emit_pct).abs() >= 1.0 || downloaded >= total {
                    last_emit_pct = pct;
                    let mb_text = format!(
                        "{:.1}MB / {:.1}MB",
                        dl_f / (1024.0 * 1024.0),
                        total_f / (1024.0 * 1024.0)
                    );
                    emit_progress("正在高速下载中...", pct, &mb_text, false, None);
                }
            },
        )
        .await
        .map_err(|e| {
            emit_progress("安装失败", 0.0, "失败", true, Some(e.clone()));
            e
        })?;

        record_modrinth_install(&base_dir, &project_type, &project_id, safe_filename);

        emit_progress("安装完成！", 100.0, "完成", true, None);
        Ok(format!("已成功安装到 {:?}", dest_path))
    }

    /// 扫描指定 Java 目标版本实际有效目录（兼容共享目录与独立隔离目录）的社区资源安装快照
    pub fn get_installed_snapshot(
        game_dir: &Path,
        version_path: Option<&str>,
        project_type: &str,
    ) -> InstalledCommunitySnapshot {
        let (base_dir, target_folder, is_isolated) =
            resolve_community_target_folder(game_dir, version_path, project_type);

        let mut project_ids = Vec::new();
        let manifest_path = base_dir.join(MODRINTH_MANIFEST_FILE);
        if let Ok(text) = fs::read_to_string(&manifest_path) {
            if let Ok(map) = serde_json::from_str::<HashMap<String, HashMap<String, String>>>(&text)
            {
                if let Some(entries) = map.get(project_type) {
                    for (pid, fname) in entries {
                        if target_folder.join(fname).is_file() {
                            project_ids.push(pid.clone());
                        }
                    }
                }
            }
        }

        let vp_ref = version_path.map(Path::new).unwrap_or(game_dir);
        let mut mod_ids = Vec::new();
        let mut file_names = Vec::new();
        match project_type {
            "mod" => {
                for m in ModScanner::scan_mods(vp_ref, game_dir) {
                    if !m.mod_id.is_empty() {
                        mod_ids.push(m.mod_id);
                    }
                    if !m.name.is_empty() {
                        mod_ids.push(m.name);
                    }
                    file_names.push(m.file_name);
                }
            }
            "shader" => {
                for item in ResourceScanner::scan_folder(vp_ref, game_dir, "shaderpacks") {
                    file_names.push(item.file_name);
                    file_names.push(item.name);
                }
            }
            "resourcepack" => {
                for item in ResourceScanner::scan_folder(vp_ref, game_dir, "resourcepacks") {
                    file_names.push(item.file_name);
                    file_names.push(item.name);
                }
            }
            _ => {}
        }

        InstalledCommunitySnapshot {
            effective_dir: base_dir.to_string_lossy().to_string(),
            is_isolated,
            project_ids,
            mod_ids,
            file_names,
        }
    }
}

fn resolve_community_target_folder(
    game_dir: &Path,
    version_path: Option<&str>,
    project_type: &str,
) -> (PathBuf, PathBuf, bool) {
    let (base_dir, is_isolated) = match version_path.map(str::trim).filter(|s| !s.is_empty()) {
        Some(vp) => (resolve_game_dir(game_dir, Path::new(vp)), is_version_isolated(vp)),
        None => (game_dir.to_path_buf(), false),
    };
    let folder = match project_type {
        "shader" => "shaderpacks",
        "resourcepack" => "resourcepacks",
        _ => "mods",
    };
    let target_folder = base_dir.join(folder);
    (base_dir, target_folder, is_isolated)
}

fn record_modrinth_install(base_dir: &Path, project_type: &str, project_id: &str, filename: &str) {
    let manifest_path = base_dir.join(MODRINTH_MANIFEST_FILE);
    let mut map: HashMap<String, HashMap<String, String>> = fs::read_to_string(&manifest_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    map.entry(project_type.to_string())
        .or_default()
        .insert(project_id.to_string(), filename.to_string());

    if let Ok(json) = serde_json::to_string_pretty(&map) {
        let _ = fs::write(manifest_path, json);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_community_snapshot_shared_vs_isolated_and_file_removal() {
        let base = std::env::temp_dir().join(format!("pcl_comm_snap_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let game_dir = base.join(".minecraft");
        let shared_v1 = game_dir.join("versions").join("1.20.1-Fabric");
        let shared_v2 = game_dir.join("versions").join("1.21.4-Fabric");
        let isolated_v = game_dir.join("versions").join("1.21.1-Isolated");

        fs::create_dir_all(&shared_v1).unwrap();
        fs::create_dir_all(&shared_v2).unwrap();
        fs::create_dir_all(isolated_v.join("saves")).unwrap();
        fs::create_dir_all(game_dir.join("shaderpacks")).unwrap();
        fs::create_dir_all(isolated_v.join("shaderpacks")).unwrap();

        fs::write(game_dir.join("shaderpacks").join("Complementary.zip"), "zip").unwrap();
        record_modrinth_install(&game_dir, "shader", "complementary-id", "Complementary.zip");

        let snap1 = CommunityManager::get_installed_snapshot(
            &game_dir,
            shared_v1.to_str(),
            "shader",
        );
        let snap2 = CommunityManager::get_installed_snapshot(
            &game_dir,
            shared_v2.to_str(),
            "shader",
        );
        assert!(!snap1.is_isolated);
        assert!(!snap2.is_isolated);
        assert_eq!(snap1.effective_dir, snap2.effective_dir);
        assert!(snap1.project_ids.contains(&"complementary-id".to_string()));
        assert!(snap2.project_ids.contains(&"complementary-id".to_string()));

        let snap_iso = CommunityManager::get_installed_snapshot(
            &game_dir,
            isolated_v.to_str(),
            "shader",
        );
        assert!(snap_iso.is_isolated);
        assert!(snap_iso.project_ids.is_empty());

        // 磁盘文件被删除后，即使登记文件仍在也应判定为未安装
        fs::remove_file(game_dir.join("shaderpacks").join("Complementary.zip")).unwrap();
        let snap_after_del = CommunityManager::get_installed_snapshot(
            &game_dir,
            shared_v1.to_str(),
            "shader",
        );
        assert!(snap_after_del.project_ids.is_empty());

        let _ = fs::remove_dir_all(&base);
    }
}
