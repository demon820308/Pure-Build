import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { AppConfig, PlatformInfo } from '../services/backend'
import { backend } from '../services/backend'

export const useConfigStore = defineStore('config', () => {
  const platformInfo = ref<PlatformInfo | null>(null)

  const themeHue = ref(205)
  const gameDir = ref('')
  const maxMemoryMb = ref(4096)
  const playerName = ref('Steve')
  const windowWidth = ref(854)
  const windowHeight = ref(480)
  const preferMirror = ref(true)

  const customBgUrl = ref('')
  const bgBlur = ref(8)
  const bgOpacity = ref(85)
  const customHomepageNotice = ref('')
  const uiStyle = ref<'classic' | 'modern'>('classic')
  const hideYggdrasilLogin = ref(true)

  const musicDirs = ref<string[]>([])
  const musicVolume = ref(45)
  const musicPlayMode = ref<'shuffle' | 'list' | 'single'>('shuffle')
  const musicAutoPlay = ref(true)
  const musicPauseOnLaunch = ref(true)
  const musicEqPreset = ref<string>('flat')
  const musicEqCustom = ref<number[]>([0, 0, 0, 0, 0])
  const musicCompressor = ref<boolean>(true)
  const musicCrossfade = ref<boolean>(true)

  const setThemeHue = (hue: number) => {
    themeHue.value = hue
    document.documentElement.style.setProperty('--pcl-hue', hue.toString())
    void saveCurrentConfig()
  }

  const setUiStyle = (style: 'classic' | 'modern') => {
    uiStyle.value = style
    document.documentElement.setAttribute('data-ui-style', style)
    void saveCurrentConfig()
  }

  const applyConfig = (cfg: AppConfig, pInfo: PlatformInfo) => {
    platformInfo.value = pInfo
    gameDir.value = cfg.game_dir || pInfo.default_game_dir
    maxMemoryMb.value = cfg.max_memory_mb
    playerName.value = cfg.player_name
    windowWidth.value = cfg.window_width
    windowHeight.value = cfg.window_height
    themeHue.value = cfg.theme_hue
    preferMirror.value = cfg.prefer_mirror ?? true
    customBgUrl.value = cfg.custom_bg_url || ''
    bgBlur.value = cfg.bg_blur ?? 8
    bgOpacity.value = cfg.bg_opacity ?? 85
    customHomepageNotice.value = cfg.custom_homepage_notice || ''
    uiStyle.value = cfg.ui_style === 'modern' ? 'modern' : 'classic'
    hideYggdrasilLogin.value = cfg.hide_yggdrasil_login ?? true

    musicDirs.value = Array.isArray(cfg.music_dirs) ? cfg.music_dirs : []
    musicVolume.value = cfg.music_volume ?? 45
    musicPlayMode.value =
      cfg.music_play_mode === 'list' || cfg.music_play_mode === 'single'
        ? cfg.music_play_mode
        : 'shuffle'
    musicAutoPlay.value = cfg.music_auto_play ?? true
    musicPauseOnLaunch.value = cfg.music_pause_on_launch ?? true
    musicEqPreset.value = cfg.music_eq_preset || 'flat'
    musicEqCustom.value =
      Array.isArray(cfg.music_eq_custom) && cfg.music_eq_custom.length === 5
        ? cfg.music_eq_custom
        : [0, 0, 0, 0, 0]
    musicCompressor.value = cfg.music_compressor ?? true
    musicCrossfade.value = cfg.music_crossfade ?? true

    document.documentElement.style.setProperty('--pcl-hue', cfg.theme_hue.toString())
    document.documentElement.setAttribute('data-ui-style', uiStyle.value)
  }

  const saveCurrentConfig = async () => {
    const cfg: AppConfig = {
      game_dir: gameDir.value,
      max_memory_mb: maxMemoryMb.value,
      player_name: playerName.value,
      window_width: windowWidth.value,
      window_height: windowHeight.value,
      theme_hue: themeHue.value,
      prefer_mirror: preferMirror.value,
      custom_bg_url: customBgUrl.value || undefined,
      bg_blur: bgBlur.value,
      bg_opacity: bgOpacity.value,
      custom_homepage_notice: customHomepageNotice.value || undefined,
      ui_style: uiStyle.value,
      hide_yggdrasil_login: hideYggdrasilLogin.value,
      music_dirs: musicDirs.value,
      music_volume: musicVolume.value,
      music_play_mode: musicPlayMode.value,
      music_auto_play: musicAutoPlay.value,
      music_pause_on_launch: musicPauseOnLaunch.value,
      music_eq_preset: musicEqPreset.value,
      music_eq_custom: musicEqCustom.value,
      music_compressor: musicCompressor.value,
      music_crossfade: musicCrossfade.value,
    }
    try {
      await backend.saveConfig(cfg)
    } catch (e) {
      console.error('保存配置失败:', e)
    }
  }

  return {
    platformInfo,
    themeHue,
    gameDir,
    maxMemoryMb,
    playerName,
    windowWidth,
    windowHeight,
    preferMirror,
    customBgUrl,
    bgBlur,
    bgOpacity,
    customHomepageNotice,
    uiStyle,
    hideYggdrasilLogin,
    musicDirs,
    musicVolume,
    musicPlayMode,
    musicAutoPlay,
    musicPauseOnLaunch,
    musicEqPreset,
    musicEqCustom,
    musicCompressor,
    musicCrossfade,
    setThemeHue,
    setUiStyle,
    applyConfig,
    saveCurrentConfig,
  }
})
