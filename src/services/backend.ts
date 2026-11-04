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
  java_version?: number
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
  async getPlatformInfo(): Promise<PlatformInfo> {
    return await invoke<PlatformInfo>('get_platform_info')
  },

  async fetchFabricMeta(): Promise<FabricMetaSummary> {
    return await invoke<FabricMetaSummary>('fetch_fabric_meta')
  },

  async fetchForgeLoaders(gameVersion: string): Promise<LoaderVersionItem[]> {
    return await invoke<LoaderVersionItem[]>('fetch_forge_loaders', { gameVersion })
  },

  async fetchNeoForgeLoaders(gameVersion: string): Promise<LoaderVersionItem[]> {
    return await invoke<LoaderVersionItem[]>('fetch_neoforge_loaders', { gameVersion })
  },

  async fetchOptifineGames(): Promise<string[]> {
    return await invoke<string[]>('fetch_optifine_games')
  },

  async fetchOptifineLoaders(gameVersion: string): Promise<LoaderVersionItem[]> {
    return await invoke<LoaderVersionItem[]>('fetch_optifine_loaders', { gameVersion })
  },

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

  async getResources(versionPath: string, gameDir: string, folderName: string): Promise<ResourceItem[]> {
    try {
      return await invoke<ResourceItem[]>('get_resources', { versionPath, gameDir, folderName })
    } catch (e) {
      console.warn('获取资源列表异常:', e)
      return []
    }
  },

  async getInstanceSaves(versionPath: string, gameDir: string): Promise<SaveInfo[]> {
    try {
      return await invoke<SaveInfo[]>('get_instance_saves', { versionPath, gameDir })
    } catch (e) {
      console.warn('获取存档列表异常:', e)
      return []
    }
  },

  async checkPathExists(targetPath: string): Promise<boolean> {
    try {
      return await invoke<boolean>('check_path_exists', { targetPath })
    } catch {
      return false
    }
  },

  async openFolder(folderPath: string): Promise<void> {
    await invoke('open_folder', { folderPath })
  },

  async openInstanceFolder(versionPath: string, gameDir: string, folderName: string): Promise<void> {
    await invoke('open_instance_folder', { versionPath, gameDir, folderName })
  },

  async openUrl(url: string): Promise<void> {
    await invoke('open_url', { url })
  },

  async scanMusicFolders(customDirs: string[]): Promise<MusicTrack[]> {
    try {
      return await invoke<MusicTrack[]>('scan_music_folders', { customDirs })
    } catch (e) {
      console.warn('扫描背景音乐文件夹异常:', e)
      return []
    }
  },

  async readMusicDataUrl(filePath: string): Promise<string> {
    return await invoke<string>('read_music_data_url', { filePath })
  },

  async getMicrosoftDeviceCode(): Promise<DeviceCodeResponse> {
    return await invoke<DeviceCodeResponse>('get_microsoft_device_code')
  },

  async pollMicrosoftToken(deviceCode: string): Promise<UserProfile> {
    return await invoke<UserProfile>('poll_microsoft_token', { deviceCode })
  },

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

  onLaunchProgress(callback: (p: LaunchProgress) => void) {
    return listen<LaunchProgress>('launch_progress', (event) => {
      callback(event.payload)
    })
  },

  onGameExitNormal(callback: (exitCode?: number) => void) {
    return listen<number | undefined>('game_exit_normal', (event) => {
      callback(event.payload)
    })
  },

  async killGameProcess(pid: number): Promise<void> {
    await invoke('kill_game_process', { pid })
  },

  async scanLanRooms(): Promise<LanRoomInfo[]> {
    try {
      return await invoke<LanRoomInfo[]>('scan_lan_rooms')
    } catch (e) {
      console.warn('扫描局域网房间异常:', e)
      return []
    }
  },

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

  async decodeLanInviteCode(code: string): Promise<[string, number]> {
    return await invoke<[string, number]>('decode_lan_invite_code', { code })
  },

  async pingDownloadEndpoints(): Promise<EndpointPingResult[]> {
    return await invoke<EndpointPingResult[]>('ping_download_endpoints')
  },

  async cleanGameTempCache(gameDir: string): Promise<CacheCleanResult> {
    return await invoke<CacheCleanResult>('clean_game_temp_cache', { gameDir })
  },

  onGameCrash(callback: (report: CrashAnalysisResult) => void) {
    return listen<CrashAnalysisResult>('game_crash', (event) => {
      callback(event.payload)
    })
  },

  onDownloadProgress(callback: (p: DownloadProgress) => void) {
    return listen<DownloadProgress>('download_progress', (event) => {
      callback(event.payload)
    })
  },

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

  async loadConfig(): Promise<AppConfig> {
    return await invoke<AppConfig>('load_app_config')
  },

  async saveConfig(config: AppConfig): Promise<void> {
    await invoke('save_app_config', { config })
  },

  async loadUserProfile(): Promise<UserProfile | null> {
    return await invoke<UserProfile | null>('load_user_profile')
  },

  async saveUserProfile(profile: UserProfile | null): Promise<void> {
    await invoke('save_user_profile', { profile })
  },

  async getInstalledJava(): Promise<JavaInfo[]> {
    try {
      return await invoke<JavaInfo[]>('get_installed_java')
    } catch (e) {
      console.warn('调用 get_installed_java 异常:', e)
      return []
    }
  },

  async getMinecraftVersions(gameDir: string): Promise<VersionInfo[]> {
    try {
      return await invoke<VersionInfo[]>('get_minecraft_versions', { gameDir })
    } catch (e) {
      console.warn('调用 get_minecraft_versions 异常:', e)
      return []
    }
  },

  async getInstalledMods(versionPath: string, gameDir: string): Promise<ModInfo[]> {
    try {
      return await invoke<ModInfo[]>('get_installed_mods', { versionPath, gameDir })
    } catch (e) {
      console.warn('调用 get_installed_mods 异常:', e)
      return []
    }
  },

  async toggleModState(modsDir: string, fileName: string): Promise<string> {
    return await invoke<string>('toggle_mod_state', { modsDir, fileName })
  },

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

  async fetchRemoteVersions(): Promise<RemoteVersion[]> {
    return await invoke<RemoteVersion[]>('fetch_remote_versions')
  },

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

  async ensureJavaVersion(params: {
    gameDir: string
    candidatePath: string
    majorVersion: number
  }): Promise<string> {
    return await invoke<string>('ensure_java_version', {
      gameDir: params.gameDir,
      candidatePath: params.candidatePath,
      majorVersion: params.majorVersion,
    })
  },

  async showMainWindow() {
    try {
      await invoke('show_main_window')
    } catch {
      try {
        await getCurrentWindow().show()
      } catch {
      }
    }
  },

  async minimizeWindow() {
    try {
      await invoke('minimize_window')
    } catch {
      try {
        await getCurrentWindow().minimize()
      } catch (e) {
        console.warn('最小化窗口失败:', e)
      }
    }
  },

  async closeWindow() {
    try {
      await invoke('close_window')
    } catch {
      try {
        await getCurrentWindow().close()
      } catch (e) {
        console.warn('关闭窗口失败:', e)
      }
    }
  },

  async startDragging() {
    try {
      await getCurrentWindow().startDragging()
    } catch (e) {
    }
  },

  async reportStartupPerf(t1: number, t2: number, t3: number): Promise<void> {
    try {
      await invoke('report_startup_perf', { t1, t2, t3 })
    } catch {
      // 容错忽略
    }
  },
}
