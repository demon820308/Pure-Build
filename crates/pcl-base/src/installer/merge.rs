use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct LoaderProfileJson {
    pub id: String,
    #[serde(rename = "inheritsFrom")]
    pub inherits_from: Option<String>,
    #[serde(rename = "mainClass")]
    pub main_class: String,
    pub arguments: Option<serde_json::Value>,
    #[serde(rename = "minecraftArguments")]
    pub minecraft_arguments: Option<String>,
    pub libraries: Vec<serde_json::Value>,
}

/// 将 Loader 的 Profile JSON（Fabric / Quilt / Forge / NeoForge）合并进目标版本的 JSON 对象
pub fn merge_loader_profile_into_version_json(
    original_json_val: &mut serde_json::Value,
    profile: LoaderProfileJson,
    final_target_name: &str,
) {
    original_json_val["mainClass"] = serde_json::Value::String(profile.main_class);
    original_json_val["id"] = serde_json::Value::String(final_target_name.to_string());

    if let Some(orig_libs) = original_json_val
        .get_mut("libraries")
        .and_then(|l| l.as_array_mut())
    {
        for l_lib in profile.libraries {
            orig_libs.push(l_lib);
        }
    }

    if let Some(loader_args) = profile.arguments {
        if let Some(loader_jvm) = loader_args.get("jvm").and_then(|j| j.as_array()) {
            if let Some(orig_jvm) = original_json_val
                .get_mut("arguments")
                .and_then(|a| a.get_mut("jvm"))
                .and_then(|j| j.as_array_mut())
            {
                for j_arg in loader_jvm {
                    orig_jvm.push(j_arg.clone());
                }
            }
        }
        if let Some(loader_game) = loader_args.get("game").and_then(|g| g.as_array()) {
            if let Some(orig_game) = original_json_val
                .get_mut("arguments")
                .and_then(|a| a.get_mut("game"))
                .and_then(|g| g.as_array_mut())
            {
                for g_arg in loader_game {
                    orig_game.push(g_arg.clone());
                }
            }
        }
    } else if let Some(mc_args) = profile.minecraft_arguments {
        original_json_val["minecraftArguments"] = serde_json::Value::String(mc_args);
    }
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

pub async fn install_fabric_or_quilt_loader(
    reporter: &crate::reporter::ReporterHandle,
    client: &reqwest::Client,
    vanilla_version: &str,
    final_target_name: &str,
    loader_type: &str,
    loader_version: Option<&str>,
    libraries_dir: &std::path::Path,
    json_file_path: &std::path::Path,
    prefer_mirror: bool,
    target_os: &str,
    target_arch: &str,
    original_json_val: &mut serde_json::Value,
) -> Result<(), String> {
    if loader_type == "fabric" {
        reporter.emit_download(crate::installer::DownloadProgress {
            version_id: final_target_name.to_string(),
            vanilla_version: Some(vanilla_version.to_string()),
            step: "正在查询匹配的 Fabric Loader...".to_string(),
            percentage: 86.0,
            speed_text: "Fabric 元数据".to_string(),
            is_finished: false,
            error: None,
        });
        let loader_urls =
            crate::endpoints::fabric_loader_list_urls(vanilla_version, prefer_mirror);
        let loaders: Vec<FabricLoaderEntry> =
            crate::download::DownloadManager::fetch_json_with_retry(&loader_urls)
                .await
                .map_err(|e| format!("查询 Fabric Loader 清单失败: {}", e))?;

        let chosen_entry = if let Some(req_ver) = loader_version {
            loaders.iter().find(|l| l.loader.version == req_ver)
        } else {
            loaders
                .iter()
                .find(|l| l.loader.stable)
                .or_else(|| loaders.first())
        }
        .ok_or_else(|| format!("未找到匹配的 Fabric Loader 版本 ({:?})", loader_version))?;

        let loader_ver = &chosen_entry.loader.version;
        let profile_urls = crate::endpoints::fabric_profile_json_urls(
            vanilla_version,
            loader_ver,
            prefer_mirror,
        );

        let fabric_profile: LoaderProfileJson =
            crate::download::DownloadManager::fetch_json_with_retry(&profile_urls)
                .await
                .map_err(|e| format!("拉取 Fabric Profile JSON 失败: {}", e))?;

        let fab_tasks = crate::installer::libraries::collect_library_tasks(
            &fabric_profile.libraries,
            libraries_dir,
            None,
            prefer_mirror,
            target_os,
            target_arch,
            Some(crate::endpoints::FABRIC_MAVEN_OFFICIAL),
        );
        crate::installer::download::execute_batch_downloads(
            reporter,
            final_target_name,
            Some(vanilla_version),
            &format!("正在并发下载 Fabric {} 依赖库", loader_ver),
            88.0,
            98.0,
            crate::endpoints::DEFAULT_LIBRARY_CONCURRENCY,
            client,
            fab_tasks,
        )
        .await?;

        merge_loader_profile_into_version_json(
            original_json_val,
            fabric_profile,
            final_target_name,
        );
        let merged_str = serde_json::to_string_pretty(original_json_val)
            .map_err(|e| format!("序列化合并后的版本 JSON 失败: {}", e))?;
        std::fs::write(json_file_path, merged_str)
            .map_err(|e| format!("写入合并后的版本 JSON 失败: {}", e))?;
    } else if loader_type == "quilt" {
        let loader_ver = loader_version
            .ok_or_else(|| "未指定 Quilt Loader 版本".to_string())?;
        let profile_urls = crate::endpoints::quilt_profile_json_urls(
            vanilla_version,
            loader_ver,
            prefer_mirror,
        );
        let quilt_profile: LoaderProfileJson =
            crate::download::DownloadManager::fetch_json_with_retry(&profile_urls)
                .await
                .map_err(|e| format!("拉取 Quilt Profile JSON 失败: {}", e))?;

        let quilt_tasks = crate::installer::libraries::collect_library_tasks(
            &quilt_profile.libraries,
            libraries_dir,
            None,
            prefer_mirror,
            target_os,
            target_arch,
            Some(crate::endpoints::QUILT_MAVEN_OFFICIAL),
        );
        crate::installer::download::execute_batch_downloads(
            reporter,
            final_target_name,
            Some(vanilla_version),
            &format!("正在并发下载 Quilt {} 依赖库", loader_ver),
            88.0,
            98.0,
            crate::endpoints::DEFAULT_LIBRARY_CONCURRENCY,
            client,
            quilt_tasks,
        )
        .await?;

        merge_loader_profile_into_version_json(
            original_json_val,
            quilt_profile,
            final_target_name,
        );
        let merged_str = serde_json::to_string_pretty(original_json_val)
            .map_err(|e| format!("序列化合并后的版本 JSON 失败: {}", e))?;
        std::fs::write(json_file_path, merged_str)
            .map_err(|e| format!("写入合并后的版本 JSON 失败: {}", e))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_loader_profile_fabric_and_arguments() {
        let mut orig_json = serde_json::json!({
            "id": "1.20.1",
            "mainClass": "net.minecraft.client.main.Main",
            "libraries": [
                { "name": "net.minecraft:client:1.20.1" }
            ],
            "arguments": {
                "game": ["--username", "${auth_player_name}"],
                "jvm": ["-Djava.library.path=${natives_directory}"]
            }
        });

        let fabric_profile_json = r#"{
            "id": "fabric-loader-0.15.11-1.20.1",
            "mainClass": "net.fabricmc.loader.impl.launch.knot.KnotClient",
            "arguments": {
                "game": ["--fabricGame"],
                "jvm": ["-Dfabric.dli.main=true"]
            },
            "libraries": [
                { "name": "net.fabricmc:fabric-loader:0.15.11" },
                { "name": "net.fabricmc:intermediary:1.20.1" }
            ]
        }"#;

        let profile: LoaderProfileJson = serde_json::from_str(fabric_profile_json).unwrap();
        merge_loader_profile_into_version_json(
            &mut orig_json,
            profile,
            "1.20.1-Fabric 0.15.11",
        );

        assert_eq!(orig_json["id"], "1.20.1-Fabric 0.15.11");
        assert_eq!(
            orig_json["mainClass"],
            "net.fabricmc.loader.impl.launch.knot.KnotClient"
        );
        let libs = orig_json["libraries"].as_array().unwrap();
        assert_eq!(libs.len(), 3);
        assert_eq!(libs[1]["name"], "net.fabricmc:fabric-loader:0.15.11");
        assert_eq!(libs[2]["name"], "net.fabricmc:intermediary:1.20.1");

        let game_args = orig_json["arguments"]["game"].as_array().unwrap();
        assert_eq!(game_args.len(), 3);
        assert_eq!(game_args[2], "--fabricGame");

        let jvm_args = orig_json["arguments"]["jvm"].as_array().unwrap();
        assert_eq!(jvm_args.len(), 2);
        assert_eq!(jvm_args[1], "-Dfabric.dli.main=true");
    }

    #[test]
    fn test_merge_loader_profile_legacy_arguments() {
        let mut orig_json = serde_json::json!({
            "id": "1.7.10",
            "mainClass": "net.minecraft.client.main.Main",
            "libraries": []
        });

        let profile = LoaderProfileJson {
            id: "1.7.10-Forge".to_string(),
            inherits_from: None,
            main_class: "net.minecraft.launchwrapper.Launch".to_string(),
            arguments: None,
            minecraft_arguments: Some("--tweakClass cpw.mods.fml.common.launcher.FMLTweaker".to_string()),
            libraries: vec![],
        };

        merge_loader_profile_into_version_json(
            &mut orig_json,
            profile,
            "1.7.10-Forge10.13.4",
        );

        assert_eq!(orig_json["id"], "1.7.10-Forge10.13.4");
        assert_eq!(orig_json["mainClass"], "net.minecraft.launchwrapper.Launch");
        assert_eq!(
            orig_json["minecraftArguments"],
            "--tweakClass cpw.mods.fml.common.launcher.FMLTweaker"
        );
    }

    #[test]
    fn test_merge_loader_profile_empty_libraries_preserve_original() {
        let mut orig_json = serde_json::json!({
            "id": "1.16.5",
            "mainClass": "net.minecraft.client.main.Main",
            "libraries": [
                { "name": "org.lwjgl:lwjgl:3.2.2" }
            ]
        });

        let profile = LoaderProfileJson {
            id: "1.16.5-custom".to_string(),
            inherits_from: Some("1.16.5".to_string()),
            main_class: "custom.Main".to_string(),
            arguments: None,
            minecraft_arguments: None,
            libraries: vec![],
        };

        merge_loader_profile_into_version_json(&mut orig_json, profile, "1.16.5-custom");
        assert_eq!(orig_json["id"], "1.16.5-custom");
        assert_eq!(orig_json["mainClass"], "custom.Main");
        let libs = orig_json["libraries"].as_array().unwrap();
        assert_eq!(libs.len(), 1);
        assert_eq!(libs[0]["name"], "org.lwjgl:lwjgl:3.2.2");
    }

    #[test]
    fn test_merge_loader_profile_append_jvm_without_existing_arguments() {
        let mut orig_json = serde_json::json!({
            "id": "1.12.2",
            "mainClass": "net.minecraft.client.main.Main"
        });

        let profile = LoaderProfileJson {
            id: "1.12.2-Forge".to_string(),
            inherits_from: None,
            main_class: "net.minecraft.launchwrapper.Launch".to_string(),
            arguments: Some(serde_json::json!({
                "game": ["--tweakClass", "net.minecraftforge.fml.common.launcher.FMLTweaker"]
            })),
            minecraft_arguments: None,
            libraries: vec![],
        };

        merge_loader_profile_into_version_json(&mut orig_json, profile, "1.12.2-Forge");
        assert_eq!(orig_json["id"], "1.12.2-Forge");
        assert_eq!(orig_json["mainClass"], "net.minecraft.launchwrapper.Launch");
    }
}
