<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import { backend } from '@/services/backend'
import { useConfigStore } from '@/stores/config'
import { useMusicStore } from '@/stores/music'
import { useSessionStore } from '@/stores/session'
import { EQ_PRESETS } from '@/services/audioEngine'

const config = useConfigStore()
const music = useMusicStore()
const session = useSessionStore()

const showMusicPopover = ref(false)
const musicSearchQuery = ref('')

watch(() => session.currentTab, () => {
  showMusicPopover.value = false
})

const filteredMusicTracks = computed(() => {
  const q = musicSearchQuery.value.trim().toLowerCase()
  if (!q) return music.musicTracks
  return music.musicTracks.filter(
    (t) => t.name.toLowerCase().includes(q) || t.folder_path.toLowerCase().includes(q)
  )
})

const playModeMeta = computed(() => {
  switch (config.musicPlayMode) {
    case 'shuffle':
      return { icon: '🔀', label: '随机循环' }
    case 'list':
      return { icon: '🔁', label: '列表循环' }
    case 'single':
      return { icon: '🔂', label: '单曲循环' }
  }
})

const currentEqMeta = computed(() => {
  return EQ_PRESETS.find((p) => p.id === config.musicEqPreset) || EQ_PRESETS[0]
})

const cycleEqPreset = () => {
  const order = ['flat', 'bass_boost', 'ambient', 'vocal', 'warm']
  const idx = order.indexOf(config.musicEqPreset)
  const next = order[(idx + 1) % order.length]
  music.setMusicEqPreset(next)
}

const formatDuration = (sec: number): string => {
  if (!sec || !Number.isFinite(sec) || sec < 0) return '00:00'
  const total = Math.floor(sec)
  const m = Math.floor(total / 60)
  const s = total % 60
  return `${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`
}

const folderBaseName = (folderPath: string): string => {
  const cleaned = folderPath.replace(/[\\/]+$/, '')
  const parts = cleaned.split(/[\\/]/)
  return parts[parts.length - 1] || folderPath
}

const handleAddMusicFolder = async () => {
  try {
    const selected = await open({
      directory: true,
      multiple: true,
      title: '选择要添加的背景音乐文件夹（自动扫描子文件夹）',
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
  } catch (e) {
    console.warn('选择音乐文件夹失败:', e)
  }
}

const handleOpenDefaultMusicDir = async () => {
  try {
    const pInfo = config.platformInfo || (await backend.getPlatformInfo())
    await backend.openFolder(pInfo.default_music_dir)
  } catch (e) {
    console.warn('打开默认音乐文件夹失败:', e)
  }
}

const handleMusicBallRightClick = async () => {
  if (music.musicTracks.length > 0) {
    await music.playNextTrack()
  } else {
    showMusicPopover.value = true
  }
}

defineExpose({
  showMusicPopover,
  handleMusicBallRightClick,
})
</script>

<template>
  <div>
    <!-- Modern 风格侧栏底部音乐按钮插槽外部调用，也可在此提供全局弹出面板 -->
    <div
      v-if="showMusicPopover"
      class="fixed inset-0 z-30"
      @click="showMusicPopover = false"
    />

    <Transition name="fade-slide">
      <div
        v-if="showMusicPopover"
        class="fixed z-40 text-xs flex flex-col gap-3 transition-colors"
        :class="
          config.uiStyle === 'classic'
            ? 'right-5 bottom-[74px] w-[330px] rounded-[6px] bg-white/98 border border-[var(--pcl-brush-5)] shadow-[0_6px_22px_rgba(19,112,243,0.18)] p-3.5 text-[#343d4a]'
            : 'left-[80px] bottom-3.5 w-[340px] rounded-2xl bg-pcl-card border border-pcl-border shadow-pcl-window p-4 text-pcl-text backdrop-blur-xl'
        "
      >
        <div class="flex items-start justify-between gap-2">
          <div class="min-w-0 flex-1">
            <div class="flex items-center gap-1.5">
              <span class="text-sm">{{ music.isMusicPlaying ? '🎵' : '⏸️' }}</span>
              <span
                class="font-bold truncate text-[13px]"
                :class="config.uiStyle === 'classic' ? 'text-[var(--pcl-brush-1)]' : 'text-pcl-text'"
                :title="music.currentTrack?.name"
              >
                {{ music.currentTrack?.name || '暂无播放中的音乐' }}
              </span>
            </div>
            <div
              class="text-[11px] mt-0.5 flex items-center gap-2"
              :class="config.uiStyle === 'classic' ? 'text-[#737373]' : 'text-pcl-subtext'"
            >
              <span>{{ formatDuration(music.musicCurrentTime) }} / {{ formatDuration(music.musicDuration) }}</span>
              <span>·</span>
              <span>共 {{ music.musicTracks.length }} 首</span>
            </div>
          </div>
          <button
            class="w-6 h-6 flex items-center justify-center cursor-pointer transition-colors"
            :class="
              config.uiStyle === 'classic'
                ? 'rounded-[4px] text-[#737373] hover:bg-[var(--pcl-brush-7)] hover:text-[var(--pcl-brush-2)]'
                : 'rounded-lg text-pcl-subtext hover:bg-white/10 hover:text-pcl-text'
            "
            title="收起播放面板"
            @click="showMusicPopover = false"
          >
            ✕
          </button>
        </div>

        <div class="flex items-center gap-2">
          <input
            type="range"
            min="0"
            max="100"
            step="0.5"
            :value="music.musicProgress"
            :disabled="!music.currentTrack"
            class="w-full accent-[var(--pcl-color-primary)] cursor-pointer h-1.5 rounded-lg"
            :class="config.uiStyle === 'classic' ? 'bg-[var(--pcl-brush-6)]' : 'bg-black/20'"
            @input="(e) => music.seekMusic(Number((e.target as HTMLInputElement).value))"
          />
        </div>

        <div class="flex items-center justify-between gap-1.5">
          <div class="flex items-center gap-1 shrink-0">
            <button
              class="px-2 py-1 text-[11px] flex items-center gap-1 cursor-pointer transition-all"
              :class="
                config.uiStyle === 'classic'
                  ? 'rounded-[4px] border border-black/20 bg-white text-[#343d4a] hover:border-[var(--pcl-brush-3)] hover:text-[var(--pcl-brush-2)] hover:bg-[var(--pcl-brush-7)]'
                  : 'rounded-xl border border-pcl-border bg-[var(--pcl-color-bg)] text-pcl-text hover:border-pcl-primary/60 hover:text-pcl-primary'
              "
              :title="`循环模式：${playModeMeta.label}（点击切换）`"
              @click="music.cycleMusicPlayMode()"
            >
              <span>{{ playModeMeta.icon }}</span>
            </button>

            <button
              class="px-2 py-1 text-[11px] flex items-center cursor-pointer transition-all"
              :class="
                config.uiStyle === 'classic'
                  ? 'rounded-[4px] border border-black/20 bg-white text-[#343d4a] hover:border-[var(--pcl-brush-3)] hover:text-[var(--pcl-brush-2)] hover:bg-[var(--pcl-brush-7)]'
                  : 'rounded-xl border border-pcl-border bg-[var(--pcl-color-bg)] text-pcl-text hover:border-pcl-primary/60 hover:text-pcl-primary'
              "
              :title="`调音音效：${currentEqMeta.name}（${currentEqMeta.desc}，点击快速切换预设）`"
              @click="cycleEqPreset()"
            >
              <span class="text-[10px] font-medium">{{ currentEqMeta.name }}</span>
            </button>
          </div>

          <div class="flex items-center gap-1">
            <button
              class="w-7 h-7 flex items-center justify-center cursor-pointer transition-colors"
              :class="
                config.uiStyle === 'classic'
                  ? 'rounded-[4px] text-[#343d4a] hover:bg-[var(--pcl-brush-7)] hover:text-[var(--pcl-brush-2)]'
                  : 'rounded-lg text-pcl-text hover:bg-white/10 hover:text-pcl-primary'
              "
              title="上一曲"
              @click="music.playPrevTrack()"
            >
              ⏮
            </button>
            <button
              class="w-8 h-8 flex items-center justify-center cursor-pointer transition-all active:scale-95"
              :class="
                config.uiStyle === 'classic'
                  ? 'rounded-full border border-[var(--pcl-brush-3)] bg-[var(--pcl-brush-3)] hover:bg-[var(--pcl-brush-2)] text-white shadow-sm'
                  : 'rounded-xl bg-pcl-primary hover:bg-pcl-hover text-white shadow-md shadow-pcl-primary/25'
              "
              :title="music.isMusicPlaying ? '暂停' : '播放'"
              @click="music.toggleMusicPlay()"
            >
              {{ music.isMusicPlaying ? '⏸' : '▶' }}
            </button>
            <button
              class="w-7 h-7 flex items-center justify-center cursor-pointer transition-colors"
              :class="
                config.uiStyle === 'classic'
                  ? 'rounded-[4px] text-[#343d4a] hover:bg-[var(--pcl-brush-7)] hover:text-[var(--pcl-brush-2)]'
                  : 'rounded-lg text-pcl-text hover:bg-white/10 hover:text-pcl-primary'
              "
              title="下一曲（也可直接右键音乐按钮）"
              @click="music.playNextTrack()"
            >
              ⏭
            </button>
          </div>

          <div class="flex items-center gap-1 flex-1 min-w-0 max-w-[120px]" :title="`音量: ${config.musicVolume}%`">
            <span class="text-[11px]">{{ config.musicVolume === 0 ? '🔇' : '🔊' }}</span>
            <input
              type="range"
              min="0"
              max="100"
              :value="config.musicVolume"
              class="w-full min-w-0 accent-[var(--pcl-color-primary)] cursor-pointer h-1"
              @input="(e) => music.setMusicVolume(Number((e.target as HTMLInputElement).value))"
            />
          </div>
        </div>

        <div
          class="flex items-center justify-between gap-1.5 pt-2 border-t"
          :class="config.uiStyle === 'classic' ? 'border-black/8' : 'border-pcl-border'"
        >
          <button
            class="flex-1 py-1.5 px-2.5 font-semibold flex items-center justify-center gap-1 cursor-pointer transition-all"
            :class="
              config.uiStyle === 'classic'
                ? 'rounded-[4px] border border-[var(--pcl-brush-3)] bg-[var(--pcl-brush-8)] text-[var(--pcl-brush-2)] hover:bg-[var(--pcl-brush-6)]'
                : 'rounded-xl bg-pcl-primary hover:bg-pcl-hover text-white shadow-sm'
            "
            title="添加本地音乐文件夹（自动递归扫描子文件夹）"
            @click="handleAddMusicFolder"
          >
            <span>➕</span>
            <span>添加文件夹</span>
          </button>
          <button
            class="py-1.5 px-2.5 flex items-center gap-1 cursor-pointer transition-all"
            :class="
              config.uiStyle === 'classic'
                ? 'rounded-[4px] border border-black/20 bg-white text-[#343d4a] hover:border-[var(--pcl-brush-3)] hover:text-[var(--pcl-brush-2)] hover:bg-[var(--pcl-brush-7)]'
                : 'rounded-xl border border-pcl-border bg-[var(--pcl-color-bg)] text-pcl-text hover:border-pcl-primary/50 hover:text-pcl-primary'
            "
            title="打开默认 Musics 音乐文件夹"
            @click="handleOpenDefaultMusicDir"
          >
            <span>📂</span>
            <span>默认库</span>
          </button>
          <button
            class="py-1.5 px-2.5 flex items-center gap-1 cursor-pointer transition-all"
            :class="
              config.uiStyle === 'classic'
                ? 'rounded-[4px] border border-black/20 bg-white text-[#343d4a] hover:border-[var(--pcl-brush-3)] hover:text-[var(--pcl-brush-2)] hover:bg-[var(--pcl-brush-7)]'
                : 'rounded-xl border border-pcl-border bg-[var(--pcl-color-bg)] text-pcl-text hover:border-pcl-primary/50 hover:text-pcl-primary'
            "
            title="重新扫描所有音乐文件夹"
            @click="music.refreshMusicPlaylist(false)"
          >
            <span>🔄</span>
          </button>
        </div>

        <div v-if="config.musicDirs.length > 0" class="flex flex-wrap gap-1.5 max-h-[54px] overflow-y-auto">
          <div
            v-for="dir in config.musicDirs"
            :key="dir"
            class="inline-flex items-center gap-1 px-2 py-0.5 text-[10.5px] max-w-full transition-colors"
            :class="
              config.uiStyle === 'classic'
                ? 'rounded-[3px] bg-[var(--pcl-brush-8)] border border-[var(--pcl-brush-5)] text-[var(--pcl-brush-2)]'
                : 'rounded-lg bg-pcl-primary/15 border border-pcl-primary/30 text-pcl-primary'
            "
            :title="dir"
          >
            <span
              class="truncate max-w-[210px] cursor-pointer hover:underline"
              @click="backend.openFolder(dir)"
            >
              📁 {{ folderBaseName(dir) }}
            </span>
            <button
              class="hover:text-red-500 cursor-pointer ml-0.5 font-bold"
              title="移除此音乐文件夹"
              @click="music.removeMusicFolder(dir)"
            >
              ×
            </button>
          </div>
        </div>

        <div v-if="music.musicTracks.length > 5">
          <input
            v-model="musicSearchQuery"
            placeholder="搜索歌名或所在目录..."
            class="w-full px-2.5 py-1.5 text-[11px] outline-none transition-colors"
            :class="
              config.uiStyle === 'classic'
                ? 'rounded-[4px] bg-white border border-black/20 text-[#343d4a] placeholder:text-[#8c8c8c] focus:border-[var(--pcl-brush-3)]'
                : 'rounded-xl bg-[var(--pcl-color-bg)] border border-pcl-border text-pcl-text placeholder:text-pcl-subtext focus:border-pcl-primary'
            "
          />
        </div>

        <div class="max-h-[180px] overflow-y-auto space-y-1 pr-0.5">
          <div
            v-if="filteredMusicTracks.length === 0"
            class="py-6 text-center text-[11px]"
            :class="config.uiStyle === 'classic' ? 'text-[#737373]' : 'text-pcl-subtext'"
          >
            {{
              music.musicTracks.length === 0
                ? '暂无音乐，点击上方「➕ 添加文件夹」导入音乐库'
                : '未找到匹配的曲目'
            }}
          </div>
          <div
            v-for="track in filteredMusicTracks"
            :key="track.path"
            class="px-2.5 py-1.5 flex items-center justify-between gap-2 cursor-pointer transition-colors"
            :class="
              config.uiStyle === 'classic'
                ? music.currentTrack?.path === track.path
                  ? 'rounded-[4px] bg-[var(--pcl-brush-6)] text-[var(--pcl-brush-2)] font-semibold'
                  : 'rounded-[4px] text-[#343d4a] hover:bg-[var(--pcl-brush-7)]'
                : music.currentTrack?.path === track.path
                  ? 'rounded-xl bg-pcl-primary/15 border border-pcl-primary/30 text-pcl-primary font-semibold'
                  : 'rounded-xl border border-transparent text-pcl-text hover:bg-white/5'
            "
            :title="track.path"
            @click="music.playTrack(track, true)"
          >
            <div class="flex items-center gap-1.5 min-w-0 flex-1">
              <span class="text-[11px] shrink-0">
                {{
                  music.currentTrack?.path === track.path
                    ? music.isMusicPlaying
                      ? '🔊'
                      : '⏸'
                    : '♪'
                }}
              </span>
              <span class="truncate text-[11.5px]">{{ track.name }}</span>
            </div>
            <span
              class="text-[10px] shrink-0"
              :class="config.uiStyle === 'classic' ? 'text-[#737373]' : 'text-pcl-subtext'"
            >
              {{ track.size_mb.toFixed(1) }} MB
            </span>
          </div>
        </div>
      </div>
    </Transition>

    <!-- Classic 风格右下角悬浮球 -->
    <div
      v-if="config.uiStyle === 'classic'"
      class="fixed z-30 right-5 bottom-5 pointer-events-auto"
    >
      <button
        class="relative transition-all flex items-center justify-center cursor-pointer active:scale-95 w-11 h-11 rounded-full bg-[var(--pcl-brush-3)] text-white shadow-lg hover:bg-[var(--pcl-brush-2)] hover:scale-105"
        :title="
          music.currentTrack
            ? `${music.isMusicPlaying ? '正在播放' : '已暂停'}：${music.currentTrack.name}\n左键打开迷你播放列表，右键切下一首`
            : '左键打开音乐播放列表 / 添加文件夹，右键切下一首'
        "
        @click="showMusicPopover = !showMusicPopover"
        @contextmenu.prevent="handleMusicBallRightClick"
      >
        <svg
          class="absolute inset-0 w-full h-full -rotate-90 pointer-events-none"
          viewBox="0 0 44 44"
        >
          <circle
            cx="22"
            cy="22"
            r="19.5"
            fill="none"
            stroke="rgba(255,255,255,0.22)"
            stroke-width="2.2"
          />
          <circle
            cx="22"
            cy="22"
            r="19.5"
            fill="none"
            stroke="#ffffff"
            stroke-width="2.4"
            stroke-linecap="round"
            :stroke-dasharray="122.5"
            :stroke-dashoffset="122.5 - (122.5 * music.musicProgress) / 100"
          />
        </svg>

        <span class="text-sm relative z-10" :class="music.isMusicPlaying ? 'animate-pulse' : ''">
          {{ music.isMusicPlaying ? '🎵' : '🎶' }}
        </span>
      </button>
    </div>
  </div>
</template>
