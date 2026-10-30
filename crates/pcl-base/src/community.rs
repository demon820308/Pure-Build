use crate::installer::DownloadProgress;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use tauri::{AppHandle, Emitter};

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
}

pub struct CommunityManager;

impl CommunityManager {
    /// 搜索 Modrinth 社区资源 (Mods, Shaders, Resourcepacks)
    pub async fn search_resources(
        query: &str,
        project_type: &str,
        game_version: Option<&str>,
        loader: Option<&str>,
    ) -> Result<Vec<ModrinthProject>, String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .user_agent("PlainCraftLauncher/2.0 (Vue3+Tauri)")
            .build()
            .map_err(|e| e.to_string())?;

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

        let mut req = client
            .get("https://api.modrinth.com/v2/search")
            .query(&[("limit", "25"), ("facets", &facets_param)]);

        if !query.trim().is_empty() {
            req = req.query(&[("query", query.trim())]);
        } else {
            req = req.query(&[("index", "downloads")]);
        }

        let resp = req
            .send()
            .await
            .map_err(|e| format!("请求 Modrinth API 失败: {}", e))?;

        let search_data: ModrinthSearchResponse = resp
            .json()
            .await
            .map_err(|e| format!("解析 Modrinth 数据失败: {}", e))?;

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

    /// 下载并安装指定的 Modrinth 资源到指定文件夹
    pub async fn install_resource(
        app: AppHandle,
        game_dir_str: String,
        version_path_str: Option<String>,
        project_id: String,
        project_type: String,
        game_version: Option<String>,
        loader: Option<String>,
    ) -> Result<String, String> {
        let emit_progress = |step: &str, pct: f32, speed: &str, finished: bool, err: Option<String>| {
            let _ = app.emit(
                "download_progress",
                DownloadProgress {
                    version_id: project_id.clone(),
                    step: step.to_string(),
                    percentage: pct,
                    speed_text: speed.to_string(),
                    is_finished: finished,
                    error: err,
                },
            );
        };

        emit_progress("正在获取该资源的匹配版本...", 10.0, "解析版本", false, None);

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("PlainCraftLauncher/2.0 (Vue3+Tauri)")
            .build()
            .map_err(|e| e.to_string())?;

        // 构造查询版本列表的请求
        let ver_url = format!("https://api.modrinth.com/v2/project/{}/version", project_id);
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

        let resp = client
            .get(&ver_url)
            .query(&query_params)
            .send()
            .await
            .map_err(|e| format!("查询版本失败: {}", e))?;

        let mut versions: Vec<ModrinthVersion> = resp
            .json()
            .await
            .map_err(|e| format!("解析版本列表失败: {}", e))?;

        // 如果带特定 loader/版本筛选没搜到，再尝试不带参数检索全量版本取第一个兼容项
        if versions.is_empty() {
            let fallback_url = format!("https://api.modrinth.com/v2/project/{}/version", project_id);
            if let Ok(fallback_resp) = client.get(&fallback_url).send().await {
                if let Ok(fallback_vers) = fallback_resp.json::<Vec<ModrinthVersion>>().await {
                    versions = fallback_vers;
                }
            }
        }

        if versions.is_empty() {
            let err = "未找到与当前游戏版本或加载器兼容的资源文件！".to_string();
            emit_progress("安装失败", 0.0, "失败", true, Some(err.clone()));
            return Err(err);
        }

        // 优先选用 primary 文件，否则用第一个文件
        let target_ver = &versions[0];
        let file_info = target_ver
            .files
            .iter()
            .find(|f| f.primary)
            .or_else(|| target_ver.files.first())
            .ok_or_else(|| "该版本没有任何可下载文件".to_string())?;

        // 决定落盘目标文件夹
        let base_dir = if let Some(vp) = version_path_str {
            let v_path = Path::new(&vp);
            if v_path.join("mods").is_dir() && project_type == "mod" {
                v_path.to_path_buf()
            } else {
                Path::new(&game_dir_str).to_path_buf()
            }
        } else {
            Path::new(&game_dir_str).to_path_buf()
        };

        let target_folder = match project_type.as_str() {
            "shader" => base_dir.join("shaderpacks"),
            "resourcepack" => base_dir.join("resourcepacks"),
            _ => base_dir.join("mods"),
        };

        if !target_folder.exists() {
            fs::create_dir_all(&target_folder)
                .map_err(|e| format!("创建目标目录失败: {}", e))?;
        }

        let dest_path = target_folder.join(&file_info.filename);

        emit_progress(
            &format!("正在下载 {}...", file_info.filename),
            20.0,
            "开始下载",
            false,
            None,
        );

        let mut file_resp = client
            .get(&file_info.url)
            .send()
            .await
            .map_err(|e| format!("下载文件请求失败: {}", e))?;

        let mut dest_file = File::create(&dest_path).map_err(|e| e.to_string())?;
        let total_size = file_info.size as f32;
        let mut downloaded: f32 = 0.0;

        while let Some(chunk) = file_resp.chunk().await.map_err(|e| e.to_string())? {
            dest_file.write_all(&chunk).map_err(|e| e.to_string())?;
            downloaded += chunk.len() as f32;

            let pct = 20.0 + (downloaded / total_size) * 78.0;
            let mb_text = format!(
                "{:.1}MB / {:.1}MB",
                downloaded / (1024.0 * 1024.0),
                total_size / (1024.0 * 1024.0)
            );
            emit_progress("正在高速下载中...", pct, &mb_text, false, None);
        }

        emit_progress("安装完成！", 100.0, "完成", true, None);
        Ok(format!("已成功安装到 {:?}", dest_path))
    }
}
