<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useAppStore } from '@/stores/app'
import MyListItem from '@/components/base/MyListItem.vue'
import { backend } from '@/services/backend'
import type { RemoteVersion } from '@/services/backend'
import PclIcon from '@/components/base/PclIcon.vue'

import GameVersionCenter from '@/components/download/GameVersionCenter.vue'
import HorizonDownloadCenter from '@/components/download/HorizonDownloadCenter.vue'
import HorizonContentPanel from '@/components/download/HorizonContentPanel.vue'
import InstallConfigPanel from '@/components/download/InstallConfigPanel.vue'
import ModrinthBrowser from '@/components/download/ModrinthBrowser.vue'
import ChangelogModal from '@/components/download/ChangelogModal.vue'

const store = useAppStore()

const currentPlatform = ref<'java' | 'horizon'>('java')
const javaSection = ref<'game' | 'mod' | 'modpack' | 'shader' | 'resourcepack'>('game')
const horizonSection = ref<'builds' | 'mod' | 'shader' | 'resourcepack' | 'native-mod'>('builds')
const horizonCenterRef = ref<InstanceType<typeof HorizonDownloadCenter> | null>(null)
const horizonContentRef = ref<InstanceType<typeof HorizonContentPanel> | null>(null)

const selectJavaSection = (section: 'game' | 'mod' | 'modpack' | 'shader' | 'resourcepack') => {
  currentPlatform.value = 'java'
  javaSection.value = section
  configuringVersion.value = null
}

const selectHorizonSection = (
  section: 'builds' | 'mod' | 'shader' | 'resourcepack' | 'native-mod'
) => {
  currentPlatform.value = 'horizon'
  horizonSection.value = section
  configuringVersion.value = null
}

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
      class="w-[156px] h-full bg-white dark:bg-[#1e232b] border-r border-black/8 dark:border-white/10 pt-3 flex flex-col shrink-0 select-none overflow-y-auto"
    >
      <div class="px-3.5 pb-1 text-[12px] font-medium text-[#737373] opacity-85">
        原版 - Java
      </div>
      <MyListItem
        title="游戏版本"
        icon="vanilla"
        :checked="currentPlatform === 'java' && javaSection === 'game'"
        action-title="刷新版本列表"
        @click="selectJavaSection('game')"
        @action="loadRemoteVersions"
      />
      <MyListItem
        title="社区 Mod"
        icon="mod"
        :checked="currentPlatform === 'java' && javaSection === 'mod'"
        action-title="刷新 Mod 列表"
        @click="selectJavaSection('mod')"
        @action="modrinthBrowserRef?.searchCommunity"
      />
      <MyListItem
        title="整合包"
        icon="modpack"
        :checked="currentPlatform === 'java' && javaSection === 'modpack'"
        action-title="刷新整合包列表"
        @click="selectJavaSection('modpack')"
        @action="modrinthBrowserRef?.searchCommunity"
      />
      <MyListItem
        title="资源包"
        icon="resourcepack"
        :checked="currentPlatform === 'java' && javaSection === 'resourcepack'"
        action-title="刷新资源包列表"
        @click="selectJavaSection('resourcepack')"
        @action="modrinthBrowserRef?.searchCommunity"
      />
      <MyListItem
        title="光影包"
        icon="shader"
        :checked="currentPlatform === 'java' && javaSection === 'shader'"
        action-title="刷新光影包列表"
        @click="selectJavaSection('shader')"
        @action="modrinthBrowserRef?.searchCommunity"
      />

      <div class="px-3.5 pt-5 pb-1 text-[12px] font-medium text-[#737373] opacity-85">
        Horizon - Rust
      </div>
      <MyListItem
        title="构建与资源"
        icon="launch"
        :checked="currentPlatform === 'horizon' && horizonSection === 'builds'"
        action-title="刷新 Horizon 构建"
        @click="selectHorizonSection('builds')"
        @action="horizonCenterRef?.loadBuilds()"
      />
      <MyListItem
        title="内容模组"
        icon="mod"
        :checked="currentPlatform === 'horizon' && horizonSection === 'mod'"
        action-title="刷新内容模组"
        @click="selectHorizonSection('mod')"
        @action="horizonContentRef?.loadContentCaps()"
      />
      <MyListItem
        title="光影包"
        icon="shader"
        :checked="currentPlatform === 'horizon' && horizonSection === 'shader'"
        action-title="刷新 Horizon 光影包"
        @click="selectHorizonSection('shader')"
        @action="horizonContentRef?.loadContentCaps()"
      />
      <MyListItem
        title="材质包"
        icon="resourcepack"
        :checked="currentPlatform === 'horizon' && horizonSection === 'resourcepack'"
        action-title="刷新 Horizon 材质包"
        @click="selectHorizonSection('resourcepack')"
        @action="horizonContentRef?.loadContentCaps()"
      />
      <MyListItem
        title="原生重编类"
        icon="gear"
        :checked="currentPlatform === 'horizon' && horizonSection === 'native-mod'"
        action-title="刷新原生模组列表"
        @click="selectHorizonSection('native-mod')"
        @action="horizonContentRef?.loadContentCaps()"
      />
    </div>

    <div
      class="flex-1 h-full flex flex-col gap-3.5 overflow-hidden relative"
      :class="store.uiStyle === 'classic' && 'p-5'"
    >
      <div
        v-if="store.uiStyle === 'modern'"
        class="flex items-center justify-between gap-2.5 shrink-0 pb-1 border-b border-pcl-border flex-wrap"
      >
        <div class="flex items-center gap-1 p-1 rounded-lg bg-black/5 dark:bg-white/5 border border-pcl-border shrink-0">
          <button
            class="px-3 py-1 rounded-md text-xs font-semibold cursor-pointer transition-colors whitespace-nowrap shrink-0"
            :class="currentPlatform === 'java' ? 'bg-pcl-primary text-white shadow-xs' : 'text-pcl-subtext hover:text-pcl-text'"
            @click="currentPlatform = 'java'; configuringVersion = null"
          >
            原版 - Java
          </button>
          <button
            class="px-3 py-1 rounded-md text-xs font-semibold cursor-pointer transition-colors whitespace-nowrap shrink-0"
            :class="currentPlatform === 'horizon' ? 'bg-pcl-primary text-white shadow-xs' : 'text-pcl-subtext hover:text-pcl-text'"
            @click="currentPlatform = 'horizon'; configuringVersion = null"
          >
            Horizon - Rust
          </button>
        </div>

        <div
          v-if="currentPlatform === 'java'"
          class="flex items-center gap-1 p-1 rounded-lg bg-black/5 dark:bg-white/5 border border-pcl-border shrink-0 overflow-x-auto no-scrollbar"
        >
          <button
            class="px-2.5 py-1 rounded-md text-xs font-medium cursor-pointer transition-colors flex items-center gap-1.5 whitespace-nowrap shrink-0"
            :class="javaSection === 'game' ? 'bg-pcl-card text-pcl-text shadow-xs' : 'text-pcl-subtext hover:text-pcl-text'"
            @click="selectJavaSection('game')"
          >
            <PclIcon name="vanilla" :size="13" />
            <span>游戏版本</span>
          </button>

          <button
            class="px-2.5 py-1 rounded-md text-xs font-medium cursor-pointer transition-colors flex items-center gap-1.5 whitespace-nowrap shrink-0"
            :class="javaSection === 'mod' ? 'bg-pcl-card text-pcl-text shadow-xs' : 'text-pcl-subtext hover:text-pcl-text'"
            @click="selectJavaSection('mod')"
          >
            <PclIcon name="mod" :size="13" />
            <span>社区 Mod</span>
          </button>

          <button
            class="px-2.5 py-1 rounded-md text-xs font-medium cursor-pointer transition-colors flex items-center gap-1.5 whitespace-nowrap shrink-0"
            :class="javaSection === 'modpack' ? 'bg-pcl-card text-pcl-text shadow-xs' : 'text-pcl-subtext hover:text-pcl-text'"
            @click="selectJavaSection('modpack')"
          >
            <PclIcon name="modpack" :size="13" />
            <span>整合包</span>
          </button>

          <button
            class="px-2.5 py-1 rounded-md text-xs font-medium cursor-pointer transition-colors flex items-center gap-1.5 whitespace-nowrap shrink-0"
            :class="javaSection === 'shader' ? 'bg-pcl-card text-pcl-text shadow-xs' : 'text-pcl-subtext hover:text-pcl-text'"
            @click="selectJavaSection('shader')"
          >
            <PclIcon name="shader" :size="13" />
            <span>光影包</span>
          </button>

          <button
            class="px-2.5 py-1 rounded-md text-xs font-medium cursor-pointer transition-colors flex items-center gap-1.5 whitespace-nowrap shrink-0"
            :class="javaSection === 'resourcepack' ? 'bg-pcl-card text-pcl-text shadow-xs' : 'text-pcl-subtext hover:text-pcl-text'"
            @click="selectJavaSection('resourcepack')"
          >
            <PclIcon name="resourcepack" :size="13" />
            <span>材质包</span>
          </button>
        </div>

        <div
          v-else
          class="flex items-center gap-1 p-1 rounded-lg bg-black/5 dark:bg-white/5 border border-pcl-border shrink-0 overflow-x-auto no-scrollbar"
        >
          <button
            class="px-2.5 py-1 rounded-md text-xs font-medium cursor-pointer transition-colors flex items-center gap-1.5 whitespace-nowrap shrink-0"
            :class="horizonSection === 'builds' ? 'bg-pcl-card text-pcl-text shadow-xs' : 'text-pcl-subtext hover:text-pcl-text'"
            @click="selectHorizonSection('builds')"
          >
            <PclIcon name="launch" :size="13" />
            <span>构建与资源</span>
          </button>

          <button
            class="px-2.5 py-1 rounded-md text-xs font-medium cursor-pointer transition-colors flex items-center gap-1.5 whitespace-nowrap shrink-0"
            :class="horizonSection === 'mod' ? 'bg-pcl-card text-pcl-text shadow-xs' : 'text-pcl-subtext hover:text-pcl-text'"
            @click="selectHorizonSection('mod')"
          >
            <PclIcon name="mod" :size="13" />
            <span>内容模组</span>
          </button>

          <button
            class="px-2.5 py-1 rounded-md text-xs font-medium cursor-pointer transition-colors flex items-center gap-1.5 whitespace-nowrap shrink-0"
            :class="horizonSection === 'shader' ? 'bg-pcl-card text-pcl-text shadow-xs' : 'text-pcl-subtext hover:text-pcl-text'"
            @click="selectHorizonSection('shader')"
          >
            <PclIcon name="shader" :size="13" />
            <span>光影包</span>
          </button>

          <button
            class="px-2.5 py-1 rounded-md text-xs font-medium cursor-pointer transition-colors flex items-center gap-1.5 whitespace-nowrap shrink-0"
            :class="horizonSection === 'resourcepack' ? 'bg-pcl-card text-pcl-text shadow-xs' : 'text-pcl-subtext hover:text-pcl-text'"
            @click="selectHorizonSection('resourcepack')"
          >
            <PclIcon name="resourcepack" :size="13" />
            <span>材质包</span>
          </button>

          <button
            class="px-2.5 py-1 rounded-md text-xs font-medium cursor-pointer transition-colors flex items-center gap-1.5 whitespace-nowrap shrink-0"
            :class="horizonSection === 'native-mod' ? 'bg-pcl-card text-pcl-text shadow-xs' : 'text-pcl-subtext hover:text-pcl-text'"
            @click="selectHorizonSection('native-mod')"
          >
            <PclIcon name="gear" :size="13" />
            <span>原生重编类</span>
          </button>
        </div>
      </div>

      <template v-if="currentPlatform === 'java'">
        <template v-if="javaSection === 'game'">
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
          :section="javaSection"
        />
      </template>

      <template v-else>
        <HorizonDownloadCenter
          v-if="horizonSection === 'builds'"
          ref="horizonCenterRef"
        />
        <HorizonContentPanel
          v-else
          ref="horizonContentRef"
          :kind="horizonSection"
          @go-builds="selectHorizonSection('builds')"
        />
      </template>

      <ChangelogModal
        v-if="changelogModalVersion"
        :version="changelogModalVersion"
        @close="changelogModalVersion = null"
      />
    </div>
  </div>
</template>
