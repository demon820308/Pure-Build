//! 集中管理所有下载源、镜像地址、API 端点及产品级命名常量。
//! 业务模块只引用本模块的命名常量或辅助函数，禁止散落 URL 或魔法数字。

// 1. 产品级默认配置常量

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

/// 支持库并发下载最大并发数（对齐 PCL2 节流策略，避免瞬时高并发打爆镜像源触发 429）
pub const DEFAULT_LIBRARY_CONCURRENCY: usize = 16;

/// Assets 资源并发下载最大并发数
pub const DEFAULT_ASSET_CONCURRENCY: usize = 32;

/// BMCLAPI 镜像源建连最小间隔 (ms) —— 对齐 PCL2 ModNet.vb 针对 BMCLAPI 的请求频率控制
pub const BMCLAPI_REQUEST_INTERVAL_MS: u64 = 65;

/// 应用配置文件夹标识
pub const APP_IDENTIFIER: &str = "com.purebuild.launcher";

/// 配置文件名
pub const CONFIG_FILE_NAME: &str = "pcl_config.json";

/// 凭据文件名（存放正版登录 Profile 与 access_token，移出前端 localStorage）
pub const CREDENTIALS_FILE_NAME: &str = "pcl_credentials.json";

// 2. 微软正版认证公开端点与常量

/// 微软官方公开给 Minecraft 启动器的 OAuth Client ID
pub const MS_OAUTH_CLIENT_ID: &str = "00000000402b5328";

pub const MS_DEVICE_CODE_URL: &str = "https://login.live.com/oauth20_connect.srf";
pub const MS_TOKEN_URL: &str = "https://login.live.com/oauth20_token.srf";
pub const MS_DEFAULT_VERIFICATION_URI: &str = "https://www.microsoft.com/link";
pub const XBL_AUTH_URL: &str = "https://user.auth.xboxlive.com/user/authenticate";
pub const XSTS_AUTH_URL: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";
pub const MC_LOGIN_XBOX_URL: &str = "https://api.minecraftservices.com/authentication/login_with_xbox";
pub const MC_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";
pub const CRAFATAR_AVATAR_BASE: &str = "https://crafatar.com/avatars";

/// 默认 Yggdrasil 外置登录验证服务器地址（LittleSkin 公开 API 根地址）
pub const DEFAULT_YGGDRASIL_API_ROOT: &str = "https://littleskin.cn/api/yggdrasil";

// 3. 官方源与镜像源端点

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
pub const BMCLAPI_LIBRARIES: &str = "https://bmclapi2.bangbang93.com/libraries";
pub const BMCLAPI_ASSETS: &str = "https://bmclapi2.bangbang93.com/assets";
pub const BMCLAPI_FABRIC_META: &str = "https://bmclapi2.bangbang93.com/fabric-meta";
pub const BMCLAPI_QUILT_META: &str = "https://bmclapi2.bangbang93.com/quilt-meta";
pub const BMCLAPI_VERSION_MANIFEST_V2: &str =
    "https://bmclapi2.bangbang93.com/mc/game/version_manifest_v2.json";

/// Modrinth 社区 API 与 MCIM 国内镜像源基础地址（对齐 PCL2 ModDownload.vb DlSourceModGet）
pub const MODRINTH_API_BASE: &str = "https://api.modrinth.com/v2";
pub const MCIMIRROR_MODRINTH_API: &str = "https://mod.mcimirror.top/modrinth/v2";
pub const MODRINTH_CDN_ROOT: &str = "https://cdn.modrinth.com";
pub const MCIMIRROR_CDN_ROOT: &str = "https://mod.mcimirror.top";
pub const LAUNCHER_USER_AGENT: &str = "PCL2/2.11.2.50 (PlainCraftLauncher/2.0)";
pub const LAUNCHER_REFERER: &str = "http://408.open.pcl2.server/";

/// Adoptium / Temurin JRE 官方 API 根端点
pub const ADOPTIUM_API_BASE: &str = "https://api.adoptium.net/v3";

/// 返回指定主版本、平台与架构的 Adoptium 最新 GA JRE 资产元数据查询 URL
pub fn adoptium_assets_url(major: u32, os: &str, arch: &str) -> String {
    format!(
        "{}/assets/latest/{}/hotspot?architecture={}&image_type=jre&os={}&vendor=eclipse",
        ADOPTIUM_API_BASE, major, arch, os
    )
}

// 4. 多源 URL 组装与故障转移辅助函数

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

/// 将 Mojang / Forge / Fabric 库下载 URL 转换为多级候选 URL 列表（对齐 PCL2 DlSourceLibraryGet：同时包含 /maven、/libraries 与官方源）
pub fn resolve_mojang_library_urls(official_url: &str, prefer_mirror: bool) -> Vec<String> {
    let maven_mirror = official_url
        .replace(MOJANG_PISTON_META, BMCLAPI_MAVEN)
        .replace(MOJANG_PISTON_DATA, BMCLAPI_MAVEN)
        .replace(MOJANG_LIBRARIES, BMCLAPI_MAVEN)
        .replace(FORGE_MAVEN_OFFICIAL, BMCLAPI_MAVEN)
        .replace(NEOFORGE_MAVEN_OFFICIAL, BMCLAPI_MAVEN)
        .replace(FABRIC_MAVEN_OFFICIAL, BMCLAPI_MAVEN);

    let libs_mirror = official_url
        .replace(MOJANG_PISTON_META, BMCLAPI_LIBRARIES)
        .replace(MOJANG_PISTON_DATA, BMCLAPI_LIBRARIES)
        .replace(MOJANG_LIBRARIES, BMCLAPI_LIBRARIES);

    let mut urls = Vec::with_capacity(3);
    if prefer_mirror {
        urls.push(maven_mirror);
        if !urls.contains(&libs_mirror) {
            urls.push(libs_mirror);
        }
        let off = official_url.to_string();
        if !urls.contains(&off) {
            urls.push(off);
        }
    } else {
        urls.push(official_url.to_string());
        if !urls.contains(&maven_mirror) {
            urls.push(maven_mirror);
        }
        if !urls.contains(&libs_mirror) {
            urls.push(libs_mirror);
        }
    }
    urls
}

/// 根据 Maven 子路径与原始仓库地址构造多级候选 URL 列表（对齐 PCL2 DlSourceLibraryGet）
pub fn resolve_maven_library_urls_list(
    base_repo_url: Option<&str>,
    url_subpath: &str,
    prefer_mirror: bool,
) -> Vec<String> {
    let raw_base = base_repo_url.unwrap_or(MOJANG_LIBRARIES).trim_end_matches('/');
    let official = format!("{}/{}", raw_base, url_subpath);
    let maven_mirror = format!("{}/{}", BMCLAPI_MAVEN, url_subpath);
    let libs_mirror = format!("{}/{}", BMCLAPI_LIBRARIES, url_subpath);

    if prefer_mirror {
        vec![maven_mirror, libs_mirror, official]
    } else {
        vec![official, maven_mirror, libs_mirror]
    }
}

/// 构造 Modrinth API 请求的主备 URL（对齐 PCL2 DlSourceModGet：支持 mod.mcimirror.top 镜像与官方源互备）
pub fn resolve_modrinth_api_urls(subpath: &str, prefer_mirror: bool) -> [String; 2] {
    let clean = subpath.trim_start_matches('/');
    let official = format!("{}/{}", MODRINTH_API_BASE, clean);
    let mirror = format!("{}/{}", MCIMIRROR_MODRINTH_API, clean);
    if prefer_mirror {
        [mirror, official]
    } else {
        [official, mirror]
    }
}

/// 构造 Modrinth / 整合包资源文件下载的候选 URL 列表（对齐 PCL2 DlSourceModGet：支持 mod.mcimirror.top 镜像）
pub fn resolve_modrinth_file_urls(original_url: &str, prefer_mirror: bool) -> Vec<String> {
    let mirror_url = original_url.replace(MODRINTH_CDN_ROOT, MCIMIRROR_CDN_ROOT);
    if mirror_url == original_url {
        vec![original_url.to_string()]
    } else if prefer_mirror {
        vec![mirror_url, original_url.to_string()]
    } else {
        vec![original_url.to_string(), mirror_url]
    }
}

/// 判断给定 URL 是否属于需要建连节流与 429 豁免的镜像源（对齐 PCL2 ModNet.vb 对 bmclapi 的判定）
pub fn is_mirror_url(url: &str) -> bool {
    url.contains(BMCLAPI_ROOT) || url.contains(MCIMIRROR_CDN_ROOT)
}

/// 全局复用的 HTTP 客户端（遵循 PCL2 ModNet.vb & ModSecret.vb 规范）：
/// 1. 携带标准 `User-Agent` 与 `Referer` 请求头；
/// 2. 拆分 `connect_timeout(15s)` 与单次流数据包 `read_timeout(25s)`，彻底取消固定总时长 `.timeout()`，防止大文件下载在 30s 被掐断（PCL2 #8488）；
/// 3. 全局复用 TCP/TLS 连接池，避免多次下载耗尽端口或频繁 TLS 握手。
pub fn shared_http_client() -> &'static reqwest::Client {
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    CLIENT.get_or_init(|| {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::REFERER,
            reqwest::header::HeaderValue::from_static(LAUNCHER_REFERER),
        );
        reqwest::Client::builder()
            .user_agent(LAUNCHER_USER_AGENT)
            .default_headers(headers)
            .connect_timeout(std::time::Duration::from_secs(15))
            .read_timeout(std::time::Duration::from_secs(25))
            .pool_max_idle_per_host(32)
            .build()
            .unwrap_or_default()
    })
}

/// 针对镜像源（BMCLAPI）的全局建连节流器（对齐 PCL2 ModNet.vb 第 1744/1764 行 `Thread.Sleep(100)` 机制）：
/// 限制对镜像源的新请求发起频率，防止 16~32 并发任务在同一毫秒发起请求触发 429 限流或连接重置。
pub async fn pace_mirror_request(url: &str) {
    if !url.contains(BMCLAPI_ROOT) {
        return;
    }
    static NEXT_ALLOWED: std::sync::OnceLock<tokio::sync::Mutex<tokio::time::Instant>> =
        std::sync::OnceLock::new();
    let lock = NEXT_ALLOWED.get_or_init(|| tokio::sync::Mutex::new(tokio::time::Instant::now()));

    let wait_until = {
        let mut next = lock.lock().await;
        let now = tokio::time::Instant::now();
        let interval = std::time::Duration::from_millis(BMCLAPI_REQUEST_INTERVAL_MS);
        if *next <= now {
            *next = now + interval;
            None
        } else {
            let target = *next;
            *next = target + interval;
            Some(target)
        }
    };

    if let Some(target) = wait_until {
        tokio::time::sleep_until(target).await;
    }
}
