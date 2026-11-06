<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import MyCard from '@/components/base/MyCard.vue'
import MyButton from '@/components/base/MyButton.vue'
import PclIcon from '@/components/base/PclIcon.vue'
import { backend } from '@/services/backend'
import { open } from '@tauri-apps/plugin-dialog'
import { EQ_PRESETS, EQ_BANDS } from '@/services/audioEngine'
import { useConfigStore } from '@/stores/config'
import { useMusicStore } from '@/stores/music'

defineProps<{
  visible?: boolean
}>()

const config = useConfigStore()
const music = useMusicStore()

// 实时 7 频段动能频谱指示器
const spectrumBars = ref<number[]>([12, 12, 12, 12, 12, 12, 12])
let animFrameId: number | null = null
const spectrumRaw = new Uint8Array(32)

const updateSpectrumLoop = () => {
  if (music.isMusicPlaying) {
    music.getSpectrumData(spectrumRaw)
    const indices = [1, 3, 6, 10, 15, 22, 28]
    spectrumBars.value = indices.map((idx) => {
      const val = spectrumRaw[idx] || 0
      return Math.max(12, Math.round((val / 255) * 100))
    })
  } else {
    spectrumBars.value = [12, 12, 12, 12, 12, 12, 12]
  }
  animFrameId = requestAnimationFrame(updateSpectrumLoop)
}

onMounted(() => {
  animFrameId = requestAnimationFrame(updateSpectrumLoop)
})

onUnmounted(() => {
  if (animFrameId !== null) {
    cancelAnimationFrame(animFrameId)
  }
})

const handleAddMusicFolder = async () => {
  try {
    const selected = await open({
      directory: true,
      multiple: true,
      title: '选择要添加的背景音乐文件夹（自动递归扫描子文件夹）',
    })
    if (!selected) return
    if (Array.isArray(selected)) {
      for (const dir of selected) {
        if (typeof dir === 'string') {
          await music.addMusicFolder(dir)
        }
      }
    } else if (typeof selected === 'string') {
      await music.addMusicFolder(selected)
    }
  } catch (err) {
    console.warn('选择音乐文件夹失败:', err)
  }
}

const handleOpenDefaultMusicDir = async () => {
  const dir = config.platformInfo?.default_music_dir
  if (dir) {
    await backend.openFolder(dir)
  }
}
</script>

<template>
  <MyCard
    v-show="visible !== false"
    title="背景音乐 (BGM)"
    subtitle="支持添加多个音乐文件夹（自动扫描子文件夹）与多种循环播放模式"
  >
    <div class="space-y-4 text-xs">
      <div class="flex items-center justify-between gap-3 p-3 rounded-xl bg-black/5 dark:bg-white/5 border border-pcl-border">
        <div class="min-w-0 flex-1">
          <div class="font-semibold text-pcl-text flex items-center gap-2 truncate">
            <span>{{ music.isMusicPlaying ? '正在播放：' : '当前曲目：' }}</span>
            <span class="text-pcl-primary truncate">{{ music.currentTrack?.name || '暂无播放曲目' }}</span>
          </div>
          <div class="text-[11px] text-pcl-subtext mt-0.5">
            已从默认音乐库及 {{ config.musicDirs.length }} 个自定义文件夹中扫描到 {{ music.musicTracks.length }} 首音频曲目
          </div>
        </div>
        <div class="flex items-center gap-1.5 shrink-0">
          <MyButton type="secondary" size="small" @click="music.playPrevTrack()">
            上一首
          </MyButton>
          <MyButton type="primary" size="small" @click="music.toggleMusicPlay()">
            {{ music.isMusicPlaying ? '暂停' : '播放' }}
          </MyButton>
          <MyButton type="secondary" size="small" @click="music.playNextTrack()">
            下一首
          </MyButton>
        </div>
      </div>

      <div class="grid grid-cols-2 gap-4">
        <div class="space-y-1.5">
          <div class="flex justify-between text-[11px] text-pcl-subtext">
            <span>音乐播放音量</span>
            <span class="font-mono font-medium text-pcl-primary">{{ config.musicVolume }}%</span>
          </div>
          <input
            :value="config.musicVolume"
            type="range"
            min="0"
            max="100"
            class="w-full accent-[var(--pcl-color-primary)] cursor-pointer h-1.5"
            @input="(e) => music.setMusicVolume(Number((e.target as HTMLInputElement).value))"
          />
        </div>

        <div class="space-y-1.5">
          <span class="text-[11px] text-pcl-subtext block">循环播放模式</span>
          <div class="grid grid-cols-3 gap-1.5">
            <button
              v-for="mode in [
                { id: 'shuffle', label: '随机循环' },
                { id: 'list', label: '列表循环' },
                { id: 'single', label: '单曲循环' },
              ] as const"
              :key="mode.id"
              class="py-1 px-2 rounded-md border text-[11px] font-medium cursor-pointer transition-colors"
              :class="
                config.musicPlayMode === mode.id
                  ? 'border-[var(--pcl-brush-3)] bg-[var(--pcl-brush-3)] text-white'
                  : 'border-pcl-border bg-black/5 dark:bg-white/5 text-pcl-text hover:border-pcl-primary/50'
              "
              @click="music.setMusicPlayMode(mode.id)"
            >
              {{ mode.label }}
            </button>
          </div>
        </div>
      </div>

      <div class="space-y-3 pt-3 border-t border-pcl-border">
        <div class="flex items-center justify-between">
          <div>
            <div class="font-bold text-pcl-text flex items-center gap-1.5">
              <span>音频调音与均衡器 (DSP)</span>
              <span class="text-[10px] px-1.5 py-0.5 rounded bg-pcl-primary/10 text-pcl-primary font-mono font-medium">Web Audio API</span>
            </div>
            <div class="text-[11px] text-pcl-subtext mt-0.5">
              基于硬件级数字信号处理，提供 5 频段多阶滤波、动态防破音与声学过渡
            </div>
          </div>

          <div class="flex items-end gap-1 h-5 px-2 py-1 rounded bg-black/5 dark:bg-white/5 border border-pcl-border" title="实时频段动能分析">
            <span
              v-for="(height, i) in spectrumBars"
              :key="i"
              class="w-1 rounded-full transition-all duration-75"
              :class="music.isMusicPlaying ? 'bg-[var(--pcl-color-primary)]' : 'bg-pcl-subtext/40'"
              :style="{ height: `${height}%` }"
            />
          </div>
        </div>

        <div class="grid grid-cols-3 gap-2">
          <button
            v-for="preset in EQ_PRESETS"
            :key="preset.id"
            class="p-2.5 rounded-xl border text-left cursor-pointer transition-all flex flex-col justify-between gap-1"
            :class="
              config.musicEqPreset === preset.id
                ? 'border-pcl-primary bg-pcl-primary/10 shadow-sm'
                : 'border-pcl-border bg-black/5 dark:bg-white/5 hover:border-pcl-primary/40'
            "
            @click="music.setMusicEqPreset(preset.id)"
          >
            <div class="flex items-center justify-between">
              <span class="font-bold text-pcl-text text-xs">{{ preset.name }}</span>
              <span
                v-if="config.musicEqPreset === preset.id"
                class="text-[10px] text-pcl-primary font-bold"
              >✓</span>
            </div>
            <div class="text-[10px] text-pcl-subtext line-clamp-1">
              {{ preset.desc }}
            </div>
          </button>
        </div>

        <div
          v-if="config.musicEqPreset === 'custom'"
          class="p-3 rounded-xl bg-black/5 dark:bg-white/5 border border-pcl-border space-y-2.5"
        >
          <div class="flex items-center justify-between text-[11px]">
            <span class="font-bold text-pcl-text">5 频段精确增益 (-12dB ~ +12dB)</span>
            <button
              class="text-pcl-primary hover:underline cursor-pointer text-[11px]"
              @click="music.resetMusicEqCustom"
            >
              重置为 0dB
            </button>
          </div>
          <div class="grid grid-cols-5 gap-2">
            <div
              v-for="(band, idx) in EQ_BANDS"
              :key="band.freq"
              class="space-y-1 text-center"
            >
              <div class="text-[10px] text-pcl-subtext truncate" :title="band.label">{{ band.label }}</div>
              <input
                type="range"
                min="-12"
                max="12"
                step="0.5"
                :value="config.musicEqCustom[idx]"
                class="w-full accent-[var(--pcl-color-primary)] cursor-pointer h-1.5"
                @input="(e) => music.setMusicEqCustomBand(idx, Number((e.target as HTMLInputElement).value))"
              />
              <div
                class="text-[10px] font-mono font-medium"
                :class="
                  config.musicEqCustom[idx] > 0
                    ? 'text-pcl-primary'
                    : config.musicEqCustom[idx] < 0
                      ? 'text-orange-500'
                      : 'text-pcl-subtext'
                "
              >
                {{ config.musicEqCustom[idx] > 0 ? `+${config.musicEqCustom[idx]}` : config.musicEqCustom[idx] }}dB
              </div>
            </div>
          </div>
        </div>

        <div class="flex flex-wrap items-center gap-6 pt-1">
          <label class="flex items-center gap-2 text-pcl-text cursor-pointer select-none" title="通过动态范围压缩器自动抑制瞬态过载，杜绝破音">
            <input
              v-model="config.musicCompressor"
              type="checkbox"
              class="accent-[var(--pcl-color-primary)] cursor-pointer"
              @change="() => music.setMusicCompressor(config.musicCompressor)"
            />
            <span class="font-medium">智能防破音 (动态压缩 Limiter)</span>
          </label>

          <label class="flex items-center gap-2 text-pcl-text cursor-pointer select-none" title="在切歌、播放/暂停时进行指数平滑淡入淡出，消除切歌爆音">
            <input
              v-model="config.musicCrossfade"
              type="checkbox"
              class="accent-[var(--pcl-color-primary)] cursor-pointer"
              @change="() => music.setMusicCrossfade(config.musicCrossfade)"
            />
            <span class="font-medium">丝滑平滑过渡 (防爆音淡入淡出)</span>
          </label>
        </div>
      </div>

      <div class="flex flex-wrap items-center gap-6 pt-1">
        <label class="flex items-center gap-2 text-pcl-text cursor-pointer select-none">
          <input
            v-model="config.musicAutoPlay"
            type="checkbox"
            class="accent-[var(--pcl-color-primary)] cursor-pointer"
            @change="config.saveCurrentConfig"
          />
          <span>打开启动器时自动开始播放</span>
        </label>
        <label class="flex items-center gap-2 text-pcl-text cursor-pointer select-none">
          <input
            v-model="config.musicPauseOnLaunch"
            type="checkbox"
            class="accent-[var(--pcl-color-primary)] cursor-pointer"
            @change="config.saveCurrentConfig"
          />
          <span>游戏启动后自动暂停播放，游戏退出后自动恢复播放</span>
        </label>
      </div>

      <div class="space-y-2 pt-1 border-t border-pcl-border">
        <div class="flex items-center justify-between">
          <span class="text-pcl-subtext font-medium">
            已挂载的音乐文件夹（支持递归扫描所有子文件夹内的 .mp3 / .flac / .wav / .ogg / .m4a）
          </span>
          <div class="flex items-center gap-2">
            <MyButton type="primary" size="small" @click="handleAddMusicFolder">
              添加文件夹
            </MyButton>
            <MyButton type="secondary" size="small" @click="handleOpenDefaultMusicDir">
              <PclIcon name="folder" :size="12" class="mr-1" />
              默认音乐库
            </MyButton>
            <MyButton type="secondary" size="small" @click="music.refreshMusicPlaylist(true)">
              <PclIcon name="refresh" :size="12" class="mr-1" />
              刷新
            </MyButton>
          </div>
        </div>

        <div class="space-y-1.5">
          <div
            v-if="config.platformInfo?.default_music_dir"
            class="flex items-center justify-between px-3 py-2 rounded-lg bg-black/5 dark:bg-white/5 border border-pcl-border text-[11px]"
          >
            <div class="flex items-center gap-2 min-w-0 flex-1">
              <span class="px-1.5 py-0.2 rounded bg-pcl-primary/15 text-pcl-primary font-semibold shrink-0">默认库</span>
              <span class="font-mono text-pcl-subtext truncate" :title="config.platformInfo.default_music_dir">
                {{ config.platformInfo.default_music_dir }}
              </span>
            </div>
            <button
              class="text-pcl-primary hover:underline ml-2 shrink-0 cursor-pointer"
              @click="handleOpenDefaultMusicDir"
            >
              打开目录
            </button>
          </div>

          <div
            v-for="dir in config.musicDirs"
            :key="dir"
            class="flex items-center justify-between px-3 py-2 rounded-lg bg-black/5 dark:bg-white/5 border border-pcl-border text-[11px]"
          >
            <div class="flex items-center gap-2 min-w-0 flex-1">
              <span class="px-1.5 py-0.2 rounded bg-emerald-500/15 text-emerald-600 dark:text-emerald-400 font-semibold shrink-0">
                自定义
              </span>
              <span class="font-mono text-pcl-text truncate" :title="dir">{{ dir }}</span>
            </div>
            <div class="flex items-center gap-3 ml-2 shrink-0">
              <button
                class="text-pcl-primary hover:underline cursor-pointer"
                @click="backend.openFolder(dir)"
              >
                打开目录
              </button>
              <button
                class="text-red-500 hover:underline cursor-pointer"
                @click="music.removeMusicFolder(dir)"
              >
                移除
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  </MyCard>
</template>
