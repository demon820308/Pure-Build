import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'

export interface PlatformInfo {
  os: string
  arch: string
  default_game_dir: string
  default_music_dir: string
  server_script_name: string
  is_macos: boolean
}

export interface MusicTrack {
  name: string
  file_name: string
  path: string
  folder_path: string
  size_mb: number
}

export interface JavaInfo {
  path: string
  version: string
  major_version: number
  is_64_bit: boolean
  is_jdk: boolean
  arch?: string
}

export interface VersionInfo {
  id: string
  name: string
  version_type: string
  loader?: string
  release_time?: string
  main_class?: string
  has_indie: boolean
  path: string
}

export interface ModInfo {
  file_name: string
  name: string
  mod_id: string
  version: string
  description?: string
  authors?: string[]
  is_enabled: boolean
  icon_base64?: string
}

export interface ResourceItem {
  file_name: string
  name: string
  size_mb: number
  is_enabled: boolean
  is_directory: boolean
}

export interface SaveInfo {
  folder_name: string
  size_mb: number
  path: string
  has_level_dat: boolean
}

export interface RemoteVersion {
  id: string
  type: 'release' | 'snapshot' | 'old_beta' | 'old_alpha'
  url: string
  time: string
  releaseTime: string
  sha1: string
}

export interface LaunchProgress {
  step: string
  percentage: number
  pid?: number
  is_finished: boolean
  error?: string
}

export interface CrashAnalysisResult {
  exit_code?: number
  title: string
  reason: string
  suggestion: string
  log_snippet: string
  report_file?: string
}

export interface LoaderVersionItem {
  version: string
  stable: boolean
  description?: string
}

export interface FabricMetaSummary {
  supported_games: string[]
  loaders: LoaderVersionItem[]
}

export interface ModrinthProject {
  project_id: string
  slug: string
  title: string
  description: string
  categories: string[]
  downloads: number
  follows: number
  icon_url?: string
  author: string
  versions: string[]
}

export interface DownloadProgress {
  version_id: string
  vanilla_version?: string
  step: string
  percentage: number
  speed_text: string
  is_finished: boolean
  error?: string
}

export interface DeviceCodeResponse {
  user_code: string
  device_code: string
  verification_uri: string
  expires_in: number
  interval: number
}

export interface UserProfile {
  name: string
  uuid: string
  access_token: string
  avatar_url: string
  auth_type?: string
  auth_server?: string
}

export interface AppConfig {
  game_dir: string
  max_memory_mb: number
  player_name: string
  java_path?: string
  window_width: number
  window_height: number
  theme_hue: number
  prefer_mirror?: boolean
  custom_bg_url?: string
  bg_blur?: number
  bg_opacity?: number
  custom_homepage_notice?: string
  ui_style?: 'classic' | 'modern'
  hide_yggdrasil_login?: boolean
  music_dirs?: string[]
  music_volume?: number
  music_play_mode?: 'shuffle' | 'list' | 'single'
  music_auto_play?: boolean
  music_pause_on_launch?: boolean
  music_eq_preset?: string
  music_eq_custom?: number[]
  music_compressor?: boolean
  music_crossfade?: boolean
}

export interface LanRoomInfo {
  motd: string
  host_ip: string
  port: number
  server_address: string
  invite_code: string
}

export interface EndpointPingResult {
  name: string
  category: string
  latency_ms?: number
  status_ok: boolean
}

export interface CacheCleanResult {
  removed_items: number
  freed_bytes: number
}

export const backend = {
  // 获取当前操作系统与平台推导默认信息
  async getPlatformInfo(): Promise<PlatformInfo> {
    return await invoke<PlatformInfo>('get_platform_info')
  },

  // 获取全量 Fabric 元数据（支持版本列表及 Loader 列表）
  async fetchFabricMeta(): Promise<FabricMetaSummary> {
    return await invoke<FabricMetaSummary>('fetch_fabric_meta')
  },

  // 获取 Forge 可选加载器列表
  async fetchForgeLoaders(gameVersion: string): Promise<LoaderVersionItem[]> {
    return await invoke<LoaderVersionItem[]>('fetch_forge_loaders', { gameVersion })
  },

  // 获取 NeoForge 可选加载器列表
  async fetchNeoForgeLoaders(gameVersion: string): Promise<LoaderVersionItem[]> {
    return await invoke<LoaderVersionItem[]>('fetch_neoforge_loaders', { gameVersion })
  },

  // 获取支持 OptiFine 的原版游戏版本列表
  async fetchOptifineGames(): Promise<string[]> {
    return await invoke<string[]>('fetch_optifine_games')
  },

  // 搜索 Modrinth 社区资源 (Mods, Shaders, Resourcepacks)
  async searchModrinthResources(params: {
    query: string
    projectType: 'mod' | 'shader' | 'resourcepack' | 'modpack'
    gameVersion?: string
    loader?: string
  }): Promise<ModrinthProject[]> {
    try {
      return await invoke<ModrinthProject[]>('search_modrinth_resources', {
        query: params.query,
        projectType: params.projectType,
        gameVersion: params.gameVersion,
        loader: params.loader,
      })
    } catch (e) {
      console.warn('搜索社区资源异常:', e)
      return []
    }
  },

  // 下载并安装 Modrinth 社区资源
  async installModrinthResource(params: {
    gameDir: string
    versionPath?: string
    projectId: string
    projectType: string
    gameVersion?: string
    loader?: string
  }): Promise<string> {
    return await invoke<string>('install_modrinth_resource', {
      gameDir: params.gameDir,
      versionPath: params.versionPath,
      projectId: params.projectId,
      projectType: params.projectType,
      gameVersion: params.gameVersion,
      loader: params.loader,
    })
  },

  // 导入本地 .mrpack 整合包
  async importModpack(params: {
    gameDir: string
    mrpackPath: string
    customInstanceName?: string
  }): Promise<string> {
    return await invoke<string>('import_modpack', {
      gameDir: params.gameDir,
      mrpackPath: params.mrpackPath,
      customInstanceName: params.customInstanceName || null,
    })
  },

  // 导出本地版本实例为 .mrpack 整合包
  async exportModpack(params: {
    versionPath: string
    gameDir: string
    versionId: string
    packName: string
    packVersion: string
    outputFile: string
  }): Promise<string> {
    return await invoke<string>('export_modpack', {
      versionPath: params.versionPath,
      gameDir: params.gameDir,
      versionId: params.versionId,
      packName: params.packName,
      packVersion: params.packVersion,
      outputFile: params.outputFile,
    })
  },

  // 资源包与光影包
  async getResources(versionPath: string, gameDir: string, folderName: string): Promise<ResourceItem[]> {
    try {
      return await invoke<ResourceItem[]>('get_resources', { versionPath, gameDir, folderName })
    } catch (e) {
      console.warn('获取资源列表异常:', e)
      return []
    }
  },

  // 单人世界存档
  async getInstanceSaves(versionPath: string, gameDir: string): Promise<SaveInfo[]> {
    try {
      return await invoke<SaveInfo[]>('get_instance_saves', { versionPath, gameDir })
    } catch (e) {
      console.warn('获取存档列表异常:', e)
      return []
    }
  },

  async openFolder(folderPath: string): Promise<void> {
    await invoke('open_folder', { folderPath })
  },

  async openInstanceFolder(versionPath: string, gameDir: string, folderName: string): Promise<void> {
    await invoke('open_instance_folder', { versionPath, gameDir, folderName })
  },

  // 调用系统默认浏览器打开外部网页链接
  async openUrl(url: string): Promise<void> {
    await invoke('open_url', { url })
  },

  // 递归扫描默认音乐库与自定义音乐文件夹内的所有音频曲目
  async scanMusicFolders(customDirs: string[]): Promise<MusicTrack[]> {
    try {
      return await invoke<MusicTrack[]>('scan_music_folders', { customDirs })
    } catch (e) {
      console.warn('扫描背景音乐文件夹异常:', e)
      return []
    }
  },

  // 读取本地音频文件为 Base64 Data URL
  async readMusicDataUrl(filePath: string): Promise<string> {
    return await invoke<string>('read_music_data_url', { filePath })
  },

  // 微软设备码登录
  async getMicrosoftDeviceCode(): Promise<DeviceCodeResponse> {
    return await invoke<DeviceCodeResponse>('get_microsoft_device_code')
  },

  async pollMicrosoftToken(deviceCode: string): Promise<UserProfile> {
    return await invoke<UserProfile>('poll_microsoft_token', { deviceCode })
  },

  // 外置登录 (Yggdrasil / LittleSkin)
  async loginYggdrasilAccount(params: {
    apiRoot: string
    username: string
    password: string
  }): Promise<UserProfile> {
    return await invoke<UserProfile>('login_yggdrasil_account', {
      apiRoot: params.apiRoot,
      username: params.username,
      password: params.password,
    })
  },

  // 监听启动进度事件
  onLaunchProgress(callback: (p: LaunchProgress) => void) {
    return listen<LaunchProgress>('launch_progress', (event) => {
      callback(event.payload)
    })
  },

  // 监听游戏正常退出
  onGameExitNormal(callback: (exitCode?: number) => void) {
    return listen<number | undefined>('game_exit_normal', (event) => {
      callback(event.payload)
    })
  },

  // 强制终止游戏进程
  async killGameProcess(pid: number): Promise<void> {
    await invoke('kill_game_process', { pid })
  },

  // 局域网联机大厅：扫描当前局域网内的 Minecraft 开放世界广播
  async scanLanRooms(): Promise<LanRoomInfo[]> {
    try {
      return await invoke<LanRoomInfo[]>('scan_lan_rooms')
    } catch (e) {
      console.warn('扫描局域网房间异常:', e)
      return []
    }
  },

  // 局域网联机大厅：向局域网发送房间广播并生成 PCL2 联机邀请码
  async broadcastLanRoom(params: {
    motd: string
    hostIp: string
    port: number
  }): Promise<LanRoomInfo> {
    return await invoke<LanRoomInfo>('broadcast_lan_room', {
      motd: params.motd,
      hostIp: params.hostIp,
      port: params.port,
    })
  },

  // 局域网联机大厅：解码 PCL2 联机邀请码
  async decodeLanInviteCode(code: string): Promise<[string, number]> {
    return await invoke<[string, number]>('decode_lan_invite_code', { code })
  },

  // 百宝箱：检测各大官方源与镜像源响应延迟
  async pingDownloadEndpoints(): Promise<EndpointPingResult[]> {
    return await invoke<EndpointPingResult[]>('ping_download_endpoints')
  },

  // 百宝箱：清理游戏目录下的临时 Natives 解压目录与历史归档日志
  async cleanGameTempCache(gameDir: string): Promise<CacheCleanResult> {
    return await invoke<CacheCleanResult>('clean_game_temp_cache', { gameDir })
  },

  // 监听游戏崩溃诊断报告
  onGameCrash(callback: (report: CrashAnalysisResult) => void) {
    return listen<CrashAnalysisResult>('game_crash', (event) => {
      callback(event.payload)
    })
  },

  // 监听下载进度事件
  onDownloadProgress(callback: (p: DownloadProgress) => void) {
    return listen<DownloadProgress>('download_progress', (event) => {
      callback(event.payload)
    })
  },

  // 下载官方 Minecraft 服务端并自动生成跨平台启动脚本与 EULA 配置（返回生成的脚本文件名）
  async downloadMinecraftServer(params: {
    targetDir: string
    versionId: string
    metaUrl: string
  }): Promise<string> {
    return await invoke<string>('download_minecraft_server', {
      targetDir: params.targetDir,
      versionId: params.versionId,
      metaUrl: params.metaUrl,
    })
  },

  // 触发安装版本 (按原版 PCL 规范支持自定义命名与 Loader 选择)
  async startInstallVersion(params: {
    gameDir: string
    vanillaVersion: string
    targetName?: string
    metaUrl: string
    loaderType?: string
    loaderVersion?: string
  }): Promise<void> {
    await invoke('start_install_version', {
      gameDir: params.gameDir,
      vanillaVersion: params.vanillaVersion,
      targetName: params.targetName || null,
      metaUrl: params.metaUrl,
      loaderType: params.loaderType || null,
      loaderVersion: params.loaderVersion || null,
    })
  },

  // 读取与保存配置
  async loadConfig(): Promise<AppConfig> {
    return await invoke<AppConfig>('load_app_config')
  },

  async saveConfig(config: AppConfig): Promise<void> {
    await invoke('save_app_config', { config })
  },

  // 读取与保存正版凭据 Profile (存储在后端 app_config_dir，不存 localStorage)
  async loadUserProfile(): Promise<UserProfile | null> {
    return await invoke<UserProfile | null>('load_user_profile')
  },

  async saveUserProfile(profile: UserProfile | null): Promise<void> {
    await invoke('save_user_profile', { profile })
  },

  // 获取系统扫描到的所有 Java
  async getInstalledJava(): Promise<JavaInfo[]> {
    try {
      return await invoke<JavaInfo[]>('get_installed_java')
    } catch (e) {
      console.warn('调用 get_installed_java 异常:', e)
      return []
    }
  },

  // 扫描 Minecraft 目录下的真实版本列表
  async getMinecraftVersions(gameDir: string): Promise<VersionInfo[]> {
    try {
      return await invoke<VersionInfo[]>('get_minecraft_versions', { gameDir })
    } catch (e) {
      console.warn('调用 get_minecraft_versions 异常:', e)
      return []
    }
  },

  // 扫描指定版本的 Mods 列表
  async getInstalledMods(versionPath: string, gameDir: string): Promise<ModInfo[]> {
    try {
      return await invoke<ModInfo[]>('get_installed_mods', { versionPath, gameDir })
    } catch (e) {
      console.warn('调用 get_installed_mods 异常:', e)
      return []
    }
  },

  // 切换 Mod 启停状态
  async toggleModState(modsDir: string, fileName: string): Promise<string> {
    return await invoke<string>('toggle_mod_state', { modsDir, fileName })
  },

  // 按版本路径与目录名切换资源启停状态（后端通过 Path::join 处理）
  async toggleInstanceItem(
    versionPath: string,
    gameDir: string,
    folderName: string,
    fileName: string
  ): Promise<string> {
    return await invoke<string>('toggle_instance_item', {
      versionPath,
      gameDir,
      folderName,
      fileName,
    })
  },

  // 删除指定版本目录中的 Mod 或资源包文件
  async deleteInstanceItem(
    versionPath: string,
    gameDir: string,
    folderName: string,
    fileName: string
  ): Promise<void> {
    await invoke('delete_instance_item', {
      versionPath,
      gameDir,
      folderName,
      fileName,
    })
  },

  // 从下载源拉取全量可下载版本清单
  async fetchRemoteVersions(): Promise<RemoteVersion[]> {
    return await invoke<RemoteVersion[]>('fetch_remote_versions')
  },

  // 启动 Minecraft (支持正版 UUID 与 Token)
  async launchMinecraft(params: {
    gameDir: string
    versionId: string
    javaPath: string
    playerName: string
    maxMemoryMb: number
    uuid?: string
    accessToken?: string
  }): Promise<string> {
    return await invoke<string>('launch_minecraft', {
      gameDir: params.gameDir,
      versionId: params.versionId,
      javaPath: params.javaPath,
      playerName: params.playerName,
      maxMemoryMb: params.maxMemoryMb,
      uuid: params.uuid,
      accessToken: params.accessToken,
    })
  },

  // 窗口操作
  async minimizeWindow() {
    try {
      await getCurrentWindow().minimize()
    } catch {
      console.log('Window minimize')
    }
  },

  async closeWindow() {
    try {
      await getCurrentWindow().close()
    } catch {
      console.log('Window close')
    }
  },

  async startDragging() {
    try {
      await getCurrentWindow().startDragging()
    } catch {
      // 浏览器环境不支持
    }
  },
}
