import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { backend } from '@/services/backend'
import type { VersionInfo, AppConfig, UserProfile, PlatformInfo, MusicTrack, LaunchProgress, DownloadProgress } from '@/services/backend'
import { audioEngine } from '@/services/audioEngine'

export const useAppStore = defineStore('app', () => {
  const currentTab = ref('launch')

  const platformInfo = ref<PlatformInfo | null>(null)

  const themeHue = ref(205)

  const gameDir = ref('')

  const maxMemoryMb = ref(4096)

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

  const musicTracks = ref<MusicTrack[]>([])
  const currentTrack = ref<MusicTrack | null>(null)
  const isMusicPlaying = ref(false)
  const musicProgress = ref(0)
  const musicCurrentTime = ref(0)
  const musicDuration = ref(0)
  const musicError = ref('')

  let currentBlobUrl: string | null = null
  let shuffleQueue: string[] = []
  let historyStack: string[] = []
  let wasPlayingBeforeLaunch = false
  let consecutiveErrors = 0

  audioEngine.onTimeUpdate = (currentTime, duration, progress) => {
    musicCurrentTime.value = currentTime
    musicDuration.value = duration
    musicProgress.value = progress
  }

  audioEngine.onLoadedMetadata = (duration) => {
    musicDuration.value = duration
  }

  audioEngine.onEnded = () => {
    const next = pickNextTrack(true)
    if (next) {
      playTrack(next, true, true)
    } else {
      isMusicPlaying.value = false
    }
  }

  audioEngine.onError = () => {
    consecutiveErrors++
    musicError.value = `无法播放: ${currentTrack.value?.name || '未知曲目'}`
    isMusicPlaying.value = false
    if (consecutiveErrors < musicTracks.value.length && musicTracks.value.length > 1) {
      const next = pickNextTrack(false)
      if (next) {
        setTimeout(() => playTrack(next, true, false), 600)
      }
    }
  }

  // 将 Base64 Data URL 转换为浏览器 Blob URL，支持进度拖拽并在切歌时释放内存
  const dataUrlToBlobUrl = (dataUrl: string): string => {
    const commaIdx = dataUrl.indexOf(',')
    if (commaIdx === -1) return dataUrl
    const header = dataUrl.slice(0, commaIdx)
    const base64 = dataUrl.slice(commaIdx + 1)
    const mimeMatch = header.match(/data:([^;]+);base64/)
    const mime = mimeMatch ? mimeMatch[1] : 'audio/mpeg'
    const binary = atob(base64)
    const bytes = new Uint8Array(binary.length)
    for (let i = 0; i < binary.length; i++) {
      bytes[i] = binary.charCodeAt(i)
    }
    const blob = new Blob([bytes], { type: mime })
    return URL.createObjectURL(blob)
  }

  // 构建随机播放待播队列（还原 PCL2 ModMusic.vb 的 PreventFirst 防首尾连播算法）
  const rebuildShuffleQueue = (preventFirstPath?: string) => {
    const allPaths = musicTracks.value.map((t) => t.path)
    if (allPaths.length === 0) {
      shuffleQueue = []
      return
    }
    const copy = [...allPaths]
    for (let i = copy.length - 1; i > 0; i--) {
      const j = Math.floor(Math.random() * (i + 1))
      const tmp = copy[i]
      copy[i] = copy[j]
      copy[j] = tmp
    }
    if (preventFirstPath && copy.length > 1 && copy[0] === preventFirstPath) {
      copy.shift()
      copy.push(preventFirstPath)
    }
    shuffleQueue = copy
  }

  const pickNextTrack = (isAutoEnded = false): MusicTrack | null => {
    if (musicTracks.value.length === 0) return null
    if (isAutoEnded && musicPlayMode.value === 'single' && currentTrack.value) {
      return currentTrack.value
    }
    if (musicTracks.value.length === 1) {
      return musicTracks.value[0]
    }
    if (musicPlayMode.value === 'shuffle') {
      const validSet = new Set(musicTracks.value.map((t) => t.path))
      shuffleQueue = shuffleQueue.filter((p) => validSet.has(p))
      if (shuffleQueue.length === 0) {
        rebuildShuffleQueue(currentTrack.value?.path)
      }
      let nextPath = shuffleQueue.shift()
      if (nextPath && nextPath === currentTrack.value?.path && shuffleQueue.length > 0) {
        shuffleQueue.push(nextPath)
        nextPath = shuffleQueue.shift()
      }
      if (!shuffleQueue.length) {
        rebuildShuffleQueue(nextPath)
      }
      return musicTracks.value.find((t) => t.path === nextPath) || musicTracks.value[0]
    } else {
      const idx = currentTrack.value
        ? musicTracks.value.findIndex((t) => t.path === currentTrack.value?.path)
        : -1
      const nextIdx = (idx + 1) % musicTracks.value.length
      return musicTracks.value[nextIdx]
    }
  }

  const notificationMessage = ref('')
  let notificationTimer: any = null
  const showNotification = (msg: string) => {
    notificationMessage.value = msg
    if (notificationTimer) clearTimeout(notificationTimer)
    notificationTimer = setTimeout(() => {
      notificationMessage.value = ''
    }, 4500)
  }

  const instanceRefreshTrigger = ref(0)

  const validateTargetExists = async (targetPath?: string | null, targetLabel = '目标项'): Promise<boolean> => {
    if (!targetPath || !targetPath.trim()) {
      showNotification(`操作中止：${targetLabel}路径无效或未指定`)
      return false
    }
    try {
      const exists = await backend.checkPathExists(targetPath)
      if (!exists) {
        showNotification(`【${targetLabel}】在本地磁盘已不存在，已自动刷新同步状态`)
        await refreshInstalledVersions()
        if (musicDirs.value.length > 0) {
          await refreshMusicPlaylist(false, false)
        }
        instanceRefreshTrigger.value++
        return false
      }
      return true
    } catch {
      return true
    }
  }

  let lastFocusReconcile = 0
  const reconcileOnFocus = async () => {
    const now = Date.now()
    if (now - lastFocusReconcile < 2000) return
    lastFocusReconcile = now
    await refreshInstalledVersions()
    if (musicDirs.value.length > 0) {
      await refreshMusicPlaylist(false, false)
    }
    instanceRefreshTrigger.value++
  }

  const playTrack = async (track: MusicTrack, shouldPlay = true, recordHistory = true) => {
    if (!track) return

    if (!await validateTargetExists(track.path, `音乐曲目【${track.name}】`)) {
      musicTracks.value = musicTracks.value.filter((t) => t.path !== track.path)
      isMusicPlaying.value = false
      if (currentTrack.value?.path === track.path) {
        const next = pickNextTrack(false)
        if (next) {
          void playTrack(next, shouldPlay, false)
        } else {
          currentTrack.value = null
        }
      }
      return
    }

    musicError.value = ''
    if (recordHistory && currentTrack.value && currentTrack.value.path !== track.path) {
      historyStack.push(currentTrack.value.path)
      if (historyStack.length > 50) historyStack.shift()
    }

    if (currentBlobUrl) {
      URL.revokeObjectURL(currentBlobUrl)
      currentBlobUrl = null
    }

    currentTrack.value = track
    musicProgress.value = 0
    musicCurrentTime.value = 0
    musicDuration.value = 0

    try {
      const dataUrl = await backend.readMusicDataUrl(track.path)
      const blobUrl = dataUrlToBlobUrl(dataUrl)
      currentBlobUrl = blobUrl

      await audioEngine.loadSource(blobUrl, shouldPlay)

      if (shouldPlay) {
        isMusicPlaying.value = true
        consecutiveErrors = 0
      } else {
        isMusicPlaying.value = false
      }
    } catch (e: any) {
      console.warn('加载音乐文件失败:', e)
      musicError.value = `加载失败: ${track.name}`
      isMusicPlaying.value = false
    }
  }

  const toggleMusicPlay = async () => {
    if (musicTracks.value.length === 0) {
      await refreshMusicPlaylist(true)
      return
    }
    if (!currentTrack.value) {
      const first = pickNextTrack(false) || musicTracks.value[0]
      if (first) await playTrack(first, true)
      return
    }
    if (isMusicPlaying.value) {
      await audioEngine.pause()
      isMusicPlaying.value = false
    } else {
      try {
        await audioEngine.play()
        isMusicPlaying.value = true
      } catch (e) {
        console.warn('恢复播放失败:', e)
        isMusicPlaying.value = false
      }
    }
  }

  const playNextTrack = async () => {
    if (musicTracks.value.length === 0) return
    if (musicTracks.value.length === 1 && currentTrack.value) {
      audioEngine.seek(0)
      await audioEngine.play()
      isMusicPlaying.value = true
      return
    }
    const next = pickNextTrack(false)
    if (next) {
      await playTrack(next, true, true)
    }
  }

  const playPrevTrack = async () => {
    if (musicTracks.value.length === 0) return
    while (historyStack.length > 0) {
      const prevPath = historyStack.pop()!
      const found = musicTracks.value.find((t) => t.path === prevPath)
      if (found) {
        await playTrack(found, true, false)
        return
      }
    }
    const idx = currentTrack.value
      ? musicTracks.value.findIndex((t) => t.path === currentTrack.value?.path)
      : 0
    const prevIdx = (idx - 1 + musicTracks.value.length) % musicTracks.value.length
    await playTrack(musicTracks.value[prevIdx], true, false)
  }

  const seekMusic = (percent: number) => {
    const duration = audioEngine.getDuration()
    if (!duration || !Number.isFinite(duration)) return
    const clamped = Math.max(0, Math.min(100, percent))
    const targetSeconds = (clamped / 100) * duration
    audioEngine.seek(targetSeconds)
    musicProgress.value = clamped
    musicCurrentTime.value = targetSeconds
  }

  const setMusicVolume = (vol: number) => {
    const clamped = Math.max(0, Math.min(100, Math.round(vol)))
    musicVolume.value = clamped
    audioEngine.setVolume(clamped)
    saveCurrentConfig()
  }

  const setMusicEqPreset = (presetId: string) => {
    musicEqPreset.value = presetId
    audioEngine.setPreset(presetId, musicEqCustom.value)
    saveCurrentConfig()
  }

  const setMusicEqCustomBand = (bandIdx: number, gainDb: number) => {
    if (bandIdx >= 0 && bandIdx < 5) {
      const clamped = Math.max(-12, Math.min(12, Math.round(gainDb * 10) / 10))
      musicEqCustom.value[bandIdx] = clamped
      audioEngine.setCustomBandGain(bandIdx, clamped)
      saveCurrentConfig()
    }
  }

  const resetMusicEqCustom = () => {
    musicEqCustom.value = [0, 0, 0, 0, 0]
    audioEngine.setPreset('custom', [0, 0, 0, 0, 0])
    saveCurrentConfig()
  }

  const setMusicCompressor = (enabled: boolean) => {
    musicCompressor.value = enabled
    audioEngine.setCompressorEnabled(enabled)
    saveCurrentConfig()
  }

  const setMusicCrossfade = (enabled: boolean) => {
    musicCrossfade.value = enabled
    audioEngine.setCrossfadeEnabled(enabled)
    saveCurrentConfig()
  }

  const setMusicPlayMode = (mode: 'shuffle' | 'list' | 'single') => {
    musicPlayMode.value = mode
    if (mode === 'shuffle') {
      rebuildShuffleQueue(currentTrack.value?.path)
    }
    saveCurrentConfig()
  }

  const cycleMusicPlayMode = () => {
    const order: Array<'shuffle' | 'list' | 'single'> = ['shuffle', 'list', 'single']
    const next = order[(order.indexOf(musicPlayMode.value) + 1) % order.length]
    setMusicPlayMode(next)
  }

  const refreshMusicPlaylist = async (shouldPlayNow = false, isFirstLoad = false) => {
    const tracks = await backend.scanMusicFolders(musicDirs.value)
    musicTracks.value = tracks
    rebuildShuffleQueue(currentTrack.value?.path)

    if (tracks.length === 0) {
      await audioEngine.pause()
      isMusicPlaying.value = false
      currentTrack.value = null
      musicProgress.value = 0
      return
    }

    const stillExists =
      currentTrack.value && tracks.some((t) => t.path === currentTrack.value?.path)

    if (!stillExists || shouldPlayNow) {
      const next = pickNextTrack(false) || tracks[0]
      const autoStart = isFirstLoad ? musicAutoPlay.value : shouldPlayNow || isMusicPlaying.value
      await playTrack(next, autoStart, false)
    }
  }

  const addMusicFolder = async (folderPath: string) => {
    const trimmed = folderPath.trim()
    if (!trimmed) return
    if (!musicDirs.value.includes(trimmed)) {
      musicDirs.value.push(trimmed)
      await saveCurrentConfig()
    }
    await refreshMusicPlaylist(!isMusicPlaying.value)
  }

  const removeMusicFolder = async (folderPath: string) => {
    musicDirs.value = musicDirs.value.filter((d) => d !== folderPath)
    await saveCurrentConfig()
    await refreshMusicPlaylist(false)
  }

  // 游戏启动与退出时的音乐联动 (平滑淡出/淡入)
  const handleMusicOnGameLaunch = async () => {
    if (musicPauseOnLaunch.value && isMusicPlaying.value) {
      wasPlayingBeforeLaunch = true
      await audioEngine.pause()
      isMusicPlaying.value = false
    }
  }

  const handleMusicOnGameExit = async () => {
    if (musicPauseOnLaunch.value && wasPlayingBeforeLaunch) {
      wasPlayingBeforeLaunch = false
      try {
        await audioEngine.play()
        isMusicPlaying.value = true
      } catch {}
    }
  }

  const currentVersion = ref<VersionInfo | null>(null)

  const installedVersions = ref<VersionInfo[]>([])

  const userProfile = ref<UserProfile | null>(null)

  const playerName = ref('Steve')

  const runningPid = ref<number | null>(null)
  const isLaunching = ref(false)
  const launchProgress = ref<LaunchProgress | null>(null)

  const resetGameState = () => {
    runningPid.value = null
    isLaunching.value = false
    launchProgress.value = null
  }

  const downloadTask = ref<DownloadProgress | null>(null)
  const isDownloading = computed(() => !!downloadTask.value && !downloadTask.value.is_finished)

  let downloadFinishedTimer: any = null

  const updateDownloadProgress = (progress: DownloadProgress) => {
    downloadTask.value = progress

    if (progress.is_finished) {
      refreshInstalledVersions()
      if (downloadFinishedTimer) clearTimeout(downloadFinishedTimer)
      downloadFinishedTimer = setTimeout(() => {
        downloadTask.value = null
      }, 4000)
    }
  }

  const clearDownloadTask = () => {
    if (downloadFinishedTimer) clearTimeout(downloadFinishedTimer)
    downloadTask.value = null
  }

  const setThemeHue = (hue: number) => {
    themeHue.value = hue
    document.documentElement.style.setProperty('--pcl-hue', hue.toString())
    saveCurrentConfig()
  }

  const setUiStyle = (style: 'classic' | 'modern') => {
    uiStyle.value = style
    document.documentElement.setAttribute('data-ui-style', style)
    saveCurrentConfig()
  }

  let initPromise: Promise<void> | null = null

  // 初始化加载后端平台信息、持久化配置与凭据（带并发去重锁与首屏非阻塞异步扫描）
  const initConfig = async () => {
    if (initPromise) return initPromise
    initPromise = (async () => {
      try {
        const [pInfo, cfg, savedProfile] = await Promise.all([
          backend.getPlatformInfo(),
          backend.loadConfig(),
          backend.loadUserProfile(),
        ])
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

        audioEngine.setVolume(musicVolume.value)
        audioEngine.setPreset(musicEqPreset.value, musicEqCustom.value)
        audioEngine.setCompressorEnabled(musicCompressor.value)
        audioEngine.setCrossfadeEnabled(musicCrossfade.value)
        document.documentElement.style.setProperty('--pcl-hue', cfg.theme_hue.toString())
        document.documentElement.setAttribute('data-ui-style', uiStyle.value)

        // Profile 读自后端凭据文件
        if (savedProfile && !(hideYggdrasilLogin.value && savedProfile.auth_type === 'yggdrasil')) {
          userProfile.value = savedProfile
          playerName.value = savedProfile.name
        }

        // 后台异步扫描本地版本与音乐文件夹，不阻塞首帧显示与窗口弹出
        void refreshInstalledVersions()
        void refreshMusicPlaylist(false, true)
      } catch (e) {
        console.warn('加载配置文件失败，使用后端默认配置:', e)
      }
    })()
    return initPromise
  }

  const refreshInstalledVersions = async () => {
    if (!gameDir.value) return
    try {
      const list = await backend.getMinecraftVersions(gameDir.value)
      installedVersions.value = list
      if (currentVersion.value) {
        const stillExists = list.some((v) => v.id === currentVersion.value!.id)
        if (!stillExists) {
          const removedName = currentVersion.value.name || currentVersion.value.id
          if (list.length > 0) {
            currentVersion.value = list[0]
            showNotification(`已检测到版本【${removedName}】在磁盘被删除，已自动切换至【${list[0].name || list[0].id}】`)
          } else {
            currentVersion.value = null
            showNotification(`已检测到版本【${removedName}】在磁盘被删除，当前暂无可启动版本`)
          }
        } else {
          const updated = list.find((v) => v.id === currentVersion.value!.id)
          if (updated) currentVersion.value = updated
        }
      } else if (list.length > 0) {
        currentVersion.value = list[0]
      }
    } catch (e) {
      console.warn('获取已安装版本失败:', e)
    }
  }

  const setUserProfile = async (profile: UserProfile | null) => {
    userProfile.value = profile
    if (profile) {
      playerName.value = profile.name
    }
    try {
      await backend.saveUserProfile(profile)
    } catch (e) {
      console.error('保存用户凭据失败:', e)
    }
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
    currentTab,
    platformInfo,
    themeHue,
    gameDir,
    maxMemoryMb,
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
    musicTracks,
    currentTrack,
    isMusicPlaying,
    musicProgress,
    musicCurrentTime,
    musicDuration,
    musicError,
    currentVersion,
    installedVersions,
    userProfile,
    playerName,
    runningPid,
    isLaunching,
    launchProgress,
    resetGameState,
    downloadTask,
    isDownloading,
    updateDownloadProgress,
    clearDownloadTask,
    notificationMessage,
    showNotification,
    instanceRefreshTrigger,
    validateTargetExists,
    reconcileOnFocus,
    setThemeHue,
    setUiStyle,
    initConfig,
    refreshInstalledVersions,
    setUserProfile,
    saveCurrentConfig,
    playTrack,
    toggleMusicPlay,
    playNextTrack,
    playPrevTrack,
    seekMusic,
    setMusicVolume,
    setMusicPlayMode,
    cycleMusicPlayMode,
    setMusicEqPreset,
    setMusicEqCustomBand,
    resetMusicEqCustom,
    setMusicCompressor,
    setMusicCrossfade,
    getSpectrumData: (arr: Uint8Array) => audioEngine.getSpectrumData(arr),
    refreshMusicPlaylist,
    addMusicFolder,
    removeMusicFolder,
    handleMusicOnGameLaunch,
    handleMusicOnGameExit,
  }
})
