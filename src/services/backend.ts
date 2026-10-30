import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'

export interface JavaInfo {
  path: string
  version: string
  major_version: number
  is_64_bit: boolean
  is_jdk: boolean
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
}

export interface AppConfig {
  game_dir: string
  max_memory_mb: number
  player_name: string
  java_path?: string
  window_width: number
  window_height: number
  theme_hue: number
}

export const backend = {
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

  // 资源包与光影包
  async getResources(versionPath: string, gameDir: string, folderName: string): Promise<ResourceItem[]> {
    try {
      return await invoke<ResourceItem[]>('get_resources', { versionPath, gameDir, folderName })
    } catch (e) {
      console.warn('获取资源列表异常:', e)
      return []
    }
  },

  async openInExplorer(folderPath: string): Promise<void> {
    await invoke('open_in_explorer', { folderPath })
  },

  // 调用系统默认浏览器打开外部网页链接
  async openUrl(url: string): Promise<void> {
    await invoke('open_url', { url })
  },

  // 微软设备码登录
  async getMicrosoftDeviceCode(): Promise<DeviceCodeResponse> {
    return await invoke<DeviceCodeResponse>('get_microsoft_device_code')
  },

  async pollMicrosoftToken(deviceCode: string): Promise<UserProfile> {
    return await invoke<UserProfile>('poll_microsoft_token', { deviceCode })
  },

  // 监听启动进度事件
  onLaunchProgress(callback: (p: LaunchProgress) => void) {
    return listen<LaunchProgress>('launch_progress', (event) => {
      callback(event.payload)
    })
  },

  // 监听游戏正常退出
  onGameExitNormal(callback: () => void) {
    return listen('game_exit_normal', () => {
      callback()
    })
  },

  // 强制终止游戏进程
  async killGameProcess(pid: number): Promise<void> {
    await invoke('kill_game_process', { pid })
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

  // 下载官方 Minecraft 服务端并自动生成启动脚本与 EULA 配置
  async downloadMinecraftServer(params: {
    targetDir: string
    versionId: string
    metaUrl: string
  }): Promise<void> {
    await invoke('download_minecraft_server', {
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

  // 从 BMCLAPI / 官方拉取全量可下载版本清单
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
