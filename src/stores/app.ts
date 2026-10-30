import { defineStore } from 'pinia'
import { ref } from 'vue'
import { backend } from '@/services/backend'
import type { VersionInfo, AppConfig, UserProfile } from '@/services/backend'

export const useAppStore = defineStore('app', () => {
  // 当前激活的页面 Tab ('launch' | 'download' | 'instance' | 'setup' | 'link')
  const currentTab = ref('launch')

  // 当前主题色色相 (0 ~ 360)
  const themeHue = ref(205)

  // 默认 Minecraft 游戏路径
  const gameDir = ref('D:\\Program Files (x86)\\.minecraft')

  // 内存分配 (MB)
  const maxMemoryMb = ref(4096)

  // 分辨率
  const windowWidth = ref(854)
  const windowHeight = ref(480)

  // 当前选中的真实版本
  const currentVersion = ref<VersionInfo | null>(null)

  // 本地真实已安装版本列表
  const installedVersions = ref<VersionInfo[]>([])

  // 当前登录的用户身份 (正版档案或离线档案)
  const userProfile = ref<UserProfile | null>(null)

  // 离线/自定义玩家昵称
  const playerName = ref('Steve')

  // 设置主题色相并保存
  const setThemeHue = (hue: number) => {
    themeHue.value = hue
    document.documentElement.style.setProperty('--pcl-hue', hue.toString())
    saveCurrentConfig()
  }

  // 初始化加载本地配置
  const initConfig = async () => {
    try {
      const cfg = await backend.loadConfig()
      gameDir.value = cfg.game_dir
      maxMemoryMb.value = cfg.max_memory_mb
      playerName.value = cfg.player_name
      windowWidth.value = cfg.window_width
      windowHeight.value = cfg.window_height
      themeHue.value = cfg.theme_hue
      document.documentElement.style.setProperty('--pcl-hue', cfg.theme_hue.toString())

      // 读取本地保存的登录 Profile 缓存
      const savedProfile = localStorage.getItem('pcl_user_profile')
      if (savedProfile) {
        userProfile.value = JSON.parse(savedProfile)
      }
      // 扫描本地已安装的版本
      await refreshInstalledVersions()
    } catch (e) {
      console.warn('加载配置文件失败，使用默认配置:', e)
    }
  }

  // 刷新本地已安装版本列表
  const refreshInstalledVersions = async () => {
    try {
      const list = await backend.getMinecraftVersions(gameDir.value)
      installedVersions.value = list
      if (!currentVersion.value && list.length > 0) {
        currentVersion.value = list[0]
      }
    } catch (e) {
      console.warn('获取已安装版本失败:', e)
    }
  }

  // 保存当前用户 Profile
  const setUserProfile = (profile: UserProfile | null) => {
    userProfile.value = profile
    if (profile) {
      playerName.value = profile.name
      localStorage.setItem('pcl_user_profile', JSON.stringify(profile))
    } else {
      localStorage.removeItem('pcl_user_profile')
    }
  }

  // 触发保存全局配置
  const saveCurrentConfig = async () => {
    const cfg: AppConfig = {
      game_dir: gameDir.value,
      max_memory_mb: maxMemoryMb.value,
      player_name: playerName.value,
      window_width: windowWidth.value,
      window_height: windowHeight.value,
      theme_hue: themeHue.value,
    }
    try {
      await backend.saveConfig(cfg)
    } catch (e) {
      console.error('保存配置失败:', e)
    }
  }

  return {
    currentTab,
    themeHue,
    gameDir,
    maxMemoryMb,
    windowWidth,
    windowHeight,
    currentVersion,
    installedVersions,
    userProfile,
    playerName,
    setThemeHue,
    initConfig,
    refreshInstalledVersions,
    setUserProfile,
    saveCurrentConfig,
  }
})
