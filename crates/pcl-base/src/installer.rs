use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub version_id: String,
    pub step: String,
    pub percentage: f32,
    pub speed_text: String,
    pub is_finished: bool,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VersionDetailJson {
    downloads: Option<VersionDownloads>,
}

#[derive(Debug, Deserialize)]
struct VersionDownloads {
    client: Option<DownloadArtifact>,
    server: Option<DownloadArtifact>,
}

#[derive(Debug, Deserialize)]
struct DownloadArtifact {
    url: String,
    size: u64,
}

#[derive(Debug, Deserialize)]
struct FabricLoaderEntry {
    loader: FabricLoaderInfo,
}

#[derive(Debug, Deserialize)]
struct FabricLoaderInfo {
    version: String,
    stable: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct FabricProfileJson {
    id: String,
    #[serde(rename = "inheritsFrom")]
    inherits_from: Option<String>,
    #[serde(rename = "mainClass")]
    main_class: String,
    arguments: Option<serde_json::Value>,
    libraries: Vec<serde_json::Value>,
}

pub struct VersionInstaller;

impl VersionInstaller {
    fn maven_to_path(name: &str) -> (PathBuf, String) {
        let parts: Vec<&str> = name.split(':').collect();
        if parts.len() < 3 {
            return (PathBuf::from(name), name.to_string());
        }
        let group = parts[0].replace('.', std::path::MAIN_SEPARATOR_STR);
        let artifact = parts[1];
        let version = parts[2];
        let classifier = if parts.len() > 3 {
            format!("-{}", parts[3])
        } else {
            "".to_string()
        };
        let file_name = if classifier.is_empty() {
            format!("{}-{}.jar", artifact, version)
        } else {
            format!("{}-{}{}.jar", artifact, version, classifier)
        };
        let rel_path = PathBuf::from(group)
            .join(artifact)
            .join(version)
            .join(&file_name);
        let url_subpath = format!(
            "{}/{}/{}/{}",
            parts[0].replace('.', "/"),
            artifact,
            version,
            file_name
        );
        (rel_path, url_subpath)
    }

    async fn fetch_bytes(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, String> {
        let resp = client.get(url).send().await.map_err(|e| e.to_string())?;
        if resp.status().is_success() {
            let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
            Ok(bytes.to_vec())
        } else {
            Err(format!("HTTP {}", resp.status()))
        }
    }

    async fn download_library_file(
        client: &reqwest::Client,
        libraries_dir: &Path,
        rel_path: &Path,
        primary_url: &str,
        backup_url: Option<&str>,
    ) -> Result<(), String> {
        let target_path = libraries_dir.join(rel_path);
        if target_path.exists()
            && fs::metadata(&target_path)
                .map(|m| m.len() > 0)
                .unwrap_or(false)
        {
            return Ok(());
        }
        if let Some(parent) = target_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let bytes = match Self::fetch_bytes(client, primary_url).await {
            Ok(b) => b,
            Err(e) => {
                if let Some(backup) = backup_url {
                    Self::fetch_bytes(client, backup)
                        .await
                        .map_err(|e2| format!("下载失败: {} / {}", e, e2))?
                } else {
                    return Err(format!("下载失败: {}", e));
                }
            }
        };

        let mut file = File::create(&target_path).map_err(|e| e.to_string())?;
        file.write_all(&bytes).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 完整安装指定版本的原版核心及可选 Loader (支持独立命名、合并支持库与派发进度)
    pub async fn install_version(
        app: AppHandle,
        game_dir_str: String,
        vanilla_version: String,
        target_name: Option<String>,
        meta_url: String,
        loader_type: Option<String>,
        loader_version: Option<String>,
    ) -> Result<(), String> {
        let final_target_name = target_name.unwrap_or_else(|| vanilla_version.clone());
        let game_dir = Path::new(&game_dir_str);
        let version_dir = game_dir.join("versions").join(&final_target_name);

        if !version_dir.exists() {
            fs::create_dir_all(&version_dir).map_err(|e| format!("创建版本目录失败: {}", e))?;
        }

        let emit_progress = |step: &str, pct: f32, speed: &str, finished: bool, err: Option<String>| {
            let _ = app.emit(
                "download_progress",
                DownloadProgress {
                    version_id: final_target_name.clone(),
                    step: step.to_string(),
                    percentage: pct,
                    speed_text: speed.to_string(),
                    is_finished: finished,
                    error: err,
                },
            );
        };

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| e.to_string())?;

        // 1. 获取并保存版本元数据 JSON
        emit_progress("正在获取原版元数据 JSON...", 10.0, "拉取清单", false, None);

        let accelerated_meta_url = meta_url.replace(
            "https://piston-meta.mojang.com",
            "https://bmclapi2.bangbang93.com",
        );

        let json_resp = client
            .get(&accelerated_meta_url)
            .send()
            .await
            .map_err(|e| format!("无法下载版本 JSON: {}", e))?;

        let json_text = json_resp
            .text()
            .await
            .map_err(|e| format!("读取版本 JSON 失败: {}", e))?;

        let json_file_path = version_dir.join(format!("{}.json", final_target_name));
        let mut original_json_val: serde_json::Value = serde_json::from_str(&json_text)
            .map_err(|e| format!("解析版本 JSON 结构失败: {}", e))?;

        // 写入初始版本 id
        original_json_val["id"] = serde_json::Value::String(final_target_name.clone());
        fs::write(
            &json_file_path,
            serde_json::to_string_pretty(&original_json_val).unwrap_or(json_text.clone()),
        )
        .map_err(|e| format!("写入 JSON 失败: {}", e))?;

        let ver_detail: VersionDetailJson = serde_json::from_str(&json_text)
            .map_err(|e| format!("解析版本 JSON 结构失败: {}", e))?;

        // 2. 下载 client.jar
        emit_progress("正在准备下载游戏核心 (client.jar)...", 20.0, "解析地址", false, None);

        if let Some(downloads) = ver_detail.downloads {
            if let Some(client_art) = downloads.client {
                let accelerated_client_url = client_art.url.replace(
                    "https://piston-data.mojang.com",
                    "https://bmclapi2.bangbang93.com",
                );

                let mut jar_resp = client
                    .get(&accelerated_client_url)
                    .send()
                    .await
                    .map_err(|e| format!("下载 client.jar 请求失败: {}", e))?;

                let jar_file_path = version_dir.join(format!("{}.jar", final_target_name));
                let mut file = File::create(&jar_file_path).map_err(|e| e.to_string())?;

                let total_size = client_art.size as f32;
                let mut downloaded: f32 = 0.0;

                while let Some(chunk) = jar_resp.chunk().await.map_err(|e| e.to_string())? {
                    file.write_all(&chunk).map_err(|e| e.to_string())?;
                    downloaded += chunk.len() as f32;

                    let pct = 20.0 + (downloaded / total_size) * 50.0; // 20% ~ 70%
                    let mb_text = format!(
                        "{:.1}MB / {:.1}MB",
                        downloaded / (1024.0 * 1024.0),
                        total_size / (1024.0 * 1024.0)
                    );
                    emit_progress("正在高速下载核心 JAR 文件...", pct, &mb_text, false, None);
                }
            }
        }

        // 3. 下载原版运行支持库 (Libraries)
        let libraries_dir = game_dir.join("libraries");
        if !libraries_dir.exists() {
            let _ = fs::create_dir_all(&libraries_dir);
        }

        if let Some(orig_libs) = original_json_val.get("libraries").and_then(|l| l.as_array()) {
            emit_progress("正在校验并补全原版运行支持库...", 72.0, "支持库校验", false, None);
            for lib in orig_libs {
                if let Some(artifact) = lib.get("downloads").and_then(|d| d.get("artifact")) {
                    if let (Some(path_str), Some(url_str)) = (
                        artifact.get("path").and_then(|p| p.as_str()),
                        artifact.get("url").and_then(|u| u.as_str()),
                    ) {
                        let rel_path =
                            PathBuf::from(path_str.replace('/', std::path::MAIN_SEPARATOR_STR));
                        let accelerated_url = url_str.replace(
                            "https://libraries.minecraft.net",
                            "https://bmclapi2.bangbang93.com/maven",
                        );
                        let _ = Self::download_library_file(
                            &client,
                            &libraries_dir,
                            &rel_path,
                            &accelerated_url,
                            Some(url_str),
                        )
                        .await;
                    }
                } else if let Some(name) = lib.get("name").and_then(|n| n.as_str()) {
                    let (rel_path, url_subpath) = Self::maven_to_path(name);
                    let bmclapi_url =
                        format!("https://bmclapi2.bangbang93.com/maven/{}", url_subpath);
                    let official_url = format!("https://libraries.minecraft.net/{}", url_subpath);
                    let _ = Self::download_library_file(
                        &client,
                        &libraries_dir,
                        &rel_path,
                        &bmclapi_url,
                        Some(&official_url),
                    )
                    .await;
                }
            }
        }

        // 4. 可选：自动安装 Fabric Loader 并下载其专属支持库
        let is_fabric = loader_type.as_deref() == Some("fabric");
        if is_fabric {
            emit_progress("正在查询匹配的 Fabric Loader...", 82.0, "Fabric 元数据", false, None);
            let fabric_meta_url = format!(
                "https://bmclapi2.bangbang93.com/fabric-meta/v2/versions/loader/{}",
                vanilla_version
            );

            if let Ok(resp) = client.get(&fabric_meta_url).send().await {
                if let Ok(loaders) = resp.json::<Vec<FabricLoaderEntry>>().await {
                    // 若用户指定了版本则精确匹配，否则挑最稳定的 Loader 版本
                    let chosen_entry = if let Some(ref req_ver) = loader_version {
                        loaders.iter().find(|l| &l.loader.version == req_ver)
                    } else {
                        loaders
                            .iter()
                            .find(|l| l.loader.stable)
                            .or_else(|| loaders.first())
                    };

                    if let Some(entry) = chosen_entry {
                        let loader_ver = &entry.loader.version;
                        emit_progress(
                            &format!("正在合成 Fabric {} 加载器并下载依赖库...", loader_ver),
                            88.0,
                            "下载 Fabric 依赖",
                            false,
                            None,
                        );

                        let profile_url = format!(
                            "https://bmclapi2.bangbang93.com/fabric-meta/v2/versions/loader/{}/{}/profile/json",
                            vanilla_version, loader_ver
                        );

                        if let Ok(profile_resp) = client.get(&profile_url).send().await {
                            if let Ok(fabric_profile) =
                                profile_resp.json::<FabricProfileJson>().await
                            {
                                // 4.1 下载 Fabric 所有的专属支持库 (确保 KnotClient 所在的 fabric-loader-xxx.jar 100% 写盘)
                                for lib_val in &fabric_profile.libraries {
                                    if let Some(name) =
                                        lib_val.get("name").and_then(|n| n.as_str())
                                    {
                                        let (rel_path, url_subpath) = Self::maven_to_path(name);
                                        let raw_url = lib_val
                                            .get("url")
                                            .and_then(|u| u.as_str())
                                            .unwrap_or("https://maven.fabricmc.net/");
                                        let raw_url_trimmed = raw_url.trim_end_matches('/');
                                        let official_url =
                                            format!("{}/{}", raw_url_trimmed, url_subpath);
                                        let bmclapi_url = format!(
                                            "https://bmclapi2.bangbang93.com/maven/{}",
                                            url_subpath
                                        );

                                        let _ = Self::download_library_file(
                                            &client,
                                            &libraries_dir,
                                            &rel_path,
                                            &bmclapi_url,
                                            Some(&official_url),
                                        )
                                        .await;
                                    }
                                }

                                // 4.2 将 Fabric 主类与 Fabric libraries 合并进原版 JSON
                                original_json_val["mainClass"] =
                                    serde_json::Value::String(fabric_profile.main_class);
                                original_json_val["id"] =
                                    serde_json::Value::String(final_target_name.clone());

                                if let Some(orig_libs) = original_json_val
                                    .get_mut("libraries")
                                    .and_then(|l| l.as_array_mut())
                                {
                                    for f_lib in fabric_profile.libraries {
                                        orig_libs.push(f_lib);
                                    }
                                }

                                // 如果 Fabric 含有 JVM/Game arguments，合并进去
                                if let Some(fab_args) = fabric_profile.arguments {
                                    if let Some(fab_jvm) =
                                        fab_args.get("jvm").and_then(|j| j.as_array())
                                    {
                                        if let Some(orig_jvm) = original_json_val
                                            .get_mut("arguments")
                                            .and_then(|a| a.get_mut("jvm"))
                                            .and_then(|j| j.as_array_mut())
                                        {
                                            for j_arg in fab_jvm {
                                                orig_jvm.push(j_arg.clone());
                                            }
                                        }
                                    }
                                }

                                // 写入完整合并后的 JSON
                                let merged_json_str =
                                    serde_json::to_string_pretty(&original_json_val)
                                        .unwrap_or_default();
                                if !merged_json_str.is_empty() {
                                    let _ = fs::write(&json_file_path, merged_json_str);
                                }
                            }
                        }
                    }
                }
            }
        }

        // 5. 安装完成
        emit_progress("安装完成！已成功加入版本列表", 100.0, "完成", true, None);
        Ok(())
    }

    /// 下载并配置 Minecraft 官方服务端
    pub async fn install_server(
        app: AppHandle,
        target_dir: String,
        version_id: String,
        meta_url: String,
    ) -> Result<(), String> {
        let emit_progress = |step: &str, pct: f32, speed: &str, finished: bool, err: Option<String>| {
            let _ = app.emit(
                "download_progress",
                DownloadProgress {
                    version_id: format!("{}-server", version_id),
                    step: step.to_string(),
                    percentage: pct,
                    speed_text: speed.to_string(),
                    is_finished: finished,
                    error: err,
                },
            );
        };

        emit_progress("正在获取服务端元数据...", 10.0, "", false, None);

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| e.to_string())?;

        let mirror_meta_url = meta_url.replace(
            "https://piston-meta.mojang.com",
            "https://bmclapi2.bangbang93.com",
        );
        let meta_resp = match client.get(&mirror_meta_url).send().await {
            Ok(r) if r.status().is_success() => r,
            _ => client
                .get(&meta_url)
                .send()
                .await
                .map_err(|e| format!("获取版本元数据失败: {}", e))?,
        };

        let detail = meta_resp
            .json::<VersionDetailJson>()
            .await
            .map_err(|e| format!("解析版本元数据失败: {}", e))?;

        let server_art = match detail.downloads.and_then(|d| d.server) {
            Some(s) => s,
            None => {
                let err = format!("Mojang 官方没有为 Minecraft {} 提供服务端文件", version_id);
                emit_progress(&err, 0.0, "", true, Some(err.clone()));
                return Err(err);
            }
        };

        emit_progress("正在下载官方服务端核心...", 30.0, "下载中", false, None);

        let mirror_server_url = server_art.url.replace(
            "https://piston-data.mojang.com",
            "https://bmclapi2.bangbang93.com",
        );

        let target_path = PathBuf::from(&target_dir);
        if !target_path.exists() {
            fs::create_dir_all(&target_path).map_err(|e| e.to_string())?;
        }

        let server_jar_path = target_path.join(format!("{}-server.jar", version_id));
        let bytes = match Self::fetch_bytes(&client, &mirror_server_url).await {
            Ok(b) => b,
            Err(_) => Self::fetch_bytes(&client, &server_art.url).await?,
        };

        fs::write(&server_jar_path, &bytes).map_err(|e| format!("写入服务端 jar 失败: {}", e))?;

        emit_progress("正在生成开服脚本与配置文件...", 90.0, "", false, None);

        // 生成启动脚本
        let bat_content = format!(
            "@echo off\r\ntitle Minecraft {0} 原版服务端\r\necho 正在启动 Minecraft {0} 服务端...\r\necho 如果提示需要同意 EULA，请确认已同意并检查 eula.txt 中的 eula=true\r\necho --------------------------------------------------\r\n\"java\" -server -XX:+UseG1GC -Xmx4096M -Xms1024M -jar {0}-server.jar nogui\r\necho --------------------------------------------------\r\necho 服务端已退出。\r\npause\r\n",
            version_id
        );
        let _ = fs::write(target_path.join("启动服务端.bat"), bat_content);

        // 生成 eula.txt
        let eula_content = "#By changing the setting below to TRUE you are indicating your agreement to our EULA (https://aka.ms/MinecraftEULA).\r\neula=true\r\n";
        let _ = fs::write(target_path.join("eula.txt"), eula_content);

        emit_progress("服务端与启动脚本已准备就绪！", 100.0, "完成", true, None);
        Ok(())
    }
}
