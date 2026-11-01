<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import LaunchView from '@/views/LaunchView.vue'
import InstanceView from '@/views/InstanceView.vue'
import DownloadView from '@/views/DownloadView.vue'
import SetupView from '@/views/SetupView.vue'
import LinkView from '@/views/LinkView.vue'
import OtherView from '@/views/OtherView.vue'
import CrashDialog from '@/components/CrashDialog.vue'
import PclIcon from '@/components/base/PclIcon.vue'
import { backend } from '@/services/backend'
import type { CrashAnalysisResult } from '@/services/backend'
import { useAppStore } from '@/stores/app'
import { EQ_PRESETS } from '@/services/audioEngine'

const store = useAppStore()
const crashReport = ref<CrashAnalysisResult | null>(null)
let unlistenCrash: (() => void) | null = null
let unlistenLaunchBgm: (() => void) | null = null
let unlistenExitBgm: (() => void) | null = null
let unlistenDownloadTask: (() => void) | null = null

// 迷你播放列表浮层开关与搜索过滤（现代模式位于左下角侧栏，经典模式位于右下角）
const showMusicPopover = ref(false)
const musicSearchQuery = ref('')

// 切换导航 Tab 时自动收起播放列表浮层
watch(() => store.currentTab, () => {
  showMusicPopover.value = false
})

const filteredMusicTracks = computed(() => {
  const q = musicSearchQuery.value.trim().toLowerCase()
  if (!q) return store.musicTracks
  return store.musicTracks.filter(
    (t) => t.name.toLowerCase().includes(q) || t.folder_path.toLowerCase().includes(q)
  )
})

const playModeMeta = computed(() => {
  switch (store.musicPlayMode) {
    case 'shuffle':
      return { icon: '🔀', label: '随机循环' }
    case 'list':
      return { icon: '🔁', label: '列表循环' }
    case 'single':
      return { icon: '🔂', label: '单曲循环' }
  }
})

const currentEqMeta = computed(() => {
  return EQ_PRESETS.find((p) => p.id === store.musicEqPreset) || EQ_PRESETS[0]
})

const cycleEqPreset = () => {
  const order = ['flat', 'bass_boost', 'ambient', 'vocal', 'warm']
  const idx = order.indexOf(store.musicEqPreset)
  const next = order[(idx + 1) % order.length]
  store.setMusicEqPreset(next)
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

// 唤起系统文件夹选择器添加音乐文件夹（自动递归扫描子目录）
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
          await store.addMusicFolder(dir)
        }
      }
    } else if (typeof selected === 'string') {
      await store.addMusicFolder(selected)
    }
  } catch (e) {
    console.warn('选择音乐文件夹失败:', e)
  }
}

// 右键悬浮球：直接切下一首；若列表为空则打开浮层引导添加文件夹
const handleMusicBallRightClick = async () => {
  if (store.musicTracks.length === 0) {
    showMusicPopover.value = true
    return
  }
  await store.playNextTrack()
}

const handleOpenDefaultMusicDir = async () => {
  const dir = store.platformInfo?.default_music_dir
  if (dir) {
    await backend.openFolder(dir)
  }
}

onMounted(async () => {
  await store.initConfig()
  unlistenCrash = await backend.onGameCrash((report) => {
    crashReport.value = report
    store.resetGameState()
    store.handleMusicOnGameExit()
  })
  // 游戏进程拉起时同步全局运行状态并自动暂停背景音乐，退出时自动恢复（还原 PCL2 经典行为）
  unlistenLaunchBgm = await backend.onLaunchProgress((p) => {
    store.launchProgress = p
    if (p.pid) {
      store.runningPid = p.pid
      store.handleMusicOnGameLaunch()
    }
    if (p.is_finished) {
      store.isLaunching = false
      if (p.error) {
        store.runningPid = null
      }
    }
  })
  unlistenExitBgm = await backend.onGameExitNormal(() => {
    store.resetGameState()
    store.handleMusicOnGameExit()
  })
  unlistenDownloadTask = await backend.onDownloadProgress((p) => {
    store.updateDownloadProgress(p)
  })
})

onUnmounted(() => {
  if (unlistenCrash) unlistenCrash()
  if (unlistenLaunchBgm) unlistenLaunchBgm()
  if (unlistenExitBgm) unlistenExitBgm()
  if (unlistenDownloadTask) unlistenDownloadTask()
})

// 现代模式左侧导航项
const modernNavItems = [
  { id: 'launch', name: '启动', icon: '🚀' },
  { id: 'download', name: '下载', icon: '📥' },
  { id: 'instance', name: '版本', icon: '📦' },
  { id: 'setup', name: '设置', icon: '⚙️' },
  { id: 'link', name: '联机', icon: '🌐' },
  { id: 'other', name: '百宝箱', icon: '🧰' },
]

// 原版 PCL2 FormMain.xaml 顶栏 5 大主导航项 (Tag 0~4)
const classicNavItems: Array<{
  id: string
  name: string
  icon: 'launch' | 'download' | 'link' | 'setup' | 'more'
}> = [
  { id: 'launch', name: '启动', icon: 'launch' },
  { id: 'download', name: '下载', icon: 'download' },
  { id: 'link', name: '联机', icon: 'link' },
  { id: 'setup', name: '设置', icon: 'setup' },
  { id: 'other', name: '更多', icon: 'more' },
]
</script>

<template>
  <div
    class="relative h-screen w-screen flex flex-col overflow-hidden select-none"
    :class="
      store.uiStyle === 'classic'
        ? 'bg-gradient-to-b from-[var(--pcl-brush-6)] via-[var(--pcl-brush-7)] to-[var(--pcl-brush-8)]'
        : 'bg-pcl-bg'
    "
  >
    <!-- 自定义背景壁纸图层（当用户在设置中配置了壁纸时生效） -->
    <div
      v-if="store.customBgUrl"
      class="absolute inset-0 pointer-events-none z-0 bg-cover bg-center transition-all duration-300"
      :style="{
        backgroundImage: `url(${store.customBgUrl})`,
        filter: `blur(${store.bgBlur}px)`,
        opacity: (100 - store.bgOpacity) / 100,
      }"
    />

    <!-- ==================== 1A. 原版 PCL2 经典顶栏 (FormMain.xaml PanTitle 52px) ==================== -->
    <header
      v-if="store.uiStyle === 'classic'"
      data-tauri-drag-region
      class="relative z-10 h-[52px] flex items-center justify-between px-5 bg-white dark:bg-[#1e232b] border-b border-black/8 dark:border-white/10 cursor-default shrink-0"
      @dblclick.prevent.stop
    >
      <!-- 左侧：原版 ShapeTitleLogo 矢量线条 + PCL 字样；若在二级版本设置页则显示 PanTitleInner 返回栏 -->
      <div data-tauri-drag-region class="flex items-center gap-2.5">
        <template v-if="store.currentTab === 'instance'">
          <button
            class="w-7 h-7 rounded-full flex items-center justify-center text-[#1D1D1F] dark:text-gray-100 hover:bg-black/5 dark:hover:bg-white/10 transition-colors cursor-pointer"
            title="返回启动页"
            @click="store.currentTab = 'launch'"
          >
            <PclIcon name="back" :size="14" />
          </button>
          <span class="text-[15px] font-medium text-[#1D1D1F] dark:text-gray-100 pointer-events-none">
            版本选择与管理 ({{ store.currentVersion?.name || '全部实例' }})
          </span>
        </template>
        <template v-else>
          <img
            src="/favicon.png"
            alt="PBL"
            class="w-6 h-6 object-contain pointer-events-none select-none drop-shadow-sm"
          />
          <svg
            width="44"
            height="15"
            viewBox="24 0 64 31"
            fill="none"
            stroke="currentColor"
            stroke-width="2.2"
            stroke-linecap="round"
            stroke-linejoin="round"
            class="text-[#1D1D1F] dark:text-gray-100 pointer-events-none"
          >
            <!-- 矢量绘制 P B L 三字母 -->
            <path
              d="M26,29 v-25 h6 a7,7 0 0 1 0,14 h-6 M47,29 v-25 h6.5 a6,6 0 0 1 0,12 h-6.5 h7 a6.5,6.5 0 0 1 0,13 h-7 M71,4 v25 h13.5"
            />
          </svg>
          <span class="text-[14px] font-semibold text-[#1D1D1F] dark:text-gray-100 tracking-wide pointer-events-none">
            Pure Build Launcher
          </span>
        </template>
      </div>

      <!-- 右上角：复刻 FormMain.xaml PanTopRightHub 胶囊导航托盘 + 窗口控制 -->
      <div
        class="flex items-center bg-[#F2F2F7] dark:bg-white/10 border border-[#E5E5EA] dark:border-white/15 rounded-[18px] p-[3px] gap-0.5 z-10"
      >
        <button
          v-for="item in classicNavItems"
          :key="item.id"
          class="h-[28px] px-3 rounded-[14px] flex items-center gap-1.5 text-[12.5px] font-medium transition-all duration-150 cursor-pointer"
          :class="[
            store.currentTab === item.id
              ? 'bg-[var(--pcl-brush-3)] text-white shadow-sm'
              : 'text-[#1D1D1F] dark:text-gray-200 hover:bg-black/5 dark:hover:bg-white/10'
          ]"
          @click="store.currentTab = item.id"
        >
          <PclIcon :name="item.icon" :size="13" />
          <span>{{ item.name }}</span>
        </button>

        <!-- 一键切换经典/现代布局快捷按钮 -->
        <div class="w-[1px] h-4 bg-[#D1D1D6] dark:bg-white/20 mx-1" />
        <button
          class="h-[26px] px-2.5 rounded-[13px] text-[11px] font-medium text-[var(--pcl-brush-2)] hover:bg-[var(--pcl-brush-6)] transition-colors cursor-pointer"
          title="切换为现代侧栏布局 (可在设置中随时切回)"
          @click="store.setUiStyle('modern')"
        >
          ✨ 现代布局
        </button>

        <!-- Windows/Linux 最小化与关闭按钮（macOS 使用原生红绿灯，自动隐藏） -->
        <template v-if="!store.platformInfo?.is_macos">
          <div class="w-[1px] h-4 bg-[#D1D1D6] dark:bg-white/20 mx-1" />
          <button
            class="w-6 h-6 rounded-full flex items-center justify-center text-[#1D1D1F] dark:text-gray-200 hover:bg-black/10 transition-colors cursor-pointer"
            title="最小化"
            @click="backend.minimizeWindow"
          >
            <PclIcon name="min" :size="10" />
          </button>
          <button
            class="w-6 h-6 rounded-full flex items-center justify-center text-[#1D1D1F] dark:text-gray-200 hover:bg-[#ff4c4c] hover:text-white transition-colors cursor-pointer mr-0.5"
            title="关闭"
            @click="backend.closeWindow"
          >
            <PclIcon name="close" :size="10" />
          </button>
        </template>
      </div>
    </header>

    <!-- ==================== 1B. 现代 PBL 顶栏 (40px) ==================== -->
    <header
      v-else
      data-tauri-drag-region
      class="relative z-10 h-10 flex items-center justify-between px-4 bg-pcl-card border-b border-pcl-border cursor-default shrink-0"
      @dblclick.prevent.stop
    >
      <div data-tauri-drag-region class="flex items-center gap-2 text-xs font-semibold text-pcl-text pointer-events-none">
        <img
          src="/favicon.png"
          alt="PBL"
          class="w-5 h-5 object-contain select-none drop-shadow-sm"
        />
        <span>Pure Build Launcher (PBL)</span>
      </div>

      <div class="flex items-center gap-1.5 z-10">
        <button
          class="px-2.5 h-7 flex items-center gap-1 rounded-lg border border-pcl-primary/30 bg-pcl-primary/15 text-pcl-primary hover:bg-pcl-primary/25 text-[11px] font-medium transition-colors cursor-pointer"
          title="一键切换为 1:1 经典布局"
          @click="store.setUiStyle('classic')"
        >
          🏛️ 切换经典布局
        </button>

        <template v-if="!store.platformInfo?.is_macos">
          <button
            class="w-7 h-7 flex items-center justify-center rounded-lg hover:bg-black/5 dark:hover:bg-white/10 text-xs transition-colors cursor-pointer text-pcl-text"
            @click="backend.minimizeWindow"
          >
            ─
          </button>
          <button
            class="w-7 h-7 flex items-center justify-center rounded-lg hover:bg-red-500 hover:text-white text-xs transition-colors cursor-pointer text-pcl-text"
            @click="backend.closeWindow"
          >
            ✕
          </button>
        </template>
      </div>
    </header>

    <!-- ==================== 2. 主体区域 ==================== -->
    <div class="relative z-10 flex-1 flex overflow-hidden">
      <!-- 现代模式下显示最左侧图标导航栏；经典模式下隐藏（由顶栏胶囊 + 各页面内 PanMainLeft 承载） -->
      <aside
        v-if="store.uiStyle === 'modern'"
        class="w-18 flex flex-col items-center py-4 gap-3 bg-pcl-card/30 border-r border-pcl-border shrink-0 select-none"
      >
        <button
          v-for="item in modernNavItems"
          :key="item.id"
          class="w-12 h-12 rounded-2xl flex flex-col items-center justify-center gap-1 text-xs font-medium cursor-pointer transition-all duration-300 ease-[var(--pcl-ease-bounce)]"
          :class="[
            store.currentTab === item.id
              ? 'bg-pcl-primary text-white shadow-md shadow-pcl-primary/30 scale-105'
              : 'text-pcl-subtext hover:bg-black/5 dark:hover:bg-white/5 hover:text-pcl-text'
          ]"
          @click="store.currentTab = item.id"
        >
          <span class="text-base">{{ item.icon }}</span>
          <span class="text-[10px] scale-90">{{ item.name }}</span>
        </button>

        <!-- 底部常驻：音乐播放器按钮（现代布局左下角） -->
        <button
          class="mt-auto w-12 h-12 rounded-2xl flex flex-col items-center justify-center relative cursor-pointer transition-all duration-300 ease-[var(--pcl-ease-bounce)] active:scale-95 group"
          :class="[
            showMusicPopover
              ? 'bg-pcl-primary text-white shadow-md shadow-pcl-primary/30 scale-105'
              : 'bg-pcl-card border border-pcl-border text-pcl-primary shadow-pcl-card hover:border-pcl-primary/60 hover:bg-pcl-primary/10 hover:scale-105'
          ]"
          :title="
            store.currentTrack
              ? `${store.isMusicPlaying ? '正在播放' : '已暂停'}：${store.currentTrack.name}\n左键打开迷你播放列表，右键切下一首`
              : '左键打开音乐播放列表 / 添加文件夹，右键切下一首'
          "
          @click="showMusicPopover = !showMusicPopover"
          @contextmenu.prevent="handleMusicBallRightClick"
        >
          <!-- 底部内嵌细进度条 -->
          <div
            class="absolute bottom-1 left-2 right-2 h-1 rounded-full bg-black/20 dark:bg-white/20 overflow-hidden pointer-events-none"
          >
            <div
              class="h-full transition-all duration-200"
              :class="showMusicPopover ? 'bg-white' : 'bg-pcl-primary'"
              :style="{ width: `${store.musicProgress}%` }"
            />
          </div>

          <span class="text-base relative z-10" :class="store.isMusicPlaying ? 'animate-pulse' : ''">
            {{ store.isMusicPlaying ? '🎵' : '🎶' }}
          </span>
          <span
            class="text-[10px] scale-85 leading-none transition-colors"
            :class="showMusicPopover ? 'text-white font-medium' : 'text-pcl-subtext group-hover:text-pcl-primary'"
          >
            音乐
          </span>
        </button>
      </aside>

      <!-- 核心页面视口（经典模式为无内边距全画幅分屏 PanMain，现代模式带紧凑内边距） -->
      <main
        class="flex-1 overflow-hidden"
        :class="store.uiStyle === 'classic' ? 'p-0' : 'pt-5 px-6 pb-3.5'"
      >
        <Transition name="fade-slide" mode="out-in">
          <component :is="LaunchView" v-if="store.currentTab === 'launch'" />
          <component :is="DownloadView" v-else-if="store.currentTab === 'download'" />
          <component :is="InstanceView" v-else-if="store.currentTab === 'instance'" />
          <component :is="SetupView" v-else-if="store.currentTab === 'setup'" />
          <component :is="LinkView" v-else-if="store.currentTab === 'link'" />
          <component :is="OtherView" v-else-if="store.currentTab === 'other'" />
        </Transition>
      </main>
    </div>

    <!-- ==================== 3. 背景音乐浮层与经典悬浮球 ==================== -->
    <!-- 浮层外部遮罩（点击浮层外部空白区域自动收起播放器） -->
    <div
      v-if="showMusicPopover"
      class="fixed inset-0 z-30"
      @click="showMusicPopover = false"
    />

    <!-- 迷你播放列表浮层（经典模式靠右下角，现代模式靠左下角侧栏旁） -->
    <Transition name="fade-slide">
      <div
        v-if="showMusicPopover"
        class="fixed z-40 text-xs flex flex-col gap-3 transition-colors"
        :class="
          store.uiStyle === 'classic'
            ? 'right-5 bottom-[74px] w-[330px] rounded-[6px] bg-white/98 border border-[var(--pcl-brush-5)] shadow-[0_6px_22px_rgba(19,112,243,0.18)] p-3.5 text-[#343d4a]'
            : 'left-[80px] bottom-3.5 w-[340px] rounded-2xl bg-pcl-card border border-pcl-border shadow-pcl-window p-4 text-pcl-text backdrop-blur-xl'
        "
      >
          <!-- 顶部当前曲目信息与快捷操作 -->
          <div class="flex items-start justify-between gap-2">
            <div class="min-w-0 flex-1">
              <div class="flex items-center gap-1.5">
                <span class="text-sm">{{ store.isMusicPlaying ? '🎵' : '⏸️' }}</span>
                <span
                  class="font-bold truncate text-[13px]"
                  :class="store.uiStyle === 'classic' ? 'text-[var(--pcl-brush-1)]' : 'text-pcl-text'"
                  :title="store.currentTrack?.name"
                >
                  {{ store.currentTrack?.name || '暂无播放中的音乐' }}
                </span>
              </div>
              <div
                class="text-[11px] mt-0.5 flex items-center gap-2"
                :class="store.uiStyle === 'classic' ? 'text-[#737373]' : 'text-pcl-subtext'"
              >
                <span>{{ formatDuration(store.musicCurrentTime) }} / {{ formatDuration(store.musicDuration) }}</span>
                <span>·</span>
                <span>共 {{ store.musicTracks.length }} 首</span>
              </div>
            </div>
            <button
              class="w-6 h-6 flex items-center justify-center cursor-pointer transition-colors"
              :class="
                store.uiStyle === 'classic'
                  ? 'rounded-[4px] text-[#737373] hover:bg-[var(--pcl-brush-7)] hover:text-[var(--pcl-brush-2)]'
                  : 'rounded-lg text-pcl-subtext hover:bg-white/10 hover:text-pcl-text'
              "
              title="收起播放面板"
              @click="showMusicPopover = false"
            >
              ✕
            </button>
          </div>

          <!-- 播放进度条 -->
          <div class="flex items-center gap-2">
            <input
              type="range"
              min="0"
              max="100"
              step="0.5"
              :value="store.musicProgress"
              :disabled="!store.currentTrack"
              class="w-full accent-[var(--pcl-color-primary)] cursor-pointer h-1.5 rounded-lg"
              :class="store.uiStyle === 'classic' ? 'bg-[var(--pcl-brush-6)]' : 'bg-black/20'"
              @input="(e) => store.seekMusic(Number((e.target as HTMLInputElement).value))"
            />
          </div>

          <!-- 核心播放控制栏：循环模式 + EQ 调音音效 + 上一曲/播放暂停/下一曲 + 音量 -->
          <div class="flex items-center justify-between gap-1.5">
            <div class="flex items-center gap-1 shrink-0">
              <button
                class="px-2 py-1 text-[11px] flex items-center gap-1 cursor-pointer transition-all"
                :class="
                  store.uiStyle === 'classic'
                    ? 'rounded-[4px] border border-black/20 bg-white text-[#343d4a] hover:border-[var(--pcl-brush-3)] hover:text-[var(--pcl-brush-2)] hover:bg-[var(--pcl-brush-7)]'
                    : 'rounded-xl border border-pcl-border bg-[var(--pcl-color-bg)] text-pcl-text hover:border-pcl-primary/60 hover:text-pcl-primary'
                "
                :title="`循环模式：${playModeMeta.label}（点击切换）`"
                @click="store.cycleMusicPlayMode()"
              >
                <span>{{ playModeMeta.icon }}</span>
              </button>

              <button
                class="px-2 py-1 text-[11px] flex items-center gap-1 cursor-pointer transition-all"
                :class="
                  store.uiStyle === 'classic'
                    ? 'rounded-[4px] border border-black/20 bg-white text-[#343d4a] hover:border-[var(--pcl-brush-3)] hover:text-[var(--pcl-brush-2)] hover:bg-[var(--pcl-brush-7)]'
                    : 'rounded-xl border border-pcl-border bg-[var(--pcl-color-bg)] text-pcl-text hover:border-pcl-primary/60 hover:text-pcl-primary'
                "
                :title="`调音音效：${currentEqMeta.name}（${currentEqMeta.desc}，点击快速切换预设）`"
                @click="cycleEqPreset()"
              >
                <span>{{ currentEqMeta.icon }}</span>
                <span class="text-[10px] font-medium">{{ currentEqMeta.name }}</span>
              </button>
            </div>

            <div class="flex items-center gap-1">
              <button
                class="w-7 h-7 flex items-center justify-center cursor-pointer transition-colors"
                :class="
                  store.uiStyle === 'classic'
                    ? 'rounded-[4px] text-[#343d4a] hover:bg-[var(--pcl-brush-7)] hover:text-[var(--pcl-brush-2)]'
                    : 'rounded-lg text-pcl-text hover:bg-white/10 hover:text-pcl-primary'
                "
                title="上一曲"
                @click="store.playPrevTrack()"
              >
                ⏮
              </button>
              <button
                class="w-8 h-8 flex items-center justify-center cursor-pointer transition-all active:scale-95"
                :class="
                  store.uiStyle === 'classic'
                    ? 'rounded-full border border-[var(--pcl-brush-3)] bg-[var(--pcl-brush-3)] hover:bg-[var(--pcl-brush-2)] text-white shadow-sm'
                    : 'rounded-xl bg-pcl-primary hover:bg-pcl-hover text-white shadow-md shadow-pcl-primary/25'
                "
                :title="store.isMusicPlaying ? '暂停' : '播放'"
                @click="store.toggleMusicPlay()"
              >
                {{ store.isMusicPlaying ? '⏸' : '▶' }}
              </button>
              <button
                class="w-7 h-7 flex items-center justify-center cursor-pointer transition-colors"
                :class="
                  store.uiStyle === 'classic'
                    ? 'rounded-[4px] text-[#343d4a] hover:bg-[var(--pcl-brush-7)] hover:text-[var(--pcl-brush-2)]'
                    : 'rounded-lg text-pcl-text hover:bg-white/10 hover:text-pcl-primary'
                "
                title="下一曲（也可直接右键音乐按钮）"
                @click="store.playNextTrack()"
              >
                ⏭
              </button>
            </div>

            <div class="flex items-center gap-1 w-[82px] shrink-0" :title="`音量: ${store.musicVolume}%`">
              <span class="text-[11px]">{{ store.musicVolume === 0 ? '🔇' : '🔊' }}</span>
              <input
                type="range"
                min="0"
                max="100"
                :value="store.musicVolume"
                class="w-full accent-[var(--pcl-color-primary)] cursor-pointer h-1"
                @input="(e) => store.setMusicVolume(Number((e.target as HTMLInputElement).value))"
              />
            </div>
          </div>

          <!-- 文件夹快捷操作栏 -->
          <div
            class="flex items-center justify-between gap-1.5 pt-2 border-t"
            :class="store.uiStyle === 'classic' ? 'border-black/8' : 'border-pcl-border'"
          >
            <button
              class="flex-1 py-1.5 px-2.5 font-semibold flex items-center justify-center gap-1 cursor-pointer transition-all"
              :class="
                store.uiStyle === 'classic'
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
                store.uiStyle === 'classic'
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
                store.uiStyle === 'classic'
                  ? 'rounded-[4px] border border-black/20 bg-white text-[#343d4a] hover:border-[var(--pcl-brush-3)] hover:text-[var(--pcl-brush-2)] hover:bg-[var(--pcl-brush-7)]'
                  : 'rounded-xl border border-pcl-border bg-[var(--pcl-color-bg)] text-pcl-text hover:border-pcl-primary/50 hover:text-pcl-primary'
              "
              title="重新扫描所有音乐文件夹"
              @click="store.refreshMusicPlaylist(false)"
            >
              <span>🔄</span>
            </button>
          </div>

          <!-- 已添加的自定义音乐文件夹标签列表 -->
          <div v-if="store.musicDirs.length > 0" class="flex flex-wrap gap-1.5 max-h-[54px] overflow-y-auto">
            <div
              v-for="dir in store.musicDirs"
              :key="dir"
              class="inline-flex items-center gap-1 px-2 py-0.5 text-[10.5px] max-w-full transition-colors"
              :class="
                store.uiStyle === 'classic'
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
                @click="store.removeMusicFolder(dir)"
              >
                ×
              </button>
            </div>
          </div>

          <!-- 曲目搜索框 -->
          <div v-if="store.musicTracks.length > 5">
            <input
              v-model="musicSearchQuery"
              placeholder="搜索歌名或所在目录..."
              class="w-full px-2.5 py-1.5 text-[11px] outline-none transition-colors"
              :class="
                store.uiStyle === 'classic'
                  ? 'rounded-[4px] bg-white border border-black/20 text-[#343d4a] placeholder:text-[#8c8c8c] focus:border-[var(--pcl-brush-3)]'
                  : 'rounded-xl bg-[var(--pcl-color-bg)] border border-pcl-border text-pcl-text placeholder:text-pcl-subtext focus:border-pcl-primary'
              "
            />
          </div>

          <!-- 曲目滚动列表 -->
          <div class="max-h-[180px] overflow-y-auto space-y-1 pr-0.5">
            <div
              v-if="filteredMusicTracks.length === 0"
              class="py-6 text-center text-[11px]"
              :class="store.uiStyle === 'classic' ? 'text-[#737373]' : 'text-pcl-subtext'"
            >
              {{
                store.musicTracks.length === 0
                  ? '暂无音乐，点击上方「➕ 添加文件夹」导入音乐库'
                  : '未找到匹配的曲目'
              }}
            </div>
            <div
              v-for="track in filteredMusicTracks"
              :key="track.path"
              class="px-2.5 py-1.5 flex items-center justify-between gap-2 cursor-pointer transition-colors"
              :class="
                store.uiStyle === 'classic'
                  ? store.currentTrack?.path === track.path
                    ? 'rounded-[4px] bg-[var(--pcl-brush-6)] text-[var(--pcl-brush-2)] font-semibold'
                    : 'rounded-[4px] text-[#343d4a] hover:bg-[var(--pcl-brush-7)]'
                  : store.currentTrack?.path === track.path
                    ? 'rounded-xl bg-pcl-primary/15 border border-pcl-primary/30 text-pcl-primary font-semibold'
                    : 'rounded-xl border border-transparent text-pcl-text hover:bg-white/5'
              "
              :title="track.path"
              @click="store.playTrack(track, true)"
            >
              <div class="flex items-center gap-1.5 min-w-0 flex-1">
                <span class="text-[11px] shrink-0">
                  {{
                    store.currentTrack?.path === track.path
                      ? store.isMusicPlaying
                        ? '🔊'
                        : '⏸'
                      : '♪'
                  }}
                </span>
                <span class="truncate text-[11.5px]">{{ track.name }}</span>
              </div>
              <span
                class="text-[10px] shrink-0"
                :class="store.uiStyle === 'classic' ? 'text-[#737373]' : 'text-pcl-subtext'"
              >
                {{ track.size_mb.toFixed(1) }} MB
              </span>
            </div>
          </div>
        </div>
      </Transition>

    <!-- 经典模式下的右下角背景音乐悬浮球 (PCL2 经典圆形悬浮球，严格固定在右下角 right-5 bottom-5 绝不漂移) -->
    <div
      v-if="store.uiStyle === 'classic'"
      class="fixed z-30 right-5 bottom-5 pointer-events-auto"
    >
      <button
        class="relative transition-all flex items-center justify-center cursor-pointer active:scale-95 w-11 h-11 rounded-full bg-[var(--pcl-brush-3)] text-white shadow-lg hover:bg-[var(--pcl-brush-2)] hover:scale-105"
        :title="
          store.currentTrack
            ? `${store.isMusicPlaying ? '正在播放' : '已暂停'}：${store.currentTrack.name}\n左键打开迷你播放列表，右键切下一首`
            : '左键打开音乐播放列表 / 添加文件夹，右键切下一首'
        "
        @click="showMusicPopover = !showMusicPopover"
        @contextmenu.prevent="handleMusicBallRightClick"
      >
        <!-- 经典模式：外圈圆形进度环 -->
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
            :stroke-dashoffset="122.5 - (122.5 * store.musicProgress) / 100"
          />
        </svg>

        <span class="text-sm relative z-10" :class="store.isMusicPlaying ? 'animate-pulse' : ''">
          {{ store.isMusicPlaying ? '🎵' : '🎶' }}
        </span>
      </button>
    </div>

    <!-- ==================== PCL2 经典全局下载/安装常驻任务浮动栏 ==================== -->
    <Transition name="fade-slide">
      <div
        v-if="store.downloadTask"
        class="fixed bottom-4 left-1/2 -translate-x-1/2 z-50 px-4 py-2.5 rounded-2xl bg-white/95 dark:bg-[#1e232b]/95 backdrop-blur-md border border-black/10 dark:border-white/10 shadow-2xl flex items-center gap-3.5 min-w-[380px] max-w-[540px] select-none transition-all duration-300 hover:shadow-xl hover:-translate-y-0.5 cursor-pointer"
        @click="store.currentTab = 'download'"
      >
        <!-- 任务图标指示 -->
        <div
          class="w-8 h-8 rounded-xl flex items-center justify-center text-sm shrink-0 transition-colors"
          :class="[
            store.downloadTask.error
              ? 'bg-rose-500/15 text-rose-500'
              : store.downloadTask.is_finished
                ? 'bg-emerald-500/15 text-emerald-500'
                : 'bg-pcl-primary/15 text-pcl-primary'
          ]"
        >
          <span v-if="store.downloadTask.error">❌</span>
          <span v-else-if="store.downloadTask.is_finished">✓</span>
          <span v-else class="animate-spin text-xs">⏳</span>
        </div>

        <!-- 任务核心内容区 -->
        <div class="flex-1 min-w-0 flex flex-col gap-1">
          <div class="flex items-center justify-between text-xs">
            <span class="font-bold text-pcl-text truncate max-w-[280px]">
              {{ store.downloadTask.version_id }}
            </span>
            <span
              class="font-mono font-bold shrink-0 ml-2"
              :class="store.downloadTask.error ? 'text-rose-500' : 'text-pcl-primary'"
            >
              {{ store.downloadTask.error ? '任务失败' : `${store.downloadTask.percentage.toFixed(0)}%` }}
            </span>
          </div>

          <!-- 动态进度条 -->
          <div class="w-full h-1.5 rounded-full bg-black/5 dark:bg-white/10 overflow-hidden">
            <div
              class="h-full rounded-full transition-all duration-300 ease-out"
              :class="store.downloadTask.error ? 'bg-rose-500' : 'bg-pcl-primary'"
              :style="{ width: `${store.downloadTask.percentage}%` }"
            />
          </div>

          <!-- 步骤文案与速度/文件数 -->
          <div class="flex items-center justify-between text-[11px] text-pcl-subtext">
            <span class="truncate max-w-[260px]">{{ store.downloadTask.step }}</span>
            <span class="shrink-0 font-mono text-[10px]">{{ store.downloadTask.speed_text }}</span>
          </div>
        </div>

        <!-- 动作操作区 -->
        <div class="shrink-0 flex items-center gap-1.5 pl-1 border-l border-black/8 dark:border-white/10">
          <button
            v-if="store.currentTab !== 'download'"
            class="px-2.5 py-1 rounded-xl text-[11px] font-semibold text-pcl-primary hover:bg-pcl-primary/10 transition-colors"
            title="前往下载页查看详情"
            @click.stop="store.currentTab = 'download'"
          >
            查看
          </button>
          <button
            v-if="store.downloadTask.is_finished || store.downloadTask.error"
            class="p-1 rounded-lg text-pcl-subtext hover:text-pcl-text hover:bg-black/5 dark:hover:bg-white/10 transition-colors text-xs"
            title="关闭提示"
            @click.stop="store.clearDownloadTask()"
          >
            ✕
          </button>
        </div>
      </div>
    </Transition>

    <!-- 全局游戏崩溃诊断弹窗 (PCL2 经典风格) -->
    <CrashDialog
      v-if="crashReport"
      :crash="crashReport"
      @close="crashReport = null"
    />
  </div>
</template>

<style scoped>
.fade-slide-enter-active,
.fade-slide-leave-active {
  transition: all 0.22s var(--pcl-ease-smooth);
}

.fade-slide-enter-from {
  opacity: 0;
  transform: translateY(6px);
}

.fade-slide-leave-to {
  opacity: 0;
  transform: translateY(-6px);
}
</style>
