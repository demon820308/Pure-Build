import { defineStore } from 'pinia'
import { toRef } from 'vue'
import { backend } from '../services/backend'
import { audioEngine } from '../services/audioEngine'
import { useConfigStore } from './config'
import { useVersionsStore } from './versions'
import { useMusicStore } from './music'
import { useSessionStore } from './session'
import { shouldTriggerFocusReconcile } from '../utils/coreLogic'

export const useAppStore = defineStore('app', () => {
  const config = useConfigStore()
  const versions = useVersionsStore()
  const music = useMusicStore()
  const session = useSessionStore()

  let initPromise: Promise<void> | null = null
  let lastFocusReconcile = 0

  const refreshInstalledVersions = async () => {
    await versions.refreshInstalledVersions(config.gameDir, session.showNotification)
  }

  const validateTargetExists = async (targetPath?: string | null, targetLabel = '目标项'): Promise<boolean> => {
    if (!targetPath || !targetPath.trim()) {
      session.showNotification(`操作中止：${targetLabel}路径无效或未指定`)
      return false
    }
    try {
      if (!await backend.checkPathExists(targetPath)) {
        session.showNotification(`【${targetLabel}】在本地磁盘已不存在，已自动刷新同步状态`)
        await refreshInstalledVersions()
        if (config.musicDirs.length > 0) await music.refreshMusicPlaylist(false, false)
        versions.triggerInstanceRefresh()
        return false
      }
      return true
    } catch {
      return true
    }
  }

  const reconcileOnFocus = async () => {
    const now = Date.now()
    if (!shouldTriggerFocusReconcile(now, lastFocusReconcile)) return
    lastFocusReconcile = now
    await refreshInstalledVersions()
    if (config.musicDirs.length > 0) await music.refreshMusicPlaylist(false, false)
    versions.triggerInstanceRefresh()
  }

  const initConfig = async () => {
    if (initPromise) return initPromise
    initPromise = (async () => {
      try {
        const [pInfo, cfg, savedProfile] = await Promise.all([
          backend.getPlatformInfo(),
          backend.loadConfig(),
          backend.loadUserProfile(),
        ])
        config.applyConfig(cfg, pInfo)
        audioEngine.setVolume(config.musicVolume)
        audioEngine.setPreset(config.musicEqPreset, config.musicEqCustom)
        audioEngine.setCompressorEnabled(config.musicCompressor)
        audioEngine.setCrossfadeEnabled(config.musicCrossfade)
        if (savedProfile && !(config.hideYggdrasilLogin && savedProfile.auth_type === 'yggdrasil')) {
          session.userProfile = savedProfile
          config.playerName = savedProfile.name
        }
        void refreshInstalledVersions()
        void music.refreshMusicPlaylist(false, true)
      } catch (e) {
        console.warn('加载配置失败:', e)
      }
    })()
    return initPromise
  }

  const setUserProfile = async (profile: typeof session.userProfile) => {
    await session.setUserProfile(profile)
    if (profile) config.playerName = profile.name
  }

  const updateDownloadProgress = (progress: Parameters<typeof session.updateDownloadProgress>[0]) => {
    session.updateDownloadProgress(progress, () => void refreshInstalledVersions())
  }

  return {
    platformInfo: toRef(config, 'platformInfo'), themeHue: toRef(config, 'themeHue'),
    gameDir: toRef(config, 'gameDir'), maxMemoryMb: toRef(config, 'maxMemoryMb'),
    playerName: toRef(config, 'playerName'), windowWidth: toRef(config, 'windowWidth'),
    windowHeight: toRef(config, 'windowHeight'), preferMirror: toRef(config, 'preferMirror'),
    customBgUrl: toRef(config, 'customBgUrl'), bgBlur: toRef(config, 'bgBlur'),
    bgOpacity: toRef(config, 'bgOpacity'), customHomepageNotice: toRef(config, 'customHomepageNotice'),
    uiStyle: toRef(config, 'uiStyle'), hideYggdrasilLogin: toRef(config, 'hideYggdrasilLogin'),
    musicDirs: toRef(config, 'musicDirs'), musicVolume: toRef(config, 'musicVolume'),
    musicPlayMode: toRef(config, 'musicPlayMode'), musicAutoPlay: toRef(config, 'musicAutoPlay'),
    musicPauseOnLaunch: toRef(config, 'musicPauseOnLaunch'), musicEqPreset: toRef(config, 'musicEqPreset'),
    musicEqCustom: toRef(config, 'musicEqCustom'), musicCompressor: toRef(config, 'musicCompressor'),
    musicCrossfade: toRef(config, 'musicCrossfade'),
    setThemeHue: config.setThemeHue, setUiStyle: config.setUiStyle, saveCurrentConfig: config.saveCurrentConfig,
    installedVersions: toRef(versions, 'installedVersions'), currentVersion: toRef(versions, 'currentVersion'),
    instanceRefreshTrigger: toRef(versions, 'instanceRefreshTrigger'),
    refreshInstalledVersions, setCurrentVersion: versions.setCurrentVersion, triggerInstanceRefresh: versions.triggerInstanceRefresh,
    musicTracks: toRef(music, 'musicTracks'), currentTrack: toRef(music, 'currentTrack'),
    isMusicPlaying: toRef(music, 'isMusicPlaying'), musicProgress: toRef(music, 'musicProgress'),
    musicCurrentTime: toRef(music, 'musicCurrentTime'), musicDuration: toRef(music, 'musicDuration'),
    musicError: toRef(music, 'musicError'),
    playTrack: music.playTrack, toggleMusicPlay: music.toggleMusicPlay,
    playNextTrack: music.playNextTrack, playPrevTrack: music.playPrevTrack,
    seekMusic: music.seekMusic, setMusicVolume: music.setMusicVolume,
    setMusicPlayMode: music.setMusicPlayMode, cycleMusicPlayMode: music.cycleMusicPlayMode,
    setMusicEqPreset: music.setMusicEqPreset, setMusicEqCustomBand: music.setMusicEqCustomBand,
    resetMusicEqCustom: music.resetMusicEqCustom, setMusicCompressor: music.setMusicCompressor,
    setMusicCrossfade: music.setMusicCrossfade, getSpectrumData: music.getSpectrumData,
    refreshMusicPlaylist: music.refreshMusicPlaylist, addMusicFolder: music.addMusicFolder,
    removeMusicFolder: music.removeMusicFolder, handleMusicOnGameLaunch: music.handleMusicOnGameLaunch,
    handleMusicOnGameExit: music.handleMusicOnGameExit,
    userProfile: toRef(session, 'userProfile'), isLoginModalOpen: toRef(session, 'isLoginModalOpen'),
    currentTab: toRef(session, 'currentTab'), runningPid: toRef(session, 'runningPid'),
    isLaunching: toRef(session, 'isLaunching'), launchProgress: toRef(session, 'launchProgress'),
    downloadTask: toRef(session, 'downloadTask'), isDownloading: toRef(session, 'isDownloading'),
    notificationMessage: toRef(session, 'notificationMessage'),
    showNotification: session.showNotification,
    setUserProfile, resetGameState: session.resetGameState,
    updateDownloadProgress, clearDownloadTask: session.clearDownloadTask,
    validateTargetExists, reconcileOnFocus, initConfig,
  }
})
