<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import LaunchView from '@/views/LaunchView.vue'
import InstanceView from '@/views/InstanceView.vue'
import DownloadView from '@/views/DownloadView.vue'
import SetupView from '@/views/SetupView.vue'
import CrashDialog from '@/components/CrashDialog.vue'
import { backend } from '@/services/backend'
import type { CrashAnalysisResult } from '@/services/backend'
import { useAppStore } from '@/stores/app'

const store = useAppStore()
const crashReport = ref<CrashAnalysisResult | null>(null)
let unlistenCrash: (() => void) | null = null

onMounted(async () => {
  store.initConfig()
  unlistenCrash = await backend.onGameCrash((report) => {
    crashReport.value = report
  })
})

onUnmounted(() => {
  if (unlistenCrash) unlistenCrash()
})

const navItems = [
  { id: 'launch', name: '启动', icon: '🚀' },
  { id: 'download', name: '下载', icon: '📥' },
  { id: 'instance', name: '版本', icon: '📦' },
  { id: 'setup', name: '设置', icon: '⚙️' },
  { id: 'link', name: '联机', icon: '🌐' },
]

const handleTitleBarMouseDown = (e: MouseEvent) => {
  if ((e.target as HTMLElement).closest('button')) return
  backend.startDragging()
}
</script>

<template>
  <div class="h-screen w-screen flex flex-col bg-pcl-bg overflow-hidden select-none">
    <!-- 自定义标题栏 (支持窗口拖拽) -->
    <header
      class="h-10 flex items-center justify-between px-4 bg-pcl-card/60 backdrop-blur-md border-b border-pcl-border cursor-default"
      @mousedown="handleTitleBarMouseDown"
    >
      <div class="flex items-center gap-2 text-xs font-semibold text-pcl-text pointer-events-none">
        <span class="text-base">💎</span>
        <span>Plain Craft Launcher 2 (TCL)</span>
      </div>

      <!-- 窗口控制按钮 -->
      <div class="flex items-center gap-1 z-10">
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
      </div>
    </header>

    <!-- 主体区域：左侧导航 + 右侧内容容器 -->
    <div class="flex-1 flex overflow-hidden">
      <!-- 左侧图标导航栏 -->
      <aside class="w-18 flex flex-col items-center py-4 gap-3 bg-pcl-card/30 border-r border-pcl-border">
        <button
          v-for="item in navItems"
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
      </aside>

      <!-- 核心页面视口 -->
      <main class="flex-1 p-6 overflow-hidden">
        <Transition name="fade-slide" mode="out-in">
          <component :is="LaunchView" v-if="store.currentTab === 'launch'" />
          <component :is="DownloadView" v-else-if="store.currentTab === 'download'" />
          <component :is="InstanceView" v-else-if="store.currentTab === 'instance'" />
          <component :is="SetupView" v-else-if="store.currentTab === 'setup'" />
          <div v-else class="h-full flex items-center justify-center text-pcl-subtext text-sm">
            模块 [{{ store.currentTab }}] 正在从旧版 PCL2 迁移重构中...
          </div>
        </Transition>
      </main>
    </div>

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
  transition: all 0.25s var(--pcl-ease-smooth);
}

.fade-slide-enter-from {
  opacity: 0;
  transform: translateY(8px);
}

.fade-slide-leave-to {
  opacity: 0;
  transform: translateY(-8px);
}
</style>
