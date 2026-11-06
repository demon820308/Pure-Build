<script setup lang="ts">
import { defineAsyncComponent, onMounted, onUnmounted, ref } from 'vue'
import LaunchView from '@/views/LaunchView.vue'
import CrashDialog from '@/components/CrashDialog.vue'
import PclIcon from '@/components/base/PclIcon.vue'
import MusicPlayerBar from '@/components/music/MusicPlayerBar.vue'
import { backend } from '@/services/backend'
import type { CrashAnalysisResult } from '@/services/backend'
import { useAppStore } from '@/stores/app'

// 非首屏视图按需懒加载，大幅缩减首包解析体积（消除白屏 RC3）
const InstanceView = defineAsyncComponent(() => import('@/views/InstanceView.vue'))
const DownloadView = defineAsyncComponent(() => import('@/views/DownloadView.vue'))
const SetupView = defineAsyncComponent(() => import('@/views/SetupView.vue'))
const LinkView = defineAsyncComponent(() => import('@/views/LinkView.vue'))
const OtherView = defineAsyncComponent(() => import('@/views/OtherView.vue'))

const store = useAppStore()
const musicPlayerRef = ref<InstanceType<typeof MusicPlayerBar> | null>(null)
const crashReport = ref<CrashAnalysisResult | null>(null)
let unlistenCrash: (() => void) | null = null
let unlistenLaunchBgm: (() => void) | null = null
let unlistenExitBgm: (() => void) | null = null
let unlistenDownloadTask: (() => void) | null = null
const handleWindowFocus = () => {
  void store.reconcileOnFocus()
}

// 捕获阶段拦截顶栏双击 (e.detail >= 2)，在 Tauri drag.js 监听前阻断双击最大化请求
const handleHeaderMouseDown = (e: MouseEvent) => {
  if (e.detail >= 2) {
    e.preventDefault()
    e.stopPropagation()
  }
}

onMounted(async () => {
  const t2 = performance.timeOrigin + performance.now()
  const t1 = (window as unknown as { __PBL_T1__?: number }).__PBL_T1__ || t2
  try {
    await store.initConfig()
  } catch (err) {
    console.error('initConfig error:', err)
  } finally {
    const t3 = performance.timeOrigin + performance.now()
    void backend.reportStartupPerf(t1, t2, t3)
    backend.showMainWindow()
  }
  unlistenCrash = await backend.onGameCrash((report) => {
    crashReport.value = report
    store.resetGameState()
    store.handleMusicOnGameExit()
  })
  // 游戏进程拉起时同步全局运行状态并自动暂停背景音乐，退出时自动恢复
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
  window.addEventListener('focus', handleWindowFocus)
})

onUnmounted(() => {
  if (unlistenCrash) unlistenCrash()
  if (unlistenLaunchBgm) unlistenLaunchBgm()
  if (unlistenExitBgm) unlistenExitBgm()
  if (unlistenDownloadTask) unlistenDownloadTask()
  window.removeEventListener('focus', handleWindowFocus)
})

const modernNavItems: Array<{
  id: string
  name: string
  icon: 'launch' | 'download' | 'instance' | 'setup' | 'link' | 'more'
}> = [
  { id: 'launch', name: '启动', icon: 'launch' },
  { id: 'download', name: '下载', icon: 'download' },
  { id: 'instance', name: '版本', icon: 'instance' },
  { id: 'setup', name: '设置', icon: 'setup' },
  { id: 'link', name: '联机', icon: 'link' },
  { id: 'other', name: '百宝箱', icon: 'more' },
]

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
    <!-- 全局非阻塞通知横幅 -->
    <Transition name="fade-slide">
      <div
        v-if="store.notificationMessage"
        class="fixed top-4 left-1/2 -translate-x-1/2 z-50 px-3.5 py-1.5 bg-black/85 dark:bg-white/95 text-white dark:text-black rounded-md shadow-md text-xs font-medium flex items-center gap-2 backdrop-blur-md pointer-events-none select-none max-w-[90vw]"
      >
        <PclIcon name="info" :size="13" />
        <span>{{ store.notificationMessage }}</span>
      </div>
    </Transition>

    <div
      v-if="store.customBgUrl"
      class="absolute inset-0 pointer-events-none z-0 bg-cover bg-center transition-all duration-300"
      :style="{
        backgroundImage: `url(${store.customBgUrl})`,
        filter: `blur(${store.bgBlur}px)`,
        opacity: (100 - store.bgOpacity) / 100,
      }"
    />

    <header
      v-if="store.uiStyle === 'classic'"
      data-tauri-drag-region="deep"
      class="relative z-10 h-[52px] flex items-center justify-between px-5 bg-white dark:bg-[#1e232b] border-b border-black/8 dark:border-white/10 cursor-default shrink-0"
      @mousedown.capture="handleHeaderMouseDown"
    >
      <div data-tauri-drag-region="deep" class="flex items-center gap-2.5">
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
          <span class="text-[14px] font-semibold text-[#1D1D1F] dark:text-gray-100 tracking-wide pointer-events-none">
            Pure Build Launcher
          </span>
        </template>
      </div>

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

        <div class="w-[1px] h-4 bg-[#D1D1D6] dark:bg-white/20 mx-1" />
        <button
          class="h-[26px] px-2.5 rounded-[13px] text-[11px] font-medium text-[var(--pcl-brush-2)] hover:bg-[var(--pcl-brush-6)] transition-colors cursor-pointer"
          title="切换为现代侧栏布局 (可在设置中随时切回)"
          @click="store.setUiStyle('modern')"
        >
          现代布局
        </button>

        <!-- macOS 使用原生红绿灯，Windows/Linux 显示自绘按钮 -->
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

    <header
      v-else
      data-tauri-drag-region="deep"
      class="relative z-10 h-10 flex items-center justify-between px-4 bg-pcl-card border-b border-pcl-border cursor-default shrink-0"
      @mousedown.capture="handleHeaderMouseDown"
    >
      <div data-tauri-drag-region="deep" class="flex items-center gap-2 text-xs font-semibold text-pcl-text pointer-events-none">
        <img
          src="/favicon.png"
          alt="PBL"
          class="w-5 h-5 object-contain select-none drop-shadow-sm"
        />
        <span>Pure Build Launcher</span>
      </div>

      <div class="flex items-center gap-1.5 z-10">
        <button
          class="px-2.5 h-7 flex items-center gap-1 rounded-md border border-pcl-border bg-black/5 dark:bg-white/5 hover:bg-black/10 dark:hover:bg-white/10 text-pcl-subtext hover:text-pcl-text text-[11px] font-medium transition-colors cursor-pointer"
          title="切换为经典布局"
          @click="store.setUiStyle('classic')"
        >
          经典布局
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

    <div class="relative z-10 flex-1 flex overflow-hidden">
      <aside
        v-if="store.uiStyle === 'modern'"
        class="w-16 flex flex-col items-center py-3 gap-2 bg-pcl-card border-r border-pcl-border shrink-0 select-none"
      >
        <button
          v-for="item in modernNavItems"
          :key="item.id"
          class="w-11 h-11 rounded-xl flex flex-col items-center justify-center gap-1 cursor-pointer transition-colors duration-150"
          :class="[
            store.currentTab === item.id
              ? 'bg-pcl-primary text-white font-medium shadow-xs'
              : 'text-pcl-subtext hover:bg-black/5 dark:hover:bg-white/5 hover:text-pcl-text'
          ]"
          @click="store.currentTab = item.id"
        >
          <PclIcon :name="item.icon" :size="16" />
          <span class="text-[10px] leading-none">{{ item.name }}</span>
        </button>

        <button
          class="mt-auto w-11 h-11 rounded-xl flex flex-col items-center justify-center relative cursor-pointer transition-colors duration-150 group"
          :class="[
            musicPlayerRef?.showMusicPopover
              ? 'bg-pcl-primary text-white font-medium shadow-xs'
              : 'text-pcl-subtext hover:bg-black/5 dark:hover:bg-white/5 hover:text-pcl-text'
          ]"
          :title="
            store.currentTrack
              ? `${store.isMusicPlaying ? '正在播放' : '已暂停'}：${store.currentTrack.name}\n左键打开播放列表，右键切下一首`
              : '左键打开音乐播放列表，右键切下一首'
          "
          @click="musicPlayerRef && (musicPlayerRef.showMusicPopover = !musicPlayerRef.showMusicPopover)"
          @contextmenu.prevent="musicPlayerRef?.handleMusicBallRightClick()"
        >
          <div
            class="absolute bottom-1 left-2 right-2 h-0.5 rounded-full bg-black/20 dark:bg-white/20 overflow-hidden pointer-events-none"
          >
            <div
              class="h-full transition-all duration-200"
              :class="musicPlayerRef?.showMusicPopover ? 'bg-white' : 'bg-pcl-primary'"
              :style="{ width: `${store.musicProgress}%` }"
            />
          </div>

          <PclIcon name="music" :size="16" />
          <span class="text-[10px] leading-none mt-1">
            音乐
          </span>
        </button>
      </aside>

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

    <MusicPlayerBar ref="musicPlayerRef" />

    <Transition name="fade-slide">
      <div
        v-if="store.downloadTask"
        class="fixed bottom-4 left-1/2 -translate-x-1/2 z-50 px-4 py-2.5 rounded-2xl bg-white/95 dark:bg-[#1e232b]/95 backdrop-blur-md border border-black/10 dark:border-white/10 shadow-2xl flex items-center gap-3.5 min-w-[380px] max-w-[540px] select-none transition-all duration-300 hover:shadow-xl hover:-translate-y-0.5 cursor-pointer"
        @click="store.currentTab = 'download'"
      >
        <div
          class="w-7 h-7 rounded-lg flex items-center justify-center shrink-0 transition-colors"
          :class="[
            store.downloadTask.error
              ? 'bg-rose-500/10 text-rose-500'
              : store.downloadTask.is_finished
                ? 'bg-emerald-500/10 text-emerald-500'
                : 'bg-pcl-primary/10 text-pcl-primary'
          ]"
        >
          <PclIcon v-if="store.downloadTask.error" name="close" :size="12" />
          <PclIcon v-else-if="store.downloadTask.is_finished" name="check" :size="13" />
          <PclIcon v-else name="refresh" :size="12" class="animate-spin" />
        </div>

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

          <div class="w-full h-1.5 rounded-full bg-black/5 dark:bg-white/10 overflow-hidden">
            <div
              class="h-full rounded-full transition-all duration-300 ease-out"
              :class="store.downloadTask.error ? 'bg-rose-500' : 'bg-pcl-primary'"
              :style="{ width: `${store.downloadTask.percentage}%` }"
            />
          </div>

          <div class="flex items-center justify-between text-[11px] text-pcl-subtext">
            <span class="truncate max-w-[260px]">{{ store.downloadTask.step }}</span>
            <span class="shrink-0 font-mono text-[10px]">{{ store.downloadTask.speed_text }}</span>
          </div>
        </div>

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
