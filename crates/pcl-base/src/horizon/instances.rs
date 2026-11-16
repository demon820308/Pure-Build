use super::build::read_installed_build_meta;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use ts_rs::TS;

/// Horizon 实例注册项（仅记录名称、绑定的构建 tag 与创建时间，不侵入现有 Java 版本结构）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
pub struct HorizonInstance {
    pub name: String,
    pub build_tag: String,
    pub created_at: String,
}

/// 生成 UTC ISO-8601 时间戳（零外部日期库依赖）
pub(crate) fn now_iso_utc() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    unix_secs_to_iso_utc(secs)
}

pub(crate) fn unix_secs_to_iso_utc(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let hour = rem / 3_600;
    let minute = (rem % 3_600) / 60;
    let second = rem % 60;

    // Howard Hinnant civil_from_days 算法
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = y + if m <= 2 { 1 } else { 0 };

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, m, d, hour, minute, second
    )
}

/// 获取 Horizon 实例注册表文件路径 `<game_dir>/horizon/instances.json`
pub fn instances_registry_path(game_dir: &Path) -> PathBuf {
    game_dir.join("horizon").join("instances.json")
}

/// 获取指定 Horizon 实例目录 `<game_dir>/horizon/instances/<name>/`
pub fn instance_dir(game_dir: &Path, name: &str) -> PathBuf {
    game_dir.join("horizon").join("instances").join(name)
}

/// 校验实例名是否安全合法（防路径遍历与非法字符）
pub fn validate_instance_name(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("实例名称不能为空".to_string());
    }
    if trimmed == "." || trimmed == ".." {
        return Err("实例名称不能为 . 或 ..".to_string());
    }
    let forbidden = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
    if trimmed.chars().any(|c| forbidden.contains(&c) || c.is_control()) {
        return Err("实例名称包含非法路径字符".to_string());
    }
    Ok(trimmed.to_string())
}

/// 列出所有已创建的 Horizon 实例
pub fn list_instances(game_dir: &Path) -> Vec<HorizonInstance> {
    let path = instances_registry_path(game_dir);
    if !path.is_file() {
        return Vec::new();
    }
    let Ok(content) = fs::read_to_string(path) else {
        return Vec::new();
    };
    serde_json::from_str(&content).unwrap_or_default()
}

/// 查找指定名称的 Horizon 实例
pub fn find_instance(game_dir: &Path, name: &str) -> Option<HorizonInstance> {
    list_instances(game_dir)
        .into_iter()
        .find(|inst| inst.name == name)
}

fn save_instances(game_dir: &Path, list: &[HorizonInstance]) -> Result<(), String> {
    let path = instances_registry_path(game_dir);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建 Horizon 数据目录失败: {}", e))?;
    }
    let content =
        serde_json::to_string_pretty(list).map_err(|e| format!("序列化实例列表失败: {}", e))?;
    fs::write(&path, content).map_err(|e| format!("写入 instances.json 失败: {}", e))
}

/// 创建新的 Horizon 实例（校验名称、绑定已安装构建、创建独立目录并写入注册表）
pub fn create_instance(
    game_dir: &Path,
    name: &str,
    build_tag: &str,
) -> Result<HorizonInstance, String> {
    let clean_name = validate_instance_name(name)?;
    if read_installed_build_meta(game_dir, build_tag).is_none() {
        return Err(format!("所选 Horizon 构建 {} 未安装或已损坏", build_tag));
    }

    let mut list = list_instances(game_dir);
    if list.iter().any(|i| i.name == clean_name) {
        return Err(format!("Horizon 实例「{}」已存在", clean_name));
    }

    let inst_dir = instance_dir(game_dir, &clean_name);
    fs::create_dir_all(&inst_dir).map_err(|e| format!("创建实例目录失败: {}", e))?;

    let item = HorizonInstance {
        name: clean_name,
        build_tag: build_tag.to_string(),
        created_at: now_iso_utc(),
    };
    list.push(item.clone());
    save_instances(game_dir, &list)?;
    Ok(item)
}

/// 删除指定 Horizon 实例记录及其独立目录
pub fn delete_instance(game_dir: &Path, name: &str) -> Result<(), String> {
    let clean_name = validate_instance_name(name)?;
    let mut list = list_instances(game_dir);
    let before = list.len();
    list.retain(|i| i.name != clean_name);
    if list.len() == before {
        return Err(format!("找不到 Horizon 实例「{}」", clean_name));
    }
    save_instances(game_dir, &list)?;

    let inst_dir = instance_dir(game_dir, &clean_name);
    if inst_dir.exists() {
        fs::remove_dir_all(&inst_dir).map_err(|e| format!("删除实例目录失败: {}", e))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::horizon::build::PblBuildMeta;

    struct TempDir(PathBuf);
    impl TempDir {
        fn new(suffix: &str) -> Self {
            let p = std::env::temp_dir().join(format!(
                "pbl_horizon_inst_{}_{}",
                std::process::id(),
                suffix
            ));
            let _ = fs::remove_dir_all(&p);
            fs::create_dir_all(&p).unwrap();
            Self(p)
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn seed_build(game_dir: &Path, tag: &str) {
        let dir = game_dir.join("horizon").join("builds").join(tag);
        fs::create_dir_all(&dir).unwrap();
        let meta = PblBuildMeta {
            tag: tag.to_string(),
            package_name: "horizon-windows-x64.zip".to_string(),
            sha256: "0".repeat(64),
            horizon_version: "0.1.1".to_string(),
            minecraft_version: "26.3".to_string(),
            launch_api_version: "0.1.1".to_string(),
            java_major: 25,
            installed_at: "2026-11-16T00:00:00Z".to_string(),
        };
        fs::write(
            dir.join("pbl-build.json"),
            serde_json::to_string_pretty(&meta).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn test_unix_secs_to_iso_utc() {
        assert_eq!(unix_secs_to_iso_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(unix_secs_to_iso_utc(1_794_800_000), "2026-11-16T03:33:20Z");
    }

    #[test]
    fn test_instance_lifecycle_and_validation() {
        let tmp = TempDir::new("lifecycle");
        seed_build(&tmp.0, "nightly");

        assert!(validate_instance_name("../evil").is_err());
        assert!(validate_instance_name("  ").is_err());
        assert!(create_instance(&tmp.0, "test1", "missing-tag").is_err());

        let created = create_instance(&tmp.0, "Survival", "nightly").expect("创建实例应成功");
        assert_eq!(created.name, "Survival");
        assert_eq!(created.build_tag, "nightly");
        assert!(instance_dir(&tmp.0, "Survival").is_dir());

        // 重复名称应拒绝
        assert!(create_instance(&tmp.0, "Survival", "nightly").is_err());

        let list = list_instances(&tmp.0);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "Survival");

        delete_instance(&tmp.0, "Survival").expect("删除实例应成功");
        assert!(list_instances(&tmp.0).is_empty());
        assert!(!instance_dir(&tmp.0, "Survival").exists());
    }
}
