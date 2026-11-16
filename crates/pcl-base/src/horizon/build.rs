use super::api::{negotiate_client_api, negotiate_prepare_api};
use super::instances::{list_instances, now_iso_utc};
use crate::endpoints;
use crate::installer::DownloadProgress;
use crate::modpack::ModpackManager;
use crate::platform;
use crate::reporter::ReporterHandle;
use base64::{engine::general_purpose::STANDARD, Engine};
use ed25519_dalek::{Signature, VerifyingKey};
use flate2::read::GzDecoder;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::path::Path;
use tar::Archive;
use ts_rs::TS;
use zip::ZipArchive;

/// Horizon 官方发布公钥 (Base64 编码的 32 字节 Ed25519 公钥)
/// 来源：上游 horizon-rust/horizon 仓库 crates/horizon-release-sign/release-keys.txt
pub const HORIZON_RELEASE_PUBLIC_KEY_B64: &str =
    "MU4oiIQcCxkZvbwSjjXs7SySCuM6ywRQ3nGaJYZG0gw=";

/// PBL 自记 Horizon 构建元数据 (落盘于 <game_dir>/horizon/builds/<tag>/pbl-build.json)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
pub struct PblBuildMeta {
    pub tag: String,
    pub package_name: String,
    pub sha256: String,
    pub horizon_version: String,
    pub minecraft_version: String,
    pub launch_api_version: String,
    pub java_major: u32,
    pub installed_at: String,
}

/// 合并远端发行与本地状态的构建视图项
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
pub struct HorizonBuildItem {
    pub tag: String,
    pub published_at: Option<String>,
    pub package_name: Option<String>,
    #[ts(type = "number | null")]
    pub package_size: Option<u64>,
    pub is_installed: bool,
    pub installed_meta: Option<PblBuildMeta>,
}

/// GitHub Releases 原始资产项
#[derive(Debug, Clone, Deserialize)]
pub struct GitHubReleaseAsset {
    pub name: String,
    pub size: u64,
    pub browser_download_url: String,
}

/// GitHub Releases 原始项
#[derive(Debug, Clone, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub name: Option<String>,
    pub published_at: Option<String>,
    pub prerelease: bool,
    pub assets: Vec<GitHubReleaseAsset>,
}

/// 解析 SHA256SUMS 文件内容为 (文件名 -> 期望哈希小写) 映射表
pub fn parse_sha256sums(sums_text: &str) -> Result<HashMap<String, String>, String> {
    let mut map = HashMap::new();
    for line in sums_text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let mut parts = trimmed.split_whitespace();
        let hash = parts
            .next()
            .ok_or_else(|| "SHA256SUMS 缺少校验和字段".to_string())?
            .to_lowercase();
        let filename = parts
            .next()
            .ok_or_else(|| "SHA256SUMS 缺少文件名字段".to_string())?
            .trim_start_matches('*');

        if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!("非法的 SHA256 哈希值: {}", hash));
        }

        map.insert(filename.to_string(), hash);
    }
    Ok(map)
}

/// 校验 SHA256SUMS 的 Ed25519 签名
/// 契约说明：签名消息为 `format!("horizon release {}\n{}", tag, sha256sums_text)`
pub fn verify_release_signature(
    tag: &str,
    sums_bytes: &[u8],
    signature_text: &str,
    trusted_keys_b64: &[&str],
) -> Result<(), String> {
    if trusted_keys_b64.is_empty() {
        return Err("验签失败：受信任公钥列表为空".to_string());
    }

    let mut message = Vec::new();
    message.extend_from_slice(b"horizon release ");
    message.extend_from_slice(tag.as_bytes());
    message.extend_from_slice(b"\n");
    message.extend_from_slice(sums_bytes);

    let mut public_keys = Vec::new();
    for key_b64 in trusted_keys_b64 {
        let key_bytes = STANDARD
            .decode(key_b64.trim())
            .map_err(|e| format!("公钥 Base64 解码失败: {}", e))?;
        let key_arr: [u8; 32] = key_bytes
            .try_into()
            .map_err(|_| "公钥长度不满足 Ed25519 32 字节要求".to_string())?;
        let verifying_key = VerifyingKey::from_bytes(&key_arr)
            .map_err(|e| format!("解析 Ed25519 公钥失败: {}", e))?;
        public_keys.push(verifying_key);
    }

    let mut any_valid = false;
    for line in signature_text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let sig_bytes = match STANDARD.decode(trimmed) {
            Ok(b) => b,
            Err(_) => continue,
        };

        let sig_arr: [u8; 64] = match sig_bytes.try_into() {
            Ok(arr) => arr,
            Err(_) => continue,
        };

        let signature = Signature::from_bytes(&sig_arr);

        for key in &public_keys {
            if key.verify_strict(&message, &signature).is_ok() {
                any_valid = true;
                break;
            }
        }
        if any_valid {
            break;
        }
    }

    if any_valid {
        Ok(())
    } else {
        Err("Ed25519 签名校验失败：SHA256SUMS 未通过受信任公钥校验".to_string())
    }
}

/// 校验字节流 SHA256 哈希
pub fn verify_bytes_sha256(data: &[u8], expected_hex: &str) -> bool {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let actual = format!("{:x}", hasher.finalize());
    actual.eq_ignore_ascii_case(expected_hex.trim())
}

/// 安全解压构建归档包（防 zip-slip 与 tar 逃逸）
pub fn extract_build_package(
    archive_path: &Path,
    dest_dir: &Path,
    pkg_name: &str,
) -> Result<(), String> {
    if dest_dir.exists() {
        let _ = fs::remove_dir_all(dest_dir);
    }
    fs::create_dir_all(dest_dir).map_err(|e| format!("创建目标解压目录失败: {}", e))?;

    if pkg_name.ends_with(".zip") {
        let file = File::open(archive_path)
            .map_err(|e| format!("打开 zip 失败 {:?}: {}", archive_path, e))?;
        let mut zip = ZipArchive::new(file).map_err(|e| format!("解析 zip 归档失败: {}", e))?;
        for i in 0..zip.len() {
            let mut item = zip.by_index(i).map_err(|e| e.to_string())?;
            let raw_name = item.name().to_string();
            let Some(target) = ModpackManager::sanitize_relative_path(dest_dir, &raw_name) else {
                continue;
            };
            if item.is_dir() {
                let _ = fs::create_dir_all(&target);
            } else {
                if let Some(parent) = target.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let mut out = File::create(&target).map_err(|e| e.to_string())?;
                std::io::copy(&mut item, &mut out).map_err(|e| e.to_string())?;
                let _ = platform::make_executable(&target);
            }
        }
    } else if pkg_name.ends_with(".tar.gz") || pkg_name.ends_with(".tgz") {
        let file = File::open(archive_path)
            .map_err(|e| format!("打开 tar.gz 失败 {:?}: {}", archive_path, e))?;
        let tar = GzDecoder::new(file);
        let mut archive = Archive::new(tar);
        for entry_res in archive.entries().map_err(|e| e.to_string())? {
            let mut entry = entry_res.map_err(|e| e.to_string())?;
            let path = entry.path().map_err(|e| e.to_string())?;
            let path_str = path.to_string_lossy();
            let Some(target) = ModpackManager::sanitize_relative_path(dest_dir, &path_str) else {
                continue;
            };
            if entry.header().entry_type().is_dir() {
                let _ = fs::create_dir_all(&target);
            } else {
                if let Some(parent) = target.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let mut out = File::create(&target).map_err(|e| e.to_string())?;
                std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
                let _ = platform::make_executable(&target);
            }
        }
    } else {
        return Err(format!("不支持的归档格式: {}", pkg_name));
    }

    let _ = platform::clear_quarantine(dest_dir);
    Ok(())
}

/// 读取本地已安装构建的元数据
pub fn read_installed_build_meta(game_dir: &Path, tag: &str) -> Option<PblBuildMeta> {
    let meta_file = game_dir
        .join("horizon")
        .join("builds")
        .join(tag)
        .join("pbl-build.json");
    if !meta_file.is_file() {
        return None;
    }
    let content = fs::read_to_string(meta_file).ok()?;
    serde_json::from_str(&content).ok()
}

/// 列出所有本地已安装的 Horizon 构建
pub fn list_installed_builds(game_dir: &Path) -> Vec<PblBuildMeta> {
    let builds_dir = game_dir.join("horizon").join("builds");
    let mut list = Vec::new();
    if let Ok(entries) = fs::read_dir(builds_dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                if let Some(tag) = entry.file_name().to_str() {
                    if let Some(meta) = read_installed_build_meta(game_dir, tag) {
                        list.push(meta);
                    }
                }
            }
        }
    }
    list
}

/// 合并远端 GitHub Releases 与本地已安装构建列表（纯函数，便于测试）
/// 保留已失效（超过 7 天清理期）但本地仍存在的构建
pub fn merge_remote_and_local_builds(
    remote: &[GitHubRelease],
    local: &[PblBuildMeta],
    pkg_name: Option<&str>,
) -> Vec<HorizonBuildItem> {
    let local_map: HashMap<&str, &PblBuildMeta> =
        local.iter().map(|m| (m.tag.as_str(), m)).collect();
    let mut seen_tags: HashSet<String> = HashSet::new();
    let mut result = Vec::new();

    for rel in remote {
        let matched_asset = pkg_name.and_then(|target| {
            rel.assets.iter().find(|a| a.name == target)
        });
        let local_meta = local_map.get(rel.tag_name.as_str()).copied().cloned();
        seen_tags.insert(rel.tag_name.clone());
        result.push(HorizonBuildItem {
            tag: rel.tag_name.clone(),
            published_at: rel.published_at.clone(),
            package_name: matched_asset
                .map(|a| a.name.clone())
                .or_else(|| pkg_name.map(|s| s.to_string())),
            package_size: matched_asset.map(|a| a.size),
            is_installed: local_meta.is_some(),
            installed_meta: local_meta,
        });
    }

    for meta in local {
        if !seen_tags.contains(&meta.tag) {
            result.push(HorizonBuildItem {
                tag: meta.tag.clone(),
                published_at: Some(meta.installed_at.clone()),
                package_name: Some(meta.package_name.clone()),
                package_size: None,
                is_installed: true,
                installed_meta: Some(meta.clone()),
            });
        }
    }

    result
}

/// 查询远端可用 Horizon 构建并与本地已安装状态合并
pub async fn fetch_horizon_builds(
    client: &Client,
    game_dir: &Path,
) -> Result<Vec<HorizonBuildItem>, String> {
    let resp = client
        .get(endpoints::HORIZON_RELEASES_API_URL)
        .header("User-Agent", "PureBuildLauncher")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("拉取 Horizon Releases 列表失败: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("GitHub Releases API 返回异常状态: {}", resp.status()));
    }

    let releases: Vec<GitHubRelease> = resp
        .json()
        .await
        .map_err(|e| format!("解析 GitHub Releases JSON 失败: {}", e))?;
    let local = list_installed_builds(game_dir);
    let pkg_name = platform::horizon_package_name();

    Ok(merge_remote_and_local_builds(&releases, &local, pkg_name))
}

/// 删除指定构建包及其关联预备资源（若有实例绑定该构建则拒绝删除）
pub fn delete_installed_build(game_dir: &Path, tag: &str) -> Result<(), String> {
    let bound: Vec<String> = list_instances(game_dir)
        .into_iter()
        .filter(|inst| inst.build_tag == tag)
        .map(|inst| inst.name)
        .collect();
    if !bound.is_empty() {
        return Err(format!(
            "构建 {} 正被实例 {} 绑定使用，请先删除相关实例后再移除构建",
            tag,
            bound.join("、")
        ));
    }

    let build_dir = game_dir.join("horizon").join("builds").join(tag);
    if build_dir.exists() {
        fs::remove_dir_all(&build_dir).map_err(|e| format!("删除构建目录失败: {}", e))?;
    }
    let resource_dir = game_dir.join("horizon").join("resources").join(tag);
    if resource_dir.exists() {
        fs::remove_dir_all(&resource_dir).map_err(|e| format!("删除预备资源目录失败: {}", e))?;
    }
    Ok(())
}

/// 下载指定 URL 内容为字符串
async fn fetch_string(client: &Client, url: &str) -> Result<String, String> {
    let resp = client
        .get(url)
        .header("User-Agent", "PureBuildLauncher")
        .send()
        .await
        .map_err(|e| format!("网络请求失败: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP 错误状态: {} ({})", resp.status(), url));
    }
    resp.text().await.map_err(|e| format!("读取内容失败: {}", e))
}

/// 完整的 Horizon 构建安装管线：
/// 1. 获取发布资产信息并定位当前平台的构建包；
/// 2. 下载 SHA256SUMS 与 SHA256SUMS.sig；
/// 3. 进行 Ed25519 签名验证（验签失败整包拒绝并报错，不降级为只验 SHA）；
/// 4. 解析 SHA256SUMS，校验目标包是否存在；
/// 5. 下载目标包并进行流式 SHA-256 完整性校验；
/// 6. 解压至 `<game_dir>/horizon/builds/<tag>/`；
/// 7. 运行 `--launch-api` 进行选项与版本协商；
/// 8. 写入 `pbl-build.json` 元数据。
pub async fn install_horizon_build(
    reporter: &ReporterHandle,
    client: &Client,
    game_dir: &Path,
    tag: &str,
) -> Result<PblBuildMeta, String> {
    let pkg_name = platform::horizon_package_name()
        .ok_or_else(|| "当前平台与架构暂无适配的 Horizon 官方预编译包".to_string())?;

    reporter.emit_download(DownloadProgress {
        version_id: format!("horizon-{}", tag),
        vanilla_version: None,
        step: format!("正在获取 Horizon {} 签名与校验单...", tag),
        percentage: 10.0,
        speed_text: "".to_string(),
        is_finished: false,
        error: None,
    });

    let sums_url = endpoints::horizon_release_download_url(tag, "SHA256SUMS");
    let sums_text = fetch_string(client, &sums_url)
        .await
        .map_err(|e| format!("下载 SHA256SUMS 失败: {}", e))?;

    // 优先尝试读取 SHA256SUMS.sig
    let sig_url = endpoints::horizon_release_download_url(tag, "SHA256SUMS.sig");
    let sig_text = match fetch_string(client, &sig_url).await {
        Ok(s) => s,
        Err(_) => {
            // 回退尝试 SHA256SUMS.v2.sig
            let sig_v2_url = endpoints::horizon_release_download_url(tag, "SHA256SUMS.v2.sig");
            fetch_string(client, &sig_v2_url)
                .await
                .map_err(|e| format!("下载 SHA256SUMS.sig 与 SHA256SUMS.v2.sig 均失败: {}", e))?
        }
    };

    reporter.emit_download(DownloadProgress {
        version_id: format!("horizon-{}", tag),
        vanilla_version: None,
        step: "正在验证 Ed25519 官方发布签名...".to_string(),
        percentage: 20.0,
        speed_text: "".to_string(),
        is_finished: false,
        error: None,
    });

    verify_release_signature(
        tag,
        sums_text.as_bytes(),
        &sig_text,
        &[HORIZON_RELEASE_PUBLIC_KEY_B64],
    )?;

    let sums_map = parse_sha256sums(&sums_text)?;
    let expected_hash = sums_map.get(pkg_name).ok_or_else(|| {
        format!(
            "在 Horizon {} 的已签名校验单中找不到当前平台的包 {}",
            tag, pkg_name
        )
    })?;

    reporter.emit_download(DownloadProgress {
        version_id: format!("horizon-{}", tag),
        vanilla_version: None,
        step: format!("正在下载 Horizon 构建包 {}...", pkg_name),
        percentage: 25.0,
        speed_text: "".to_string(),
        is_finished: false,
        error: None,
    });

    let pkg_url = endpoints::horizon_release_download_url(tag, pkg_name);
    let mut resp = client
        .get(&pkg_url)
        .header("User-Agent", "PureBuildLauncher")
        .send()
        .await
        .map_err(|e| format!("请求下载构建包失败: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!(
            "下载构建包 HTTP 状态异常: {} ({})",
            resp.status(),
            pkg_url
        ));
    }

    let total_bytes = resp.content_length();
    let builds_dir = game_dir.join("horizon").join("builds");
    let target_build_dir = builds_dir.join(tag);
    let temp_archive = builds_dir.join(format!(".downloading_{}.tmp", tag));

    if let Some(p) = temp_archive.parent() {
        let _ = fs::create_dir_all(p);
    }

    use tokio::io::AsyncWriteExt;
    let mut temp_file = tokio::fs::File::create(&temp_archive)
        .await
        .map_err(|e| format!("创建临时文件失败: {}", e))?;

    let mut hasher = Sha256::new();
    let mut downloaded = 0u64;

    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|e| format!("读取下载数据流失败: {}", e))?
    {
        hasher.update(&chunk);
        temp_file
            .write_all(&chunk)
            .await
            .map_err(|e| format!("写入构建包失败: {}", e))?;

        downloaded += chunk.len() as u64;
        if let Some(total) = total_bytes {
            if total > 0 {
                let pct = 25.0 + ((downloaded as f32 / total as f32) * 50.0).min(50.0);
                let mb_down = downloaded as f64 / 1_048_576.0;
                let mb_total = total as f64 / 1_048_576.0;
                reporter.emit_download(DownloadProgress {
                    version_id: format!("horizon-{}", tag),
                    vanilla_version: None,
                    step: format!(
                        "正在下载 Horizon 构建包 ({:.1}MB / {:.1}MB)...",
                        mb_down, mb_total
                    ),
                    percentage: pct,
                    speed_text: "".to_string(),
                    is_finished: false,
                    error: None,
                });
            }
        }
    }

    let _ = temp_file.flush().await;
    drop(temp_file);

    let actual_hash = format!("{:x}", hasher.finalize());
    if !actual_hash.eq_ignore_ascii_case(expected_hash) {
        let _ = fs::remove_file(&temp_archive);
        return Err(format!(
            "构建包 SHA256 完整性校验失败：期望 {}, 实际 {}",
            expected_hash, actual_hash
        ));
    }

    reporter.emit_download(DownloadProgress {
        version_id: format!("horizon-{}", tag),
        vanilla_version: None,
        step: "下载校验通过，正在安全解压构建包...".to_string(),
        percentage: 80.0,
        speed_text: "".to_string(),
        is_finished: false,
        error: None,
    });

    let extract_res = extract_build_package(&temp_archive, &target_build_dir, pkg_name);
    let _ = fs::remove_file(&temp_archive);

    if let Err(e) = extract_res {
        let _ = fs::remove_dir_all(&target_build_dir);
        return Err(format!("解压构建包失败: {}", e));
    }

    reporter.emit_download(DownloadProgress {
        version_id: format!("horizon-{}", tag),
        vanilla_version: None,
        step: "正在与构建协商 Launch API 契约...".to_string(),
        percentage: 90.0,
        speed_text: "".to_string(),
        is_finished: false,
        error: None,
    });

    let client_bin = target_build_dir.join(platform::horizon_client_bin_name());
    let prepare_bin = target_build_dir.join(platform::horizon_prepare_bin_name());

    let client_api_res = negotiate_client_api(&client_bin);
    let prepare_api_res = negotiate_prepare_api(&prepare_bin);

    let (client_api, prepare_api) = match (client_api_res, prepare_api_res) {
        (Ok(c), Ok(p)) => (c, p),
        (Err(e), _) | (_, Err(e)) => {
            let _ = fs::remove_dir_all(&target_build_dir);
            return Err(format!("构建协商失败 (已撤销安装): {}", e));
        }
    };

    let meta = PblBuildMeta {
        tag: tag.to_string(),
        package_name: pkg_name.to_string(),
        sha256: actual_hash,
        horizon_version: client_api.horizon,
        minecraft_version: client_api.minecraft,
        launch_api_version: client_api.launch_api,
        java_major: prepare_api.java_major,
        installed_at: now_iso_utc(),
    };

    let meta_json = serde_json::to_string_pretty(&meta)
        .map_err(|e| format!("序列化 pbl-build.json 失败: {}", e))?;
    fs::write(target_build_dir.join("pbl-build.json"), meta_json)
        .map_err(|e| format!("写入 pbl-build.json 失败: {}", e))?;

    reporter.emit_download(DownloadProgress {
        version_id: format!("horizon-{}", tag),
        vanilla_version: None,
        step: format!("Horizon {} 构建安装完成", tag),
        percentage: 100.0,
        speed_text: "".to_string(),
        is_finished: true,
        error: None,
    });

    Ok(meta)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;

    #[test]
    fn test_parse_sha256sums_valid() {
        let content = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855  horizon-windows-x64.zip\n\
                       a591a6d40bf420404a011733cfb7b190d62c65bf0bcda32b57b277d9ad9f146e *horizon-macos-arm64.zip\n";
        let parsed = parse_sha256sums(content).expect("应正确解析");
        assert_eq!(
            parsed.get("horizon-windows-x64.zip"),
            Some(&"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string())
        );
        assert_eq!(
            parsed.get("horizon-macos-arm64.zip"),
            Some(&"a591a6d40bf420404a011733cfb7b190d62c65bf0bcda32b57b277d9ad9f146e".to_string())
        );
    }

    #[test]
    fn test_verify_release_signature_success_and_tamper_fails() {
        let secret_bytes = [42u8; 32];
        let signing_key = SigningKey::from_bytes(&secret_bytes);
        let verifying_key = signing_key.verifying_key();
        let pub_b64 = STANDARD.encode(verifying_key.as_bytes());

        let tag = "nightly-2026-11-16";
        let sums_content = b"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855  horizon-windows-x64.zip\n";

        let mut message = Vec::new();
        message.extend_from_slice(b"horizon release ");
        message.extend_from_slice(tag.as_bytes());
        message.extend_from_slice(b"\n");
        message.extend_from_slice(sums_content);

        use ed25519_dalek::Signer;
        let sig = signing_key.sign(&message);
        let sig_b64 = STANDARD.encode(sig.to_bytes());

        let res = verify_release_signature(tag, sums_content, &sig_b64, &[&pub_b64]);
        assert!(res.is_ok(), "正确签名校验应通过: {:?}", res);

        let tampered_sums = b"ffffc44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855  horizon-windows-x64.zip\n";
        let tamper_res = verify_release_signature(tag, tampered_sums, &sig_b64, &[&pub_b64]);
        assert!(tamper_res.is_err(), "篡改内容必须验签失败");

        let bad_tag_res =
            verify_release_signature("nightly-2026-11-17", sums_content, &sig_b64, &[&pub_b64]);
        assert!(bad_tag_res.is_err(), "篡改 Tag 必须验签失败");
    }

    #[test]
    fn test_merge_remote_and_local_builds_preserves_expired_local() {
        let remote = vec![GitHubRelease {
            tag_name: "nightly".to_string(),
            name: Some("Nightly".to_string()),
            published_at: Some("2026-11-16T00:00:00Z".to_string()),
            prerelease: true,
            assets: vec![GitHubReleaseAsset {
                name: "horizon-windows-x64.zip".to_string(),
                size: 12_345_678,
                browser_download_url: "https://example.invalid/horizon.zip".to_string(),
            }],
        }];
        let local = vec![PblBuildMeta {
            tag: "nightly-2026-11-01".to_string(),
            package_name: "horizon-windows-x64.zip".to_string(),
            sha256: "a".repeat(64),
            horizon_version: "0.1.0".to_string(),
            minecraft_version: "26.3".to_string(),
            launch_api_version: "0.1.0".to_string(),
            java_major: 25,
            installed_at: "2026-11-01T00:00:00Z".to_string(),
        }];

        let merged = merge_remote_and_local_builds(&remote, &local, Some("horizon-windows-x64.zip"));
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].tag, "nightly");
        assert!(!merged[0].is_installed);
        assert_eq!(merged[0].package_size, Some(12_345_678));

        assert_eq!(merged[1].tag, "nightly-2026-11-01");
        assert!(merged[1].is_installed);
    }

    #[test]
    fn test_delete_build_blocked_when_instance_bound() {
        let tmp = std::env::temp_dir().join(format!("pbl_build_del_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let build_dir = tmp.join("horizon").join("builds").join("nightly");
        fs::create_dir_all(&build_dir).unwrap();
        let meta = PblBuildMeta {
            tag: "nightly".to_string(),
            package_name: "horizon-windows-x64.zip".to_string(),
            sha256: "0".repeat(64),
            horizon_version: "0.1.1".to_string(),
            minecraft_version: "26.3".to_string(),
            launch_api_version: "0.1.1".to_string(),
            java_major: 25,
            installed_at: "2026-11-16T00:00:00Z".to_string(),
        };
        fs::write(
            build_dir.join("pbl-build.json"),
            serde_json::to_string(&meta).unwrap(),
        )
        .unwrap();

        super::super::instances::create_instance(&tmp, "MyWorld", "nightly").unwrap();

        let err = delete_installed_build(&tmp, "nightly").unwrap_err();
        assert!(err.contains("MyWorld"));
        assert!(build_dir.exists());

        super::super::instances::delete_instance(&tmp, "MyWorld").unwrap();
        delete_installed_build(&tmp, "nightly").unwrap();
        assert!(!build_dir.exists());

        let _ = fs::remove_dir_all(&tmp);
    }
}
