/**
 * Web Audio DSP 专业调音引擎
 * 提供多频段参数均衡器 (EQ)、动态防破音压缩器 (Limiter)、对数感知音量曲线、
 * 丝滑防爆音平滑过渡 (Smooth Fade In/Out) 及实时频谱分析。
 * 遵循项目平台中立规范：100% 原生标准 Web Audio API，兼容 Windows (WebView2)、macOS (WebKit)、Linux。
 */

export interface EqBandConfig {
  freq: number
  type: BiquadFilterType
  q?: number
  label: string
}

export const EQ_BANDS: EqBandConfig[] = [
  { freq: 80, type: 'lowshelf', label: '80Hz (低音)' },
  { freq: 250, type: 'peaking', q: 1.0, label: '250Hz (中低音)' },
  { freq: 1000, type: 'peaking', q: 1.0, label: '1kHz (人声/中音)' },
  { freq: 4000, type: 'peaking', q: 1.0, label: '4kHz (通透/中高)' },
  { freq: 12000, type: 'highshelf', label: '12kHz (空气感/高音)' },
]

export interface EqPreset {
  id: string
  name: string
  desc: string
  icon: string
  // [80Hz, 250Hz, 1kHz, 4kHz, 12kHz] 单位 dB (-12 ~ +12)
  gains: [number, number, number, number, number]
}

export const EQ_PRESETS: EqPreset[] = [
  {
    id: 'flat',
    name: '原声监听',
    desc: '原汁原味，真实还原母带细节',
    icon: '🎧',
    gains: [0, 0, 0, 0, 0],
  },
  {
    id: 'bass_boost',
    name: '重低音加强',
    desc: '强化低频下潜与节奏感，震撼澎湃',
    icon: '🔊',
    gains: [6.0, 3.0, -1.0, 1.5, 2.0],
  },
  {
    id: 'ambient',
    name: 'MC 氛围沉浸',
    desc: '空灵纯净、声场开阔，专为 C418/Lena 原声优化',
    icon: '🌲',
    gains: [3.5, 1.5, -2.0, 2.5, 4.5],
  },
  {
    id: 'vocal',
    name: '清澈人声',
    desc: '强化人声主体与歌词咬字，清脆明亮',
    icon: '🎤',
    gains: [-2.0, -1.0, 4.0, 2.5, -0.5],
  },
  {
    id: 'warm',
    name: '温润柔和',
    desc: '削减刺耳高频，柔美耐听，常驻后台防疲劳',
    icon: '☕',
    gains: [2.5, 2.0, 0.5, -2.0, -4.0],
  },
  {
    id: 'custom',
    name: '自定义',
    desc: '手动微调 5 频段增益',
    icon: '🎛️',
    gains: [0, 0, 0, 0, 0],
  },
]

export class AudioEngine {
  private audioCtx: AudioContext | null = null
  private audioElement: HTMLAudioElement | null = null
  private sourceNode: MediaElementAudioSourceNode | null = null
  private eqFilters: BiquadFilterNode[] = []
  private compressorNode: DynamicsCompressorNode | null = null
  private masterGainNode: GainNode | null = null
  private analyserNode: AnalyserNode | null = null

  // 状态变量
  private currentVolume = 45 // 0 ~ 100
  private currentPresetId = 'flat'
  private customGains: number[] = [0, 0, 0, 0, 0]
  private compressorEnabled = true
  private crossfadeEnabled = true
  private isFading = false

  // 事件回调
  public onTimeUpdate?: (currentTime: number, duration: number, progress: number) => void
  public onLoadedMetadata?: (duration: number) => void
  public onEnded?: () => void
  public onError?: (error: string) => void

  constructor() {
    // 延迟到首次用户交互时初始化 AudioContext
  }

  /**
   * 初始化 Web Audio 拓扑管线
   */
  private ensureAudioGraph() {
    if (this.audioCtx && this.audioElement) return

    const AudioContextClass = window.AudioContext || (window as any).webkitAudioContext
    if (!AudioContextClass) {
      console.warn('当前运行环境不支持 Web Audio API')
      return
    }

    this.audioCtx = new AudioContextClass()
    this.audioElement = new Audio()
    this.audioElement.crossOrigin = 'anonymous'

    // 绑定原生 Audio 事件
    this.audioElement.ontimeupdate = () => {
      if (!this.audioElement) return
      const current = this.audioElement.currentTime || 0
      const duration = Number.isFinite(this.audioElement.duration) ? this.audioElement.duration : 0
      const progress = duration > 0 ? Math.min(100, (current / duration) * 100) : 0
      this.onTimeUpdate?.(current, duration, progress)
    }

    this.audioElement.onloadedmetadata = () => {
      if (!this.audioElement) return
      const duration = Number.isFinite(this.audioElement.duration) ? this.audioElement.duration : 0
      this.onLoadedMetadata?.(duration)
    }

    this.audioElement.onended = () => {
      this.onEnded?.()
    }

    this.audioElement.onerror = () => {
      this.onError?.('音频播放遇到内部解码错误')
    }

    // 搭建 Web Audio 节点链路
    this.sourceNode = this.audioCtx.createMediaElementSource(this.audioElement)

    // 1. 创建 5 频段 BiquadFilterNode
    this.eqFilters = EQ_BANDS.map((cfg) => {
      const filter = this.audioCtx!.createBiquadFilter()
      filter.type = cfg.type
      filter.frequency.setValueAtTime(cfg.freq, this.audioCtx!.currentTime)
      if (cfg.q !== undefined) {
        filter.Q.setValueAtTime(cfg.q, this.audioCtx!.currentTime)
      }
      filter.gain.setValueAtTime(0, this.audioCtx!.currentTime)
      return filter
    })

    // 2. 创建动态防破音压缩器 (DynamicsCompressorNode)
    this.compressorNode = this.audioCtx.createDynamicsCompressor()
    this.applyCompressorConfig()

    // 3. 创建 Master GainNode (主音量 + 平滑淡入淡出)
    this.masterGainNode = this.audioCtx.createGain()
    const targetGain = this.computePerceptualGain(this.currentVolume)
    this.masterGainNode.gain.setValueAtTime(targetGain, this.audioCtx.currentTime)

    // 4. 创建 AnalyserNode (用于实时频谱可视化)
    this.analyserNode = this.audioCtx.createAnalyser()
    this.analyserNode.fftSize = 64
    this.analyserNode.smoothingTimeConstant = 0.8

    // 串联管线: Source -> Filter0 -> Filter1 -> ... -> Compressor -> MasterGain -> Analyser -> Destination
    let lastNode: AudioNode = this.sourceNode
    for (const filter of this.eqFilters) {
      lastNode.connect(filter)
      lastNode = filter
    }

    lastNode.connect(this.compressorNode)
    this.compressorNode.connect(this.masterGainNode)

    this.masterGainNode.connect(this.analyserNode)
    this.analyserNode.connect(this.audioCtx.destination)

    // 应用初始预设
    this.applyEqGains(this.getEffectiveGains())
  }

  /**
   * 人耳生理声学对数感知音量曲线转换 (二次幂感知曲线)
   */
  private computePerceptualGain(volume0to100: number): number {
    const clamped = Math.max(0, Math.min(100, volume0to100))
    if (clamped === 0) return 0
    const normalized = clamped / 100
    // 二次幂曲线在低音量处提供更细腻的微调范围
    return Math.pow(normalized, 2)
  }

  /**
   * 恢复挂起的 AudioContext（遵守现代浏览器用户手势激活策略）
   */
  private async resumeContextIfNeeded() {
    if (this.audioCtx && this.audioCtx.state === 'suspended') {
      try {
        await this.audioCtx.resume()
      } catch (e) {
        console.warn('恢复 AudioContext 失败:', e)
      }
    }
  }

  /**
   * 应用动态压缩器参数
   */
  private applyCompressorConfig() {
    if (!this.compressorNode || !this.audioCtx) return
    const now = this.audioCtx.currentTime
    if (this.compressorEnabled) {
      this.compressorNode.threshold.setTargetAtTime(-14, now, 0.05)
      this.compressorNode.knee.setTargetAtTime(12, now, 0.05)
      this.compressorNode.ratio.setTargetAtTime(6, now, 0.05)
      this.compressorNode.attack.setTargetAtTime(0.003, now, 0.05)
      this.compressorNode.release.setTargetAtTime(0.25, now, 0.05)
    } else {
      // 旁路模式 (平直直通，不压缩)
      this.compressorNode.threshold.setTargetAtTime(0, now, 0.05)
      this.compressorNode.ratio.setTargetAtTime(1, now, 0.05)
    }
  }

  /**
   * 获取当前生效的 5 频段增益数组
   */
  private getEffectiveGains(): [number, number, number, number, number] {
    if (this.currentPresetId === 'custom') {
      return [
        this.customGains[0] ?? 0,
        this.customGains[1] ?? 0,
        this.customGains[2] ?? 0,
        this.customGains[3] ?? 0,
        this.customGains[4] ?? 0,
      ]
    }
    const preset = EQ_PRESETS.find((p) => p.id === this.currentPresetId) || EQ_PRESETS[0]
    return preset.gains
  }

  /**
   * 平滑应用 5 频段增益（避免产生切频突变瞬态）
   */
  private applyEqGains(gains: [number, number, number, number, number]) {
    if (!this.audioCtx || this.eqFilters.length === 0) return
    const now = this.audioCtx.currentTime
    for (let i = 0; i < 5; i++) {
      const filter = this.eqFilters[i]
      if (filter) {
        const clampedDb = Math.max(-12, Math.min(12, gains[i] ?? 0))
        filter.gain.setTargetAtTime(clampedDb, now, 0.04)
      }
    }
  }

  // ==================== 公共业务控制接口 ====================

  /**
   * 加载音频源并播放
   */
  public async loadSource(blobUrl: string, autoPlay: boolean): Promise<void> {
    this.ensureAudioGraph()
    await this.resumeContextIfNeeded()

    if (!this.audioElement || !this.audioCtx || !this.masterGainNode) return

    // 如果当前正在播放且启用了淡出，先快速衰减再切源
    if (!this.audioElement.paused && this.crossfadeEnabled) {
      const now = this.audioCtx.currentTime
      this.masterGainNode.gain.cancelScheduledValues(now)
      this.masterGainNode.gain.setValueAtTime(this.masterGainNode.gain.value, now)
      this.masterGainNode.gain.linearRampToValueAtTime(0.0001, now + 0.12)
      await new Promise((resolve) => setTimeout(resolve, 120))
    }

    this.audioElement.pause()
    this.audioElement.src = blobUrl
    this.audioElement.load()

    const targetGain = this.computePerceptualGain(this.currentVolume)

    if (autoPlay) {
      if (this.crossfadeEnabled) {
        const now = this.audioCtx.currentTime
        this.masterGainNode.gain.cancelScheduledValues(now)
        this.masterGainNode.gain.setValueAtTime(0.0001, now)
        await this.audioElement.play()
        this.masterGainNode.gain.linearRampToValueAtTime(targetGain, now + 0.2)
      } else {
        this.masterGainNode.gain.cancelScheduledValues(this.audioCtx.currentTime)
        this.masterGainNode.gain.setValueAtTime(targetGain, this.audioCtx.currentTime)
        await this.audioElement.play()
      }
    }
  }

  /**
   * 恢复播放 (带丝滑淡入防爆音)
   */
  public async play(): Promise<void> {
    this.ensureAudioGraph()
    await this.resumeContextIfNeeded()
    if (!this.audioElement || !this.audioCtx || !this.masterGainNode) return

    const targetGain = this.computePerceptualGain(this.currentVolume)

    if (this.crossfadeEnabled) {
      const now = this.audioCtx.currentTime
      this.masterGainNode.gain.cancelScheduledValues(now)
      this.masterGainNode.gain.setValueAtTime(0.0001, now)
      await this.audioElement.play()
      this.masterGainNode.gain.linearRampToValueAtTime(targetGain, now + 0.18)
    } else {
      this.masterGainNode.gain.cancelScheduledValues(this.audioCtx.currentTime)
      this.masterGainNode.gain.setValueAtTime(targetGain, this.audioCtx.currentTime)
      await this.audioElement.play()
    }
  }

  /**
   * 暂停播放 (带丝滑淡出防爆音)
   */
  public async pause(): Promise<void> {
    if (!this.audioElement || !this.audioCtx || !this.masterGainNode) return
    if (this.audioElement.paused) return

    if (this.crossfadeEnabled && !this.isFading) {
      this.isFading = true
      const now = this.audioCtx.currentTime
      const currentGain = this.masterGainNode.gain.value
      this.masterGainNode.gain.cancelScheduledValues(now)
      this.masterGainNode.gain.setValueAtTime(currentGain, now)
      this.masterGainNode.gain.linearRampToValueAtTime(0.0001, now + 0.15)

      await new Promise((resolve) => setTimeout(resolve, 150))
      this.audioElement.pause()
      this.isFading = false
    } else {
      this.audioElement.pause()
    }
  }

  /**
   * 跳转播放进度
   */
  public seek(timeInSeconds: number) {
    if (!this.audioElement || !Number.isFinite(timeInSeconds)) return
    this.audioElement.currentTime = Math.max(0, timeInSeconds)
  }

  /**
   * 设置主音量 (0 ~ 100)
   */
  public setVolume(volume0to100: number) {
    this.currentVolume = Math.max(0, Math.min(100, Math.round(volume0to100)))
    const targetGain = this.computePerceptualGain(this.currentVolume)
    if (this.masterGainNode && this.audioCtx && !this.isFading) {
      const now = this.audioCtx.currentTime
      this.masterGainNode.gain.cancelScheduledValues(now)
      this.masterGainNode.gain.setTargetAtTime(targetGain, now, 0.03)
    }
  }

  /**
   * 切换调音预设 ('flat' | 'bass_boost' | 'ambient' | 'vocal' | 'warm' | 'custom')
   */
  public setPreset(presetId: string, customGains?: number[]) {
    this.currentPresetId = presetId
    if (customGains && customGains.length === 5) {
      this.customGains = [...customGains]
    }
    this.applyEqGains(this.getEffectiveGains())
  }

  /**
   * 设置自定义频段增益
   */
  public setCustomBandGain(bandIndex: number, gainDb: number) {
    if (bandIndex < 0 || bandIndex >= 5) return
    this.customGains[bandIndex] = Math.max(-12, Math.min(12, gainDb))
    if (this.currentPresetId === 'custom') {
      this.applyEqGains(this.getEffectiveGains())
    }
  }

  /**
   * 设置动态防破音压缩器开关
   */
  public setCompressorEnabled(enabled: boolean) {
    this.compressorEnabled = enabled
    this.applyCompressorConfig()
  }

  /**
   * 设置平滑声学过渡淡入淡出开关
   */
  public setCrossfadeEnabled(enabled: boolean) {
    this.crossfadeEnabled = enabled
  }

  /**
   * 获取当前播放状态
   */
  public isPlaying(): boolean {
    return !!this.audioElement && !this.audioElement.paused
  }

  public getCurrentTime(): number {
    return this.audioElement?.currentTime || 0
  }

  public getDuration(): number {
    return Number.isFinite(this.audioElement?.duration) ? this.audioElement!.duration : 0
  }

  /**
   * 获取实时频域数据 (32 频点)，用于 UI 频谱与波形动效渲染
   */
  public getSpectrumData(outputArray: Uint8Array): void {
    if (!this.analyserNode) {
      outputArray.fill(0)
      return
    }
    this.analyserNode.getByteFrequencyData(outputArray as any)
  }

  /**
   * 销毁释放资源
   */
  public dispose() {
    if (this.audioElement) {
      this.audioElement.pause()
      this.audioElement.src = ''
      this.audioElement = null
    }
    if (this.audioCtx) {
      this.audioCtx.close().catch(() => {})
      this.audioCtx = null
    }
  }
}

// 全局单例导出
export const audioEngine = new AudioEngine()
