use super::natives::{check_rules_for_target, extract_native_jar, is_modern_native_library, select_native_classifier};
use super::{ConditionalArgument, LaunchArguments, LaunchArgumentsResult, LaunchVersionJson};
use crate::platform;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// 生成去重用的 package key（不依赖平台分隔符）
pub fn get_package_key(name: &str) -> String {
    let parts: Vec<&str> = name.split(':').collect();
    if parts.len() >= 4 {
        format!("{}:{}:{}", parts[0], parts[1], parts[3])
    } else if parts.len() >= 2 {
        format!("{}:{}", parts[0], parts[1])
    } else {
        name.to_string()
    }
}

/// 将 artifact.path（正斜杠相对路径）按 PathBuf::join 拆解组合（禁止字符串替换拼分隔符）
pub fn artifact_path_to_pathbuf(path_str: &str) -> PathBuf {
    let mut pb = PathBuf::new();
    for seg in path_str.split('/').filter(|s| !s.is_empty() && *s != "..") {
        pb = pb.join(seg);
    }
    pb
}

/// 将 Maven 坐标转换为本地相对路径（全部通过 PathBuf::join 拼接）
pub fn maven_to_path(name: &str) -> PathBuf {
    let parts: Vec<&str> = name.split(':').collect();
    if parts.len() < 3 {
        return PathBuf::from(name);
    }
    let mut rel_path = PathBuf::new();
    for seg in parts[0].split('.') {
        rel_path = rel_path.join(seg);
    }
    let artifact = parts[1];
    let version = parts[2];
    let classifier = if parts.len() > 3 {
        format!("-{}", parts[3])
    } else {
        String::new()
    };

    let file_name = if classifier.is_empty() {
        format!("{}-{}.jar", artifact, version)
    } else {
        format!("{}-{}{}.jar", artifact, version, classifier)
    };

    rel_path.join(artifact).join(version).join(file_name)
}

pub fn replace_vars(template: &str, vars: &HashMap<&str, String>) -> String {
    let mut res = template.to_string();
    for (k, v) in vars {
        let pattern = format!("${{{}}}", k);
        res = res.replace(&pattern, v);
    }
    res
}

/// 解析 JVM 或 Game 参数数组（支持纯字符串及带 rules 的条件参数对象）
pub fn resolve_arguments(
    raw_args: &[serde_json::Value],
    target_os: &str,
    target_arch: &str,
    target_os_version: Option<&str>,
    active_features: &HashMap<String, bool>,
    vars: &HashMap<&str, String>,
) -> Vec<String> {
    let mut out = Vec::new();
    for item in raw_args {
        if let Some(s) = item.as_str() {
            out.push(replace_vars(s, vars));
        } else if item.is_object() {
            if let Ok(cond) = serde_json::from_value::<ConditionalArgument>(item.clone()) {
                if check_rules_for_target(
                    &cond.rules,
                    target_os,
                    target_arch,
                    target_os_version,
                    active_features,
                ) {
                    match cond.value {
                        serde_json::Value::String(s) => {
                            out.push(replace_vars(&s, vars));
                        }
                        serde_json::Value::Array(arr) => {
                            for v in arr {
                                if let Some(s) = v.as_str() {
                                    out.push(replace_vars(s, vars));
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    out
}

/// 读取版本 JSON，若存在 `inheritsFrom` 且未内联父版本支持库，则自动合并父版本 JSON
pub fn load_version_json_with_inheritance(
    game_dir: &Path,
    version_dir: &Path,
    version_id: &str,
) -> Result<LaunchVersionJson, String> {
    let json_path = version_dir.join(format!("{}.json", version_id));
    let json_content = fs::read_to_string(&json_path).map_err(|e| e.to_string())?;
    let mut child: LaunchVersionJson =
        serde_json::from_str(&json_content).map_err(|e| format!("JSON 解析失败: {}", e))?;

    if let Some(ref parent_id) = child.inherits_from.clone() {
        let parent_json_path = game_dir
            .join("versions")
            .join(parent_id)
            .join(format!("{}.json", parent_id));
        if parent_json_path.is_file() {
            if let Ok(parent_content) = fs::read_to_string(&parent_json_path) {
                if let Ok(parent) = serde_json::from_str::<LaunchVersionJson>(&parent_content) {
                    child = merge_inherited_json(parent, child);
                }
            }
        }
    }

    Ok(child)
}

/// 将父版本（原版）与子版本（Forge/Fabric/NeoForge 等）的 LaunchVersionJson 合并
pub fn merge_inherited_json(
    mut parent: LaunchVersionJson,
    child: LaunchVersionJson,
) -> LaunchVersionJson {
    parent.id = child.id;
    if !child.main_class.is_empty() {
        parent.main_class = child.main_class;
    }
    parent.inherits_from = child.inherits_from;
    if child.asset_index.is_some() {
        parent.asset_index = child.asset_index;
    }
    if child.assets.is_some() {
        parent.assets = child.assets;
    }
    if child.minecraft_arguments.is_some() {
        parent.minecraft_arguments = child.minecraft_arguments;
    }
    if child.java_version.is_some() {
        parent.java_version = child.java_version;
    }

    // 子版本的 libraries 追加在后，以便在 get_package_key 去重时覆盖父版本的同名旧版库（如 ASM）
    parent.libraries.extend(child.libraries);

    if let Some(child_args) = child.arguments {
        let mut merged_args = parent.arguments.unwrap_or(LaunchArguments {
            game: None,
            jvm: None,
        });
        if let Some(c_jvm) = child_args.jvm {
            let mut jvm_list = merged_args.jvm.unwrap_or_default();
            jvm_list.extend(c_jvm);
            merged_args.jvm = Some(jvm_list);
        }
        if let Some(c_game) = child_args.game {
            let mut game_list = merged_args.game.unwrap_or_default();
            game_list.extend(c_game);
            merged_args.game = Some(game_list);
        }
        parent.arguments = Some(merged_args);
    }

    parent
}

/// 纯函数式构造 Minecraft 启动所需的所有命令行参数与运行时上下文（完全可离线测试）
pub fn build_launch_arguments(
    ver_json: &LaunchVersionJson,
    game_dir: &Path,
    version_id: &str,
    target_os: &str,
    target_arch: &str,
    target_os_version: Option<&str>,
    player_name: &str,
    max_memory_mb: u32,
    uuid: Option<&str>,
    access_token: Option<&str>,
    window_width: u32,
    window_height: u32,
    extract_natives: bool,
) -> Result<LaunchArgumentsResult, String> {
    let required_java_major = ver_json
        .java_version
        .as_ref()
        .and_then(|j| j.major_version)
        .unwrap_or(8);

    let version_dir = game_dir.join("versions").join(version_id);
    let libraries_dir = game_dir.join("libraries");
    let natives_dir = version_dir.join(format!("{}-natives", version_id));
    if extract_natives && !natives_dir.exists() {
        fs::create_dir_all(&natives_dir).map_err(|e| format!("创建 natives 目录失败: {}", e))?;
    }

    let mut active_features = HashMap::new();
    active_features.insert("is_demo_user".to_string(), false);
    active_features.insert(
        "has_custom_resolution".to_string(),
        window_width > 0 && window_height > 0,
    );

    let mut cp_map: HashMap<String, String> = HashMap::new();
    let mut cp_order: Vec<String> = Vec::new();

    for lib in &ver_json.libraries {
        if !check_rules_for_target(
            &lib.rules,
            target_os,
            target_arch,
            target_os_version,
            &active_features,
        ) {
            continue;
        }

        // 1.1 处理普通 artifact 或 1.19+ 独立 natives artifact
        let has_main_artifact = lib
            .downloads
            .as_ref()
            .and_then(|d| d.artifact.as_ref())
            .is_some()
            || lib.downloads.is_none();

        if has_main_artifact {
            let rel_path = if let Some(path_str) = lib
                .downloads
                .as_ref()
                .and_then(|d| d.artifact.as_ref())
                .and_then(|a| a.path.as_ref())
            {
                artifact_path_to_pathbuf(path_str)
            } else {
                maven_to_path(&lib.name)
            };

            let full_lib_path = libraries_dir.join(&rel_path);
            if full_lib_path.is_file() {
                if extract_natives && is_modern_native_library(lib) {
                    let excludes = lib.extract.as_ref().and_then(|e| e.exclude.as_deref());
                    let _ = extract_native_jar(&full_lib_path, &natives_dir, excludes);
                }

                let pkg_key = get_package_key(&lib.name);
                let full_str = full_lib_path.to_string_lossy().to_string();

                if !cp_map.contains_key(&pkg_key) {
                    cp_order.push(pkg_key.clone());
                }
                cp_map.insert(pkg_key, full_str);
            }
        }

        // 1.2 处理 1.18 及以前通过 natives 映射表 + classifiers 指定的平台原生库
        if let Some((_classifier_key, native_artifact)) =
            select_native_classifier(lib, target_os, target_arch)
        {
            if let Some(ref path_str) = native_artifact.path {
                let native_rel_path = artifact_path_to_pathbuf(path_str);
                let full_native_jar = libraries_dir.join(native_rel_path);
                if full_native_jar.is_file() {
                    if extract_natives {
                        let excludes = lib.extract.as_ref().and_then(|e| e.exclude.as_deref());
                        let _ = extract_native_jar(&full_native_jar, &natives_dir, excludes);
                    }
                }
            }
        }
    }

    let mut cp_entries: Vec<String> = cp_order
        .into_iter()
        .filter_map(|key| cp_map.remove(&key))
        .collect();

    // 添加 client.jar（若子版本自身无 jar 且继承自父版本，则回退查找父版本 jar）
    let local_jar = version_dir.join(format!("{}.jar", version_id));
    if local_jar.is_file() {
        cp_entries.push(local_jar.to_string_lossy().to_string());
    } else if let Some(ref parent_id) = ver_json.inherits_from {
        let parent_jar = game_dir
            .join("versions")
            .join(parent_id)
            .join(format!("{}.jar", parent_id));
        if parent_jar.is_file() {
            cp_entries.push(parent_jar.to_string_lossy().to_string());
        }
    }

    let sep = platform::classpath_separator_for_os(target_os).to_string();
    let classpath = cp_entries.join(&sep);

    // 2. 准备变量映射表（按目标平台实值替换占位符）
    let effective_game_dir = crate::version::resolve_game_dir(game_dir, &version_dir);
    let assets_dir = game_dir.join("assets");
    let assets_index_name = ver_json
        .asset_index
        .as_ref()
        .map(|a| a.id.clone())
        .or_else(|| ver_json.assets.clone())
        .unwrap_or_else(|| "legacy".to_string());

    let auth_uuid = uuid.unwrap_or("55a5228ca5d74e4fb3bd99aa10d8cd98");
    let token = access_token.unwrap_or("0");
    let user_type = if access_token.is_some() { "msa" } else { "legacy" };

    let mut vars = HashMap::new();
    vars.insert("auth_player_name", player_name.to_string());
    vars.insert("version_name", version_id.to_string());
    vars.insert(
        "game_directory",
        effective_game_dir.to_string_lossy().to_string(),
    );
    vars.insert("assets_root", assets_dir.to_string_lossy().to_string());
    vars.insert("game_assets", assets_dir.to_string_lossy().to_string());
    vars.insert("assets_index_name", assets_index_name);
    vars.insert("auth_uuid", auth_uuid.to_string());
    vars.insert("auth_access_token", token.to_string());
    vars.insert("user_type", user_type.to_string());
    vars.insert("version_type", "PCL".to_string());
    vars.insert(
        "natives_directory",
        natives_dir.to_string_lossy().to_string(),
    );
    vars.insert(
        "library_directory",
        libraries_dir.to_string_lossy().to_string(),
    );
    vars.insert("classpath_separator", sep);
    vars.insert("launcher_name", "PCL".to_string());
    vars.insert("launcher_version", "408".to_string());
    vars.insert("classpath", classpath.clone());
    vars.insert("arch", target_arch.to_string());
    vars.insert("resolution_width", window_width.to_string());
    vars.insert("resolution_height", window_height.to_string());
    vars.insert("clientid", "${clientid}".to_string());
    vars.insert("auth_xuid", "${auth_xuid}".to_string());

    // 3. 构造 JVM 参数（保留版本 JSON 规则选出的平台专属参数如 -XstartOnFirstThread，不覆盖 -Dos.name）
    let mut jvm_args = Vec::new();
    jvm_args.push(format!("-Xmx{}m", max_memory_mb));
    jvm_args.push("-XX:+UnlockExperimentalVMOptions".to_string());
    jvm_args.push("-XX:+UseG1GC".to_string());
    jvm_args.push("-XX:-OmitStackTraceInFastThrow".to_string());
    jvm_args.push("-Djdk.lang.Process.allowAmbiguousCommands=True".to_string());
    jvm_args.push("-Dstdout.encoding=UTF-8".to_string());
    jvm_args.push("-Dstderr.encoding=UTF-8".to_string());

    if let Some(args) = &ver_json.arguments {
        if let Some(jvms) = &args.jvm {
            let resolved = resolve_arguments(
                jvms,
                target_os,
                target_arch,
                target_os_version,
                &active_features,
                &vars,
            );
            jvm_args.extend(resolved);
        }
    } else if target_os == "osx" {
        // 对于没有 arguments.jvm 字段的旧版（1.12.2 及以前），macOS 同样需要主线程运行 GLFW
        jvm_args.push("-XstartOnFirstThread".to_string());
    }

    if !jvm_args.iter().any(|a| a.contains("java.library.path")) {
        jvm_args.push(format!(
            "-Djava.library.path={}",
            natives_dir.to_string_lossy()
        ));
    }
    if !jvm_args.iter().any(|a| a == "-cp" || a == "-classpath") {
        jvm_args.push("-cp".to_string());
        jvm_args.push(vars.get("classpath").cloned().unwrap_or_default());
    }

    // 4. 构造 Game 参数
    let mut game_args = Vec::new();
    if let Some(args) = &ver_json.arguments {
        if let Some(games) = &args.game {
            let resolved = resolve_arguments(
                games,
                target_os,
                target_arch,
                target_os_version,
                &active_features,
                &vars,
            );
            game_args.extend(resolved);
        }
    } else if let Some(mc_args) = &ver_json.minecraft_arguments {
        for token in mc_args.split_whitespace() {
            game_args.push(replace_vars(token, &vars));
        }
        if window_width > 0 && window_height > 0 {
            game_args.push("--width".to_string());
            game_args.push(window_width.to_string());
            game_args.push("--height".to_string());
            game_args.push(window_height.to_string());
        }
    }

    Ok(LaunchArgumentsResult {
        required_java_major,
        effective_game_dir,
        natives_dir,
        classpath_entries: cp_entries,
        classpath,
        main_class: ver_json.main_class.clone(),
        jvm_args,
        game_args,
    })
}
