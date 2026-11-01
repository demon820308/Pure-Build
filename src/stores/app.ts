import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { backend } from '@/services/backend'
import type { VersionInfo, AppConfig, UserProfile, PlatformInfo, MusicTrack, LaunchProgress, DownloadProgress } from '@/services/backend'
import { audioEngine } from '@/services/audioEngine'

export const useAppStore = defineStore('app', () => {
  // 当前激活的页面 Tab ('launch' | 'download' | 'instance' | 'setup' | 'link')
  const currentTab = ref('launch')

  // 当前运行平台元数据（启动时从后端获取）
  const platformInfo = ref<PlatformInfo | null>(null)

  // 当前主题色色相 (0 ~ 360)
  const themeHue = ref(205)

  // Minecraft 游戏目录（由后端按平台推导或从用户配置读取，前端零平台字面量）
  const gameDir = ref('')

  // 内存分配 (MB)
  const maxMemoryMb = ref(4096)

  // 分辨率
  const windowWidth = ref(854)
  const windowHeight = ref(480)

  // 是否优先使用 BMCLAPI 镜像源
  const preferMirror = ref(true)

  // 个性化视觉与自定义主页配置
  const customBgUrl = ref('')
  const bgBlur = ref(8)
  const bgOpacity = ref(85)
  const customHomepageNotice = ref('')
  const uiStyle = ref<'classic' | 'modern'>('classic')

  // 是否隐藏外置登录入口（默认隐藏 = true）
  const hideYggdrasilLogin = ref(true)

  // ==================== 背景音乐 (BGM) 持久化配置与播放状态 ====================
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

  // 绑定 Web Audio DSP 调音引擎事件
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

  // 从待播队列取出下一首曲目
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

  // 加载并播放指定曲目 (经 Web Audio DSP 调音引擎)
  const playTrack = async (track: MusicTrack, shouldPlay = true, recordHistory = true) => {
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

  // 播放 / 暂停切换 (带平滑淡入淡出防爆音)
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

  // 下一首（右键快捷切歌或点击下一曲按钮）
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

  // 上一首
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

  // 拖拽进度条跳转 (0 ~ 100)
  const seekMusic = (percent: number) => {
    const duration = audioEngine.getDuration()
    if (!duration || !Number.isFinite(duration)) return
    const clamped = Math.max(0, Math.min(100, percent))
    const targetSeconds = (clamped / 100) * duration
    audioEngine.seek(targetSeconds)
    musicProgress.value = clamped
    musicCurrentTime.value = targetSeconds
  }

  // 设置音量 (0 ~ 100) 映射对数感知曲线
  const setMusicVolume = (vol: number) => {
    const clamped = Math.max(0, Math.min(100, Math.round(vol)))
    musicVolume.value = clamped
    audioEngine.setVolume(clamped)
    saveCurrentConfig()
  }

  // 切换均衡器调音预设 ('flat' | 'bass_boost' | 'ambient' | 'vocal' | 'warm' | 'custom')
  const setMusicEqPreset = (presetId: string) => {
    musicEqPreset.value = presetId
    audioEngine.setPreset(presetId, musicEqCustom.value)
    saveCurrentConfig()
  }

  // 调整自定义频段增益 (-12 ~ +12 dB)
  const setMusicEqCustomBand = (bandIdx: number, gainDb: number) => {
    if (bandIdx >= 0 && bandIdx < 5) {
      const clamped = Math.max(-12, Math.min(12, Math.round(gainDb * 10) / 10))
      musicEqCustom.value[bandIdx] = clamped
      audioEngine.setCustomBandGain(bandIdx, clamped)
      saveCurrentConfig()
    }
  }

  // 重置自定义均衡器为平直 (0dB)
  const resetMusicEqCustom = () => {
    musicEqCustom.value = [0, 0, 0, 0, 0]
    audioEngine.setPreset('custom', [0, 0, 0, 0, 0])
    saveCurrentConfig()
  }

  // 设置动态防破音压缩器开关
  const setMusicCompressor = (enabled: boolean) => {
    musicCompressor.value = enabled
    audioEngine.setCompressorEnabled(enabled)
    saveCurrentConfig()
  }

  // 设置平滑声学过渡淡入淡出开关
  const setMusicCrossfade = (enabled: boolean) => {
    musicCrossfade.value = enabled
    audioEngine.setCrossfadeEnabled(enabled)
    saveCurrentConfig()
  }

  // 切换循环模式 ('shuffle' -> 'list' -> 'single')
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

  // 扫描并刷新音乐播放列表
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

  // 添加自定义音乐文件夹（自动去重并扫描子文件夹）
  const addMusicFolder = async (folderPath: string) => {
    const trimmed = folderPath.trim()
    if (!trimmed) return
    if (!musicDirs.value.includes(trimmed)) {
      musicDirs.value.push(trimmed)
      await saveCurrentConfig()
    }
    await refreshMusicPlaylist(!isMusicPlaying.value)
  }

  // 移除自定义音乐文件夹
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

  // 当前选中的真实版本
  const currentVersion = ref<VersionInfo | null>(null)

  // 本地真实已安装版本列表
  const installedVersions = ref<VersionInfo[]>([])

  // 当前登录的用户身份 (正版档案或离线档案，持久化于后端 app_config_dir)
  const userProfile = ref<UserProfile | null>(null)

  // 离线/自定义玩家昵称
  const playerName = ref('Steve')

  // ==================== 游戏运行与启动生命周期（全局共享状态，切换 Tab 不丢失） ====================
  const runningPid = ref<number | null>(null)
  const isLaunching = ref(false)
  const launchProgress = ref<LaunchProgress | null>(null)

  const resetGameState = () => {
    runningPid.value = null
    isLaunching.value = false
    launchProgress.value = null
  }

  // ==================== 全局下载与安装任务生命周期（原版 PCL2 体验，切换 Tab 不丢失） ====================
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

  // 设置主题色相并保存
  const setThemeHue = (hue: number) => {
    themeHue.value = hue
    document.documentElement.style.setProperty('--pcl-hue', hue.toString())
    saveCurrentConfig()
  }

  // 切换 UI 界面风格与布局 ('classic' = 原版 PCL2 经典布局, 'modern' = 现代侧栏布局)
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

        // 从后端安全凭据文件读取已登录的 Profile（不再使用前端 localStorage）
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

  // 刷新本地已安装版本列表
  const refreshInstalledVersions = async () => {
    if (!gameDir.value) return
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

  // 保存当前用户 Profile 到后端凭据存储
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

  // 触发保存全局配置
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
