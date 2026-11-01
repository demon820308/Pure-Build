//! 集中管理所有下载源、镜像地址、API 端点及产品级命名常量。
//! 遵循项目反写死规范（§0 & §2）：业务模块只引用本模块的命名常量或辅助函数，禁止散落 URL 或魔法数字。

// ==================== 1. 产品级默认配置常量 ====================

/// 默认最大 JVM 堆内存 (MB) —— 产品级默认常量
pub const DEFAULT_MAX_MEMORY_MB: u32 = 4096;

/// 默认最小 JVM 堆内存 (MB) —— 开服脚本等使用
pub const DEFAULT_MIN_MEMORY_MB: u32 = 1024;

/// 默认游戏窗口宽度 (px) —— Minecraft 官方默认分辨率
pub const DEFAULT_WINDOW_WIDTH: u32 = 854;

/// 默认游戏窗口高度 (px) —— Minecraft 官方默认分辨率
pub const DEFAULT_WINDOW_HEIGHT: u32 = 480;

/// 默认主题色相 (HSL Hue: 205 经典 PCL 蓝)
pub const DEFAULT_THEME_HUE: u32 = 205;

/// 默认背景毛玻璃模糊度 (px)
pub const DEFAULT_BG_BLUR: u32 = 8;

/// 默认背景遮罩不透明度 (%)
pub const DEFAULT_BG_OPACITY: u32 = 85;

/// 默认 UI 界面风格与布局模式 ("classic" = 原版 PCL2 经典布局, "modern" = 现代侧栏布局)
pub const DEFAULT_UI_STYLE: &str = "classic";

/// 默认是否隐藏外置登录入口（默认隐藏 = true）
pub const DEFAULT_HIDE_YGGDRASIL_LOGIN: bool = true;

/// 默认背景音乐子目录名（位于 app_config_dir 下）
pub const DEFAULT_MUSIC_DIR_NAME: &str = "Musics";

/// 默认背景音乐音量百分比 (0 ~ 100)
pub const DEFAULT_MUSIC_VOLUME: u32 = 45;

/// 默认背景音乐循环播放模式 ("shuffle" = 随机循环, "list" = 列表顺序循环, "single" = 单曲循环)
pub const DEFAULT_MUSIC_PLAY_MODE: &str = "shuffle";

/// 默认打开启动器时是否自动播放背景音乐
pub const DEFAULT_MUSIC_AUTO_PLAY: bool = true;

/// 默认游戏启动时是否自动暂停背景音乐（游戏退出后自动恢复）
pub const DEFAULT_MUSIC_PAUSE_ON_LAUNCH: bool = true;

/// 默认背景音乐均衡器调音预设 ("flat" | "bass_boost" | "ambient" | "vocal" | "warm" | "custom")
pub const DEFAULT_MUSIC_EQ_PRESET: &str = "flat";

/// 默认是否开启音频动态防破音压缩器
pub const DEFAULT_MUSIC_COMPRESSOR: bool = true;

/// 默认是否启用音频平滑声学过渡淡入淡出（防爆音）
pub const DEFAULT_MUSIC_CROSSFADE: bool = true;

/// 支持扫描的本地音频文件扩展名（全小写）
pub const SUPPORTED_AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "wav", "ogg", "m4a", "aac", "opus", "webm",
];

/// Minecraft 局域网广播标准多播地址与端口（协议标准常量）
pub const MC_LAN_MULTICAST_ADDR: &str = "224.0.2.60";
pub const MC_LAN_MULTICAST_PORT: u16 = 4445;
pub const MC_LAN_BIND_ADDR: &str = "0.0.0.0:4445";
pub const MC_LAN_SEND_BIND_ADDR: &str = "0.0.0.0:0";

/// 默认离线玩家名称
pub const DEFAULT_PLAYER_NAME: &str = "Steve";

/// 支持库并发下载最大并发数
pub const DEFAULT_LIBRARY_CONCURRENCY: usize = 32;

/// Assets 资源并发下载最大并发数
pub const DEFAULT_ASSET_CONCURRENCY: usize = 64;

/// 应用配置文件夹标识
pub const APP_IDENTIFIER: &str = "com.purebuild.launcher";

/// 配置文件名
pub const CONFIG_FILE_NAME: &str = "pcl_config.json";

/// 凭据文件名（存放正版登录 Profile 与 access_token，移出前端 localStorage）
pub const CREDENTIALS_FILE_NAME: &str = "pcl_credentials.json";

// ==================== 2. 微软正版认证公开端点与常量 ====================

/// 微软官方公开给 Minecraft 启动器的 OAuth Client ID（见规范 §2 例外登记）
pub const MS_OAUTH_CLIENT_ID: &str = "00000000402b5328";

pub const MS_DEVICE_CODE_URL: &str = "https://login.live.com/oauth20_connect.srf";
pub const MS_TOKEN_URL: &str = "https://login.live.com/oauth20_token.srf";
pub const MS_DEFAULT_VERIFICATION_URI: &str = "https://www.microsoft.com/link";
pub const XBL_AUTH_URL: &str = "https://user.auth.xboxlive.com/user/authenticate";
pub const XSTS_AUTH_URL: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";
pub const MC_LOGIN_XBOX_URL: &str = "https://api.minecraftservices.com/authentication/login_with_xbox";
pub const MC_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";
pub const CRAFATAR_AVATAR_BASE: &str = "https://crafatar.com/avatars";

/// 默认 Yggdrasil 外置登录验证服务器地址（LittleSkin 公开 API 根地址，见 §2 例外登记）
pub const DEFAULT_YGGDRASIL_API_ROOT: &str = "https://littleskin.cn/api/yggdrasil";

// ==================== 3. 官方源与镜像源端点 ====================

/// Mojang 官方源基础地址
pub const MOJANG_PISTON_META: &str = "https://piston-meta.mojang.com";
pub const MOJANG_PISTON_DATA: &str = "https://piston-data.mojang.com";
pub const MOJANG_LIBRARIES: &str = "https://libraries.minecraft.net";
pub const MOJANG_RESOURCES: &str = "https://resources.download.minecraft.net";
pub const MOJANG_VERSION_MANIFEST_V2: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

/// Fabric / Quilt / Forge / NeoForge 官方源基础地址
pub const FABRIC_META_OFFICIAL: &str = "https://meta.fabricmc.net";
pub const FABRIC_MAVEN_OFFICIAL: &str = "https://maven.fabricmc.net";
pub const QUILT_META_OFFICIAL: &str = "https://meta.quiltmc.org";
pub const QUILT_MAVEN_OFFICIAL: &str = "https://maven.quiltmc.org/repository/release";
pub const FORGE_MAVEN_OFFICIAL: &str = "https://maven.minecraftforge.net";
pub const NEOFORGE_MAVEN_OFFICIAL: &str = "https://maven.neoforged.net/releases";

/// BMCLAPI 镜像源基础地址（全仓唯一声明处）
pub const BMCLAPI_ROOT: &str = "https://bmclapi2.bangbang93.com";
pub const BMCLAPI_MAVEN: &str = "https://bmclapi2.bangbang93.com/maven";
pub const BMCLAPI_ASSETS: &str = "https://bmclapi2.bangbang93.com/assets";
pub const BMCLAPI_FABRIC_META: &str = "https://bmclapi2.bangbang93.com/fabric-meta";
pub const BMCLAPI_QUILT_META: &str = "https://bmclapi2.bangbang93.com/quilt-meta";
pub const BMCLAPI_VERSION_MANIFEST_V2: &str =
    "https://bmclapi2.bangbang93.com/mc/game/version_manifest_v2.json";

/// Modrinth 社区 API 基础地址
pub const MODRINTH_API_BASE: &str = "https://api.modrinth.com/v2";
pub const LAUNCHER_USER_AGENT: &str = "PlainCraftLauncher/2.0 (Vue3+Tauri)";

// ==================== 4. 多源 URL 组装与故障转移辅助函数 ====================

/// 返回版本清单候选 URL 列表（根据是否启用镜像源决定优先级，官方源与镜像源互为备份）
pub fn version_manifest_urls(prefer_mirror: bool) -> [&'static str; 2] {
    if prefer_mirror {
        [BMCLAPI_VERSION_MANIFEST_V2, MOJANG_VERSION_MANIFEST_V2]
    } else {
        [MOJANG_VERSION_MANIFEST_V2, BMCLAPI_VERSION_MANIFEST_V2]
    }
}

/// 返回 Fabric 元数据版本汇总候选 URL 列表
pub fn fabric_versions_urls(prefer_mirror: bool) -> [String; 2] {
    let official = format!("{}/v2/versions", FABRIC_META_OFFICIAL);
    let mirror = format!("{}/v2/versions", BMCLAPI_FABRIC_META);
    if prefer_mirror {
        [mirror, official]
    } else {
        [official, mirror]
    }
}

/// 返回指定游戏版本的 Fabric Loader 清单候选 URL 列表
pub fn fabric_loader_list_urls(vanilla_version: &str, prefer_mirror: bool) -> [String; 2] {
    let official = format!("{}/v2/versions/loader/{}", FABRIC_META_OFFICIAL, vanilla_version);
    let mirror = format!("{}/v2/versions/loader/{}", BMCLAPI_FABRIC_META, vanilla_version);
    if prefer_mirror {
        [mirror, official]
    } else {
        [official, mirror]
    }
}

/// 返回指定游戏版本与 Loader 版本的 Fabric Profile JSON 候选 URL 列表
pub fn fabric_profile_json_urls(
    vanilla_version: &str,
    loader_version: &str,
    prefer_mirror: bool,
) -> [String; 2] {
    let official = format!(
        "{}/v2/versions/loader/{}/{}/profile/json",
        FABRIC_META_OFFICIAL, vanilla_version, loader_version
    );
    let mirror = format!(
        "{}/v2/versions/loader/{}/{}/profile/json",
        BMCLAPI_FABRIC_META, vanilla_version, loader_version
    );
    if prefer_mirror {
        [mirror, official]
    } else {
        [official, mirror]
    }
}

/// 返回指定游戏版本与 Loader 版本的 Quilt Profile JSON 候选 URL 列表
pub fn quilt_profile_json_urls(
    vanilla_version: &str,
    loader_version: &str,
    prefer_mirror: bool,
) -> [String; 2] {
    let official = format!(
        "{}/v3/versions/loader/{}/{}/profile/json",
        QUILT_META_OFFICIAL, vanilla_version, loader_version
    );
    let mirror = format!(
        "{}/v3/versions/loader/{}/{}/profile/json",
        BMCLAPI_QUILT_META, vanilla_version, loader_version
    );
    if prefer_mirror {
        [mirror, official]
    } else {
        [official, mirror]
    }
}

/// 返回 Forge Installer JAR 的 (首选 URL, 备选 URL)
pub fn forge_installer_urls(
    mc_version: &str,
    forge_version: &str,
    prefer_mirror: bool,
) -> (String, String) {
    let official = format!(
        "{}/net/minecraftforge/forge/{}-{}/forge-{}-{}-installer.jar",
        FORGE_MAVEN_OFFICIAL, mc_version, forge_version, mc_version, forge_version
    );
    let mirror = format!(
        "{}/forge/download?mcversion={}&version={}&category=installer&format=jar",
        BMCLAPI_ROOT, mc_version, forge_version
    );
    if prefer_mirror {
        (mirror, official)
    } else {
        (official, mirror)
    }
}

/// 返回 NeoForge Installer JAR 的 (首选 URL, 备选 URL)
pub fn neoforge_installer_urls(neoforge_version: &str, prefer_mirror: bool) -> (String, String) {
    let official = format!(
        "{}/net/neoforged/neoforge/{}/neoforge-{}-installer.jar",
        NEOFORGE_MAVEN_OFFICIAL, neoforge_version, neoforge_version
    );
    let mirror = format!(
        "{}/neoforge/version/{}/download/installer.jar",
        BMCLAPI_ROOT, neoforge_version
    );
    if prefer_mirror {
        (mirror, official)
    } else {
        (official, mirror)
    }
}

/// 将 Mojang / Forge / Fabric 官方 URL 转换为 (首选 URL, 备选 URL)
pub fn resolve_mojang_download_urls(official_url: &str, prefer_mirror: bool) -> (String, String) {
    let mirror_url = official_url
        .replace(MOJANG_PISTON_META, BMCLAPI_ROOT)
        .replace(MOJANG_PISTON_DATA, BMCLAPI_ROOT)
        .replace(MOJANG_LIBRARIES, BMCLAPI_MAVEN)
        .replace(FORGE_MAVEN_OFFICIAL, BMCLAPI_MAVEN)
        .replace(NEOFORGE_MAVEN_OFFICIAL, BMCLAPI_MAVEN)
        .replace(FABRIC_MAVEN_OFFICIAL, BMCLAPI_MAVEN)
        .replace(MOJANG_RESOURCES, BMCLAPI_ASSETS);

    if prefer_mirror {
        (mirror_url, official_url.to_string())
    } else {
        (official_url.to_string(), mirror_url)
    }
}

/// 根据 Asset 对象的 SHA1 哈希构造 (首选 URL, 备选 URL)
pub fn resolve_asset_object_urls(hash: &str, prefer_mirror: bool) -> (String, String) {
    let prefix = if hash.len() >= 2 { &hash[0..2] } else { "00" };
    let official = format!("{}/{}/{}", MOJANG_RESOURCES, prefix, hash);
    let mirror = format!("{}/{}/{}", BMCLAPI_ASSETS, prefix, hash);
    if prefer_mirror {
        (mirror, official)
    } else {
        (official, mirror)
    }
}

/// 根据 Maven 子路径与原始仓库地址构造 (首选 URL, 备选 URL)
pub fn resolve_maven_library_urls(
    base_repo_url: Option<&str>,
    url_subpath: &str,
    prefer_mirror: bool,
) -> (String, String) {
    let raw_base = base_repo_url.unwrap_or(MOJANG_LIBRARIES).trim_end_matches('/');
    let official = format!("{}/{}", raw_base, url_subpath);
    let mirror = format!("{}/{}", BMCLAPI_MAVEN, url_subpath);
    if prefer_mirror {
        (mirror, official)
    } else {
        (official, mirror)
    }
}

/// Forge 版本查询地址
pub fn forge_list_url(game_version: &str) -> String {
    format!("{}/forge/minecraft/{}", BMCLAPI_ROOT, game_version)
}

/// NeoForge 版本查询地址
pub fn neoforge_list_url(game_version: &str) -> String {
    format!("{}/neoforge/list/{}", BMCLAPI_ROOT, game_version)
}

/// OptiFine 支持版本列表地址
pub fn optifine_version_list_url() -> String {
    format!("{}/optifine/versionList", BMCLAPI_ROOT)
}

/// 指定原版版本的 OptiFine 可用版本清单地址
pub fn optifine_list_url(game_version: &str) -> String {
    format!("{}/optifine/{}", BMCLAPI_ROOT, game_version)
}

/// OptiFine 安装包 JAR 下载地址
pub fn optifine_installer_url(mc_version: &str, of_type: &str, of_patch: &str) -> String {
    format!("{}/optifine/{}/{}/{}", BMCLAPI_ROOT, mc_version, of_type, of_patch)
}
