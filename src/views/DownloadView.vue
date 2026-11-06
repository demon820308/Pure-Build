<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useAppStore } from '@/stores/app'
import MyListItem from '@/components/base/MyListItem.vue'
import { backend } from '@/services/backend'
import type { RemoteVersion } from '@/services/backend'
import PclIcon from '@/components/base/PclIcon.vue'

import GameVersionCenter from '@/components/download/GameVersionCenter.vue'
import InstallConfigPanel from '@/components/download/InstallConfigPanel.vue'
import ModrinthBrowser from '@/components/download/ModrinthBrowser.vue'
import ChangelogModal from '@/components/download/ChangelogModal.vue'

const store = useAppStore()

const currentSection = ref<'game' | 'mod' | 'modpack' | 'shader' | 'resourcepack'>('game')

const versions = ref<RemoteVersion[]>([])
const isLoadingVersions = ref(false)
const configuringVersion = ref<RemoteVersion | null>(null)
const changelogModalVersion = ref<RemoteVersion | null>(null)

const modrinthBrowserRef = ref<InstanceType<typeof ModrinthBrowser> | null>(null)

const loadRemoteVersions = async () => {
  isLoadingVersions.value = true
  try {
    versions.value = await backend.fetchRemoteVersions()
  } catch (err: any) {
    alert(`获取版本列表失败: ${err}`)
  } finally {
    isLoadingVersions.value = false
  }
}

const openVersionConfig = (v: RemoteVersion) => {
  configuringVersion.value = v
}

const handleStartInstall = async (data: {
  targetName: string
  loaderType?: 'fabric' | 'forge' | 'neoforge' | 'optifine'
  loaderVersion?: string
}) => {
  if (!configuringVersion.value) return
  const v = configuringVersion.value
  configuringVersion.value = null

  try {
    store.updateDownloadProgress({
      version_id: data.targetName,
      vanilla_version: v.id,
      step: '正在初始化安装任务...',
      percentage: 5,
      speed_text: '连接中',
      is_finished: false,
    })
    await backend.startInstallVersion({
      gameDir: store.gameDir,
      vanillaVersion: v.id,
      targetName: data.targetName,
      metaUrl: v.url,
      loaderType: data.loaderType,
      loaderVersion: data.loaderVersion,
    })
  } catch (err: any) {
    alert(`安装失败: ${err}`)
    store.clearDownloadTask()
  }
}

onMounted(() => {
  store.refreshInstalledVersions()
  loadRemoteVersions()
})
</script>

<template>
  <div class="h-full flex overflow-hidden relative" :class="store.uiStyle === 'classic' ? 'flex-row gap-0' : 'flex-col gap-3.5'">
    <div
      v-if="store.uiStyle === 'classic'"
      class="w-[152px] h-full bg-white dark:bg-[#1e232b] border-r border-black/8 dark:border-white/10 pt-3 flex flex-col shrink-0 select-none"
    >
      <MyListItem
        title="原版游戏"
        icon="vanilla"
        :checked="currentSection === 'game'"
        action-title="刷新版本列表"
        @click="currentSection = 'game'; configuringVersion = null"
        @action="loadRemoteVersions"
      />

      <div class="px-3.5 pt-5 pb-1 text-[12px] text-[#737373] opacity-80">
        社区资源
      </div>

      <MyListItem
        title="Mod"
        icon="mod"
        :checked="currentSection === 'mod'"
        action-title="刷新 Mod 列表"
        @click="currentSection = 'mod'"
        @action="modrinthBrowserRef?.searchCommunity"
      />
      <MyListItem
        title="整合包"
        icon="modpack"
        :checked="currentSection === 'modpack'"
        action-title="刷新整合包列表"
        @click="currentSection = 'modpack'"
        @action="modrinthBrowserRef?.searchCommunity"
      />
      <MyListItem
        title="资源包"
        icon="resourcepack"
        :checked="currentSection === 'resourcepack'"
        action-title="刷新资源包列表"
        @click="currentSection = 'resourcepack'"
        @action="modrinthBrowserRef?.searchCommunity"
      />
      <MyListItem
        title="光影包"
        icon="shader"
        :checked="currentSection === 'shader'"
        action-title="刷新光影包列表"
        @click="currentSection = 'shader'"
        @action="modrinthBrowserRef?.searchCommunity"
      />
    </div>

    <div
      class="flex-1 h-full flex flex-col gap-3.5 overflow-hidden relative"
      :class="store.uiStyle === 'classic' && 'p-5'"
    >
      <div
        v-if="store.uiStyle === 'modern'"
        class="flex items-center justify-between gap-3 shrink-0 pb-1 border-b border-pcl-border"
      >
        <div class="flex items-center gap-1 p-1 rounded-lg bg-black/5 dark:bg-white/5 border border-pcl-border shrink-0 overflow-x-auto no-scrollbar">
          <button
            class="px-2.5 py-1 rounded-md text-xs font-medium cursor-pointer transition-colors flex items-center gap-1.5 whitespace-nowrap shrink-0"
            :class="currentSection === 'game' ? 'bg-pcl-primary text-white shadow-xs' : 'text-pcl-subtext hover:text-pcl-text'"
            @click="currentSection = 'game'"
          >
            <PclIcon name="vanilla" :size="13" />
            <span>原版游戏</span>
          </button>

          <button
            class="px-2.5 py-1 rounded-md text-xs font-medium cursor-pointer transition-colors flex items-center gap-1.5 whitespace-nowrap shrink-0"
            :class="currentSection === 'mod' ? 'bg-pcl-primary text-white shadow-xs' : 'text-pcl-subtext hover:text-pcl-text'"
            @click="currentSection = 'mod'"
          >
            <PclIcon name="mod" :size="13" />
            <span>社区 Mod</span>
          </button>

          <button
            class="px-2.5 py-1 rounded-md text-xs font-medium cursor-pointer transition-colors flex items-center gap-1.5 whitespace-nowrap shrink-0"
            :class="currentSection === 'modpack' ? 'bg-pcl-primary text-white shadow-xs' : 'text-pcl-subtext hover:text-pcl-text'"
            @click="currentSection = 'modpack'"
          >
            <PclIcon name="modpack" :size="13" />
            <span>整合包</span>
          </button>

          <button
            class="px-2.5 py-1 rounded-md text-xs font-medium cursor-pointer transition-colors flex items-center gap-1.5 whitespace-nowrap shrink-0"
            :class="currentSection === 'shader' ? 'bg-pcl-primary text-white shadow-xs' : 'text-pcl-subtext hover:text-pcl-text'"
            @click="currentSection = 'shader'"
          >
            <PclIcon name="shader" :size="13" />
            <span>光影包</span>
          </button>

          <button
            class="px-2.5 py-1 rounded-md text-xs font-medium cursor-pointer transition-colors flex items-center gap-1.5 whitespace-nowrap shrink-0"
            :class="currentSection === 'resourcepack' ? 'bg-pcl-primary text-white shadow-xs' : 'text-pcl-subtext hover:text-pcl-text'"
            @click="currentSection = 'resourcepack'"
          >
            <PclIcon name="resourcepack" :size="13" />
            <span>材质包</span>
          </button>
        </div>

        <div class="text-xs text-pcl-subtext flex items-center gap-2 shrink-0 whitespace-nowrap min-w-0">
          <span
            v-if="store.currentVersion"
            class="px-2 py-0.5 rounded-md bg-black/5 dark:bg-white/5 font-mono text-[11px] max-w-[150px] truncate shrink-0"
            :title="`当前版本: ${store.currentVersion.name}`"
          >
            {{ store.currentVersion.name }}
          </span>
          <span class="text-emerald-500 flex items-center gap-1.5 text-[11px] whitespace-nowrap shrink-0 font-medium">
            <span class="w-1.5 h-1.5 rounded-full bg-emerald-500 shrink-0" />
            <span>镜像在线</span>
          </span>
        </div>
      </div>

      <template v-if="currentSection === 'game'">
        <InstallConfigPanel
          v-if="configuringVersion"
          :version="configuringVersion"
          @close="configuringVersion = null"
          @start-install="handleStartInstall"
        />
        <GameVersionCenter
          v-else
          :versions="versions"
          :is-loading="isLoadingVersions"
          @refresh="loadRemoteVersions"
          @configure="openVersionConfig"
          @show-changelog="(v) => changelogModalVersion = v"
        />
      </template>

      <ModrinthBrowser
        v-else
        ref="modrinthBrowserRef"
        :section="currentSection"
      />

      <ChangelogModal
        v-if="changelogModalVersion"
        :version="changelogModalVersion"
        @close="changelogModalVersion = null"
      />
    </div>
  </div>
</template>
