use crate::config::ConfigManager;
use crate::installer::merge::LoaderProfileJson;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::Path;
use zip::ZipArchive;

/// 解析 OptiFine Installer JAR，处理补丁安装并生成其 version 配置
pub fn extract_optifine_installer(
    installer_bytes: &[u8],
    vanilla_version: &str,
    loader_ver: &str,
    client_jar_path: &Path,
    libraries_dir: &Path,
    final_target_name: &str,
    original_json_val: &serde_json::Value,
) -> Result<(LoaderProfileJson, Vec<serde_json::Value>), String> {
    let reader = io::Cursor::new(installer_bytes);
    let mut archive =
        ZipArchive::new(reader).map_err(|e| format!("解析 OptiFine Installer 失败: {}", e))?;

    let mut lw_ver_opt = None;
    let mut has_patches = false;

    for i in 0..archive.len() {
        let Ok(mut entry) = archive.by_index(i) else {
            continue;
        };
        let Some(enclosed) = entry.enclosed_name() else {
            continue;
        };
        let name = enclosed.to_string_lossy().to_string();

        if name == "launchwrapper-of.txt" {
            let mut s = String::new();
            if entry.read_to_string(&mut s).is_ok() {
                lw_ver_opt = Some(s.trim().to_string());
            }
        } else if name.ends_with(".patterns") || name.ends_with(".override") {
            has_patches = true;
        }
    }

    if let Some(ref lw_ver) = lw_ver_opt {
        let lw_jar_name = format!("launchwrapper-of-{}.jar", lw_ver);
        for i in 0..archive.len() {
            if let Ok(mut entry) = archive.by_index(i) {
                if let Some(enclosed) = entry.enclosed_name() {
                    if enclosed.to_string_lossy() == lw_jar_name {
                        let lw_dest = libraries_dir
                            .join("optifine")
                            .join("launchwrapper-of")
                            .join(lw_ver)
                            .join(&lw_jar_name);
                        if let Some(parent) = lw_dest.parent() {
                            let _ = fs::create_dir_all(parent);
                        }
                        if let Ok(mut out_f) = File::create(&lw_dest) {
                            let _ = io::copy(&mut entry, &mut out_f);
                        }
                        break;
                    }
                }
            }
        }
    }

    let of_full_ver = format!("{}_{}", vanilla_version, loader_ver);
    let of_lib_dir = libraries_dir
        .join("optifine")
        .join("OptiFine")
        .join(&of_full_ver);
    let _ = fs::create_dir_all(&of_lib_dir);
    let of_lib_jar = of_lib_dir.join(format!("OptiFine-{}.jar", of_full_ver));

    let mut patched_ok = false;
    if has_patches && client_jar_path.is_file() {
        let temp_installer = of_lib_dir.join(format!("OptiFine-{}-installer.jar", of_full_ver));
        if fs::write(&temp_installer, installer_bytes).is_ok() {
            let java_bin = ConfigManager::load()
                .java_path
                .filter(|p| !p.is_empty() && Path::new(p).is_file())
                .or_else(|| {
                    crate::java::JavaScanner::scan_all()
                        .into_iter()
                        .next()
                        .map(|j| j.path)
                });

            if let Some(java_exe) = java_bin {
                if let Ok(out) = std::process::Command::new(&java_exe)
                    .arg("-cp")
                    .arg(&temp_installer)
                    .arg("optifine.Patcher")
                    .arg(client_jar_path)
                    .arg(&temp_installer)
                    .arg(&of_lib_jar)
                    .output()
                {
                    if out.status.success() && of_lib_jar.is_file() {
                        patched_ok = true;
                    }
                }
            }
            let _ = fs::remove_file(&temp_installer);
        }
    }

    if !patched_ok {
        fs::write(&of_lib_jar, installer_bytes)
            .map_err(|e| format!("写入 OptiFine 库文件失败: {}", e))?;
    }

    let mut extra_libs = Vec::new();
    let lw_lib_val = if let Some(lw_ver) = lw_ver_opt {
        serde_json::json!({
            "name": format!("optifine:launchwrapper-of:{}", lw_ver)
        })
    } else {
        let legacy_lw = serde_json::json!({
            "name": "net.minecraft:launchwrapper:1.12"
        });
        extra_libs.push(legacy_lw.clone());
        legacy_lw
    };

    let of_lib_val = serde_json::json!({
        "name": format!("optifine:OptiFine:{}", of_full_ver)
    });

    let legacy_mc_args = original_json_val
        .get("minecraftArguments")
        .and_then(|v| v.as_str())
        .map(|s| format!("{} --tweakClass optifine.OptiFineTweaker", s));

    let arguments = if legacy_mc_args.is_none() {
        Some(serde_json::json!({
            "game": ["--tweakClass", "optifine.OptiFineTweaker"]
        }))
    } else {
        None
    };

    let profile = LoaderProfileJson {
        id: final_target_name.to_string(),
        inherits_from: Some(vanilla_version.to_string()),
        main_class: "net.minecraft.launchwrapper.Launch".to_string(),
        libraries: vec![lw_lib_val, of_lib_val],
        arguments,
        minecraft_arguments: legacy_mc_args,
    };

    Ok((profile, extra_libs))
}

pub async fn install_optifine_loader(
    reporter: &crate::reporter::ReporterHandle,
    client: &reqwest::Client,
    vanilla_version: &str,
    final_target_name: &str,
    loader_version: Option<&str>,
    version_dir: &Path,
    libraries_dir: &Path,
    json_file_path: &Path,
    prefer_mirror: bool,
    target_os: &str,
    target_arch: &str,
    original_json_val: &mut serde_json::Value,
) -> Result<(), String> {
    let loader_ver = loader_version
        .ok_or_else(|| "未指定 OptiFine 版本".to_string())?;
    let (of_type, of_patch) = loader_ver
        .rsplit_once('_')
        .ok_or_else(|| format!("非法的 OptiFine 版本格式: {}", loader_ver))?;

    reporter.emit_download(crate::installer::DownloadProgress {
        version_id: final_target_name.to_string(),
        vanilla_version: Some(vanilla_version.to_string()),
        step: format!("正在下载 OptiFine {} 安装包...", loader_ver),
        percentage: 86.0,
        speed_text: "下载安装包".to_string(),
        is_finished: false,
        error: None,
    });
    let inst_url =
        crate::endpoints::optifine_installer_url(vanilla_version, of_type, of_patch);
    // shortcut: OptiFine installer jar 属于第三方闭源补丁包，未提供官方 SHA-1，维持大小与 ZIP 魔数校验
    let inst_bytes = crate::installer::download::fetch_bytes_with_fallback(client, &inst_url, None)
        .await
        .map_err(|e| format!("下载 OptiFine {} 安装包失败: {}", loader_ver, e))?;

    reporter.emit_download(crate::installer::DownloadProgress {
        version_id: final_target_name.to_string(),
        vanilla_version: Some(vanilla_version.to_string()),
        step: format!("正在解包并合成 OptiFine {}...", loader_ver),
        percentage: 92.0,
        speed_text: "解包合成".to_string(),
        is_finished: false,
        error: None,
    });
    let client_jar_path = version_dir.join(format!("{}.jar", final_target_name));
    let (loader_profile, extra_libs) = extract_optifine_installer(
        &inst_bytes,
        vanilla_version,
        loader_ver,
        &client_jar_path,
        libraries_dir,
        final_target_name,
        original_json_val,
    )?;

    if !extra_libs.is_empty() {
        let of_tasks = crate::installer::libraries::collect_library_tasks(
            &extra_libs,
            libraries_dir,
            None,
            prefer_mirror,
            target_os,
            target_arch,
            None,
        );
        crate::installer::download::execute_batch_downloads(
            reporter,
            final_target_name,
            Some(vanilla_version),
            &format!("正在并发下载 OptiFine {} 依赖库", loader_ver),
            94.0,
            98.0,
            crate::endpoints::DEFAULT_LIBRARY_CONCURRENCY,
            client,
            of_tasks,
        )
        .await?;
    }

    crate::installer::merge::merge_loader_profile_into_version_json(
        original_json_val,
        loader_profile,
        final_target_name,
    );
    let merged_str = serde_json::to_string_pretty(original_json_val)
        .map_err(|e| format!("序列化合并后的版本 JSON 失败: {}", e))?;
    fs::write(json_file_path, merged_str)
        .map_err(|e| format!("写入合并后的版本 JSON 失败: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::installer::merge::merge_loader_profile_into_version_json;
    use std::io::Write;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    #[test]
    fn test_extract_optifine_installer_and_merge() {
        let mut buf = Vec::new();
        {
            let mut zw = ZipWriter::new(io::Cursor::new(&mut buf));
            let opts = SimpleFileOptions::default();
            zw.start_file("launchwrapper-of.txt", opts).unwrap();
            zw.write_all(b"2.3\n").unwrap();
            zw.start_file("launchwrapper-of-2.3.jar", opts).unwrap();
            zw.write_all(b"dummy-lw-jar").unwrap();
            zw.finish().unwrap();
        }

        let temp_dir = std::env::temp_dir().join(format!("pcl_of_test_{}", std::process::id()));
        let libs_dir = temp_dir.join("libraries");
        let client_jar = temp_dir.join("1.20.4.jar");
        let mut orig_json = serde_json::json!({
            "id": "1.20.4",
            "mainClass": "net.minecraft.client.main.Main",
            "libraries": [],
            "arguments": {
                "game": [],
                "jvm": []
            }
        });

        let (profile, extra_libs) = extract_optifine_installer(
            &buf,
            "1.20.4",
            "HD_U_I7",
            &client_jar,
            &libs_dir,
            "1.20.4-OptiFine_HD_U_I7",
            &orig_json,
        )
        .expect("extract optifine installer should succeed");

        assert!(extra_libs.is_empty());
        assert_eq!(profile.main_class, "net.minecraft.launchwrapper.Launch");
        assert!(libs_dir
            .join("optifine")
            .join("launchwrapper-of")
            .join("2.3")
            .join("launchwrapper-of-2.3.jar")
            .is_file());
        assert!(libs_dir
            .join("optifine")
            .join("OptiFine")
            .join("1.20.4_HD_U_I7")
            .join("OptiFine-1.20.4_HD_U_I7.jar")
            .is_file());

        merge_loader_profile_into_version_json(
            &mut orig_json,
            profile,
            "1.20.4-OptiFine_HD_U_I7",
        );

        assert_eq!(orig_json["id"], "1.20.4-OptiFine_HD_U_I7");
        assert_eq!(
            orig_json["mainClass"],
            "net.minecraft.launchwrapper.Launch"
        );
        let game_args = orig_json["arguments"]["game"].as_array().unwrap();
        assert!(game_args.contains(&serde_json::json!("--tweakClass")));
        assert!(game_args.contains(&serde_json::json!("optifine.OptiFineTweaker")));

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
