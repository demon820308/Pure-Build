import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { MusicTrack } from '../services/backend'
import { backend } from '../services/backend'
import { audioEngine } from '../services/audioEngine'
import { useConfigStore } from './config'
import { filterExistingTracks } from '../utils/coreLogic'

export const useMusicStore = defineStore('music', () => {
  const config = useConfigStore()

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
      void playTrack(next, true, true)
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
        setTimeout(() => void playTrack(next, true, false), 600)
      }
    }
  }

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
    if (isAutoEnded && config.musicPlayMode === 'single' && currentTrack.value) {
      return currentTrack.value
    }
    if (musicTracks.value.length === 1) {
      return musicTracks.value[0]
    }
    if (config.musicPlayMode === 'shuffle') {
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

  const playTrack = async (
    track: MusicTrack,
    shouldPlay = true,
    recordHistory = true,
    onPathCheckFailed?: (path: string, name: string) => void,
  ) => {
    if (!track) return

    try {
      const exists = await backend.checkPathExists(track.path)
      if (!exists) {
        musicTracks.value = filterExistingTracks(musicTracks.value, track.path)
        isMusicPlaying.value = false
        if (onPathCheckFailed) onPathCheckFailed(track.path, track.name)
        if (currentTrack.value?.path === track.path) {
          const next = pickNextTrack(false)
          if (next) {
            void playTrack(next, shouldPlay, false, onPathCheckFailed)
          } else {
            currentTrack.value = null
          }
        }
        return
      }
    } catch {
      // 容错放行
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
    config.musicVolume = clamped
    audioEngine.setVolume(clamped)
    void config.saveCurrentConfig()
  }

  const setMusicEqPreset = (presetId: string) => {
    config.musicEqPreset = presetId
    audioEngine.setPreset(presetId, config.musicEqCustom)
    void config.saveCurrentConfig()
  }

  const setMusicEqCustomBand = (bandIdx: number, gainDb: number) => {
    if (bandIdx >= 0 && bandIdx < 5) {
      const clamped = Math.max(-12, Math.min(12, Math.round(gainDb * 10) / 10))
      config.musicEqCustom[bandIdx] = clamped
      audioEngine.setCustomBandGain(bandIdx, clamped)
      void config.saveCurrentConfig()
    }
  }

  const resetMusicEqCustom = () => {
    config.musicEqCustom = [0, 0, 0, 0, 0]
    audioEngine.setPreset('custom', [0, 0, 0, 0, 0])
    void config.saveCurrentConfig()
  }

  const setMusicCompressor = (enabled: boolean) => {
    config.musicCompressor = enabled
    audioEngine.setCompressorEnabled(enabled)
    void config.saveCurrentConfig()
  }

  const setMusicCrossfade = (enabled: boolean) => {
    config.musicCrossfade = enabled
    audioEngine.setCrossfadeEnabled(enabled)
    void config.saveCurrentConfig()
  }

  const setMusicPlayMode = (mode: 'shuffle' | 'list' | 'single') => {
    config.musicPlayMode = mode
    if (mode === 'shuffle') {
      rebuildShuffleQueue(currentTrack.value?.path)
    }
    void config.saveCurrentConfig()
  }

  const cycleMusicPlayMode = () => {
    const order: Array<'shuffle' | 'list' | 'single'> = ['shuffle', 'list', 'single']
    const next = order[(order.indexOf(config.musicPlayMode) + 1) % order.length]
    setMusicPlayMode(next)
  }

  const refreshMusicPlaylist = async (shouldPlayNow = false, isFirstLoad = false) => {
    const tracks = await backend.scanMusicFolders(config.musicDirs)
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
      const autoStart = isFirstLoad ? config.musicAutoPlay : shouldPlayNow || isMusicPlaying.value
      await playTrack(next, autoStart, false)
    }
  }

  const addMusicFolder = async (folderPath: string) => {
    const trimmed = folderPath.trim()
    if (!trimmed) return
    if (!config.musicDirs.includes(trimmed)) {
      config.musicDirs.push(trimmed)
      await config.saveCurrentConfig()
    }
    await refreshMusicPlaylist(!isMusicPlaying.value)
  }

  const removeMusicFolder = async (folderPath: string) => {
    config.musicDirs = config.musicDirs.filter((d) => d !== folderPath)
    await config.saveCurrentConfig()
    await refreshMusicPlaylist(false)
  }

  const handleMusicOnGameLaunch = async () => {
    if (config.musicPauseOnLaunch && isMusicPlaying.value) {
      wasPlayingBeforeLaunch = true
      await audioEngine.pause()
      isMusicPlaying.value = false
    }
  }

  const handleMusicOnGameExit = async () => {
    if (config.musicPauseOnLaunch && wasPlayingBeforeLaunch) {
      wasPlayingBeforeLaunch = false
      try {
        await audioEngine.play()
        isMusicPlaying.value = true
      } catch {}
    }
  }

  return {
    musicTracks,
    currentTrack,
    isMusicPlaying,
    musicProgress,
    musicCurrentTime,
    musicDuration,
    musicError,
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
