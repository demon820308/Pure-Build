<script setup lang="ts">
import { ref, computed } from 'vue'
import { useAppStore } from '@/stores/app'
import { backend } from '@/services/backend'
import type { RemoteVersion } from '@/services/backend'
import MyButton from '@/components/base/MyButton.vue'
import PclIcon from '@/components/base/PclIcon.vue'
import InlineProgress from './InlineProgress.vue'
import { open } from '@tauri-apps/plugin-dialog'

const props = defineProps<{
  versions: RemoteVersion[]
  isLoading: boolean
}>()

const emit = defineEmits<{
  (e: 'refresh'): void
  (e: 'configure', version: RemoteVersion): void
  (e: 'showChangelog', version: RemoteVersion): void
}>()

const store = useAppStore()

const filterType = ref<'all' | 'release' | 'snapshot' | 'installed'>('release')
const searchQuery = ref('')

const isVersionDownloading = (vId: string) => {
  const task = store.downloadTask
  if (!task) return false
  return (
    task.version_id === vId ||
    task.vanilla_version === vId ||
    task.version_id.startsWith(`${vId}-`) ||
    task.version_id.startsWith(`${vId} `)
  )
}

const isInstalled = (versionId: string) => {
  return store.installedVersions.some(
    (iv) => iv.id === versionId || iv.name === versionId || iv.id.startsWith(versionId)
  )
}

const getInstalledObj = (versionId: string) => {
  return store.installedVersions.find(
    (iv) => iv.id === versionId || iv.name === versionId || iv.id.startsWith(versionId)
  )
}

const installedCount = computed(() => {
  return props.versions.filter((v) => isInstalled(v.id)).length
})

const goToLaunch = (versionId: string) => {
  const target = getInstalledObj(versionId)
  if (target) {
    store.currentVersion = target
  }
  store.currentTab = 'launch'
}

const filteredVersions = computed(() => {
  let list = props.versions

  if (filterType.value === 'installed') {
    list = list.filter((v) => isInstalled(v.id))
  } else if (filterType.value !== 'all') {
    list = list.filter((v) => v.type === filterType.value)
  }

  const q = searchQuery.value.toLowerCase().trim()
  if (q) {
    list = list.filter((v) => v.id.toLowerCase().includes(q))
  }

  return list
})

const formatDate = (isoStr: string) => {
  try {
    const d = new Date(isoStr)
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
  } catch {
    return isoStr
  }
}

// 另存为：选择文件夹后将完整客户端及依赖下载到独立目录
const handleSaveAs = async (v: RemoteVersion) => {
  try {
    const selected = await open({
      directory: true,
      multiple: false,
      title: `选择将 Minecraft ${v.id} 另存为的目标文件夹`,
    })
    if (!selected) return

    store.updateDownloadProgress({
      version_id: `${v.id}-saveas`,
      vanilla_version: v.id,
      step: '正在初始化独立版本另存为...',
      percentage: 5,
      speed_text: '连接中',
      is_finished: false,
    })

    await backend.startInstallVersion({
      gameDir: selected as string,
      vanillaVersion: v.id,
      targetName: v.id,
      metaUrl: v.url,
    })
  } catch (err: any) {
    alert(`另存为失败: ${err}`)
    store.clearDownloadTask()
  }
}

// 下载官方服务端 server.jar 并生成启动脚本
const handleDownloadServer = async (v: RemoteVersion) => {
  try {
    const selected = await open({
      directory: true,
      multiple: false,
      title: `选择存放 Minecraft ${v.id} 官方服务端的文件夹`,
    })
    if (!selected) return

    store.updateDownloadProgress({
      version_id: `${v.id}-server`,
      vanilla_version: v.id,
      step: '正在准备下载 Minecraft 官方服务端...',
      percentage: 5,
      speed_text: '连接中',
      is_finished: false,
    })

    const scriptName = await backend.downloadMinecraftServer({
      targetDir: selected as string,
      versionId: v.id,
      metaUrl: v.url,
    })

    alert(`Minecraft ${v.id} 官方服务端已成功下载到：\n${selected}\n包含配套脚本【${scriptName}】与 eula.txt，运行该脚本即可开服！`)
  } catch (err: any) {
    alert(`下载服务端失败: ${err}`)
    store.clearDownloadTask()
  }
}
</script>

<template>
  <div class="flex flex-col gap-3.5 flex-1 overflow-hidden">
    <div class="flex items-center justify-between gap-3 shrink-0 flex-wrap">
      <div class="text-xs text-pcl-subtext whitespace-nowrap shrink-0">
        已检索到 {{ versions.length }} 个版本 · 已安装 {{ store.installedVersions.length }} 个
      </div>

      <div class="flex items-center gap-2 flex-1 max-w-md justify-end min-w-0">
        <div class="flex items-center p-1 rounded-xl bg-black/5 dark:bg-white/5 border border-pcl-border text-xs shrink-0 whitespace-nowrap">
          <button
            class="px-3 py-1 rounded-lg transition-all cursor-pointer whitespace-nowrap shrink-0"
            :class="filterType === 'release' ? 'bg-pcl-card text-pcl-text font-bold shadow-sm' : 'text-pcl-subtext hover:text-pcl-text'"
            @click="filterType = 'release'"
          >
            正式版
          </button>
          <button
            class="px-3 py-1 rounded-lg transition-all cursor-pointer whitespace-nowrap shrink-0"
            :class="filterType === 'snapshot' ? 'bg-pcl-card text-pcl-text font-bold shadow-sm' : 'text-pcl-subtext hover:text-pcl-text'"
            @click="filterType = 'snapshot'"
          >
            快照版
          </button>
          <button
            class="px-3 py-1 rounded-lg transition-all cursor-pointer flex items-center gap-1 whitespace-nowrap shrink-0"
            :class="filterType === 'installed' ? 'bg-pcl-card text-pcl-primary font-bold shadow-sm' : 'text-pcl-subtext hover:text-pcl-text'"
            @click="filterType = 'installed'"
          >
            <span>已安装</span>
            <span v-if="installedCount > 0" class="text-[10px] px-1.5 py-0.2 rounded-full bg-pcl-primary/20 text-pcl-primary">
              {{ installedCount }}
            </span>
          </button>
          <button
            class="px-3 py-1 rounded-lg transition-all cursor-pointer whitespace-nowrap shrink-0"
            :class="filterType === 'all' ? 'bg-pcl-card text-pcl-text font-bold shadow-sm' : 'text-pcl-subtext hover:text-pcl-text'"
            @click="filterType = 'all'"
          >
            全部
          </button>
        </div>

        <input
          v-model="searchQuery"
          placeholder="搜索版本号..."
          class="px-3 py-1.5 rounded-md bg-black/5 dark:bg-white/5 border border-pcl-border text-xs text-pcl-text outline-none focus:border-pcl-primary transition-colors w-36 min-w-[90px]"
        />

        <MyButton type="secondary" size="small" :loading="isLoading" @click="emit('refresh')">
          <PclIcon name="refresh" :size="12" />
        </MyButton>
      </div>
    </div>

    <div class="flex-1 overflow-y-auto pr-1 space-y-2">
      <div v-if="isLoading && versions.length === 0" class="h-64 flex flex-col items-center justify-center text-pcl-subtext text-xs gap-3">
        <PclIcon name="refresh" :size="24" class="animate-spin text-pcl-primary" />
        <span>正在拉取官方版本清单...</span>
      </div>

      <div
        v-for="v in filteredVersions"
        :key="v.id"
        class="flex flex-col p-3 rounded-xl border transition-colors"
        :class="[
          isVersionDownloading(v.id)
            ? 'border-pcl-primary/60 bg-pcl-primary/5'
            : 'border-pcl-border bg-pcl-card shadow-xs'
        ]"
      >
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-3 overflow-hidden">
            <div
              class="w-8 h-8 rounded-lg flex items-center justify-center shrink-0"
              :class="v.type === 'release' ? 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400' : 'bg-black/5 dark:bg-white/10 text-pcl-subtext'"
            >
              <PclIcon name="vanilla" :size="16" />
            </div>
            <div class="truncate">
              <div class="font-semibold text-sm text-pcl-text flex items-center gap-2">
                <span>Minecraft {{ v.id }}</span>
                <span
                  class="text-[10px] px-1.5 py-0.2 rounded font-mono"
                  :class="v.type === 'release' ? 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400' : 'bg-black/5 dark:bg-white/10 text-pcl-subtext'"
                >
                  {{ v.type === 'release' ? 'Release' : 'Snapshot' }}
                </span>
                <span
                  v-if="isInstalled(v.id)"
                  class="text-[10px] bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 px-1.5 py-0.2 rounded font-medium flex items-center gap-1"
                >
                  <PclIcon name="check" :size="10" /> 已安装
                </span>
              </div>
              <div class="text-xs text-pcl-subtext mt-0.5 font-mono">
                {{ formatDate(v.releaseTime) }} · {{ v.sha1.substring(0, 8) }}
              </div>
            </div>
          </div>

          <div class="shrink-0 ml-4 flex items-center gap-2">
            <div class="flex items-center gap-1 mr-1">
              <button
                class="px-2 py-1 rounded-md text-pcl-subtext hover:text-pcl-text hover:bg-black/5 dark:hover:bg-white/10 transition-colors cursor-pointer text-xs"
                title="另存为（将该版本与依赖库下载导出到独立文件夹）"
                @click.stop="handleSaveAs(v)"
              >
                另存为
              </button>
              <button
                class="p-1.5 rounded-md text-pcl-subtext hover:text-pcl-text hover:bg-black/5 dark:hover:bg-white/10 transition-colors cursor-pointer"
                title="更新日志（查看官方版本更新说明）"
                @click.stop="emit('showChangelog', v)"
              >
                <PclIcon name="info" :size="13" />
              </button>
              <button
                class="px-2 py-1 rounded-md text-pcl-subtext hover:text-pcl-text hover:bg-black/5 dark:hover:bg-white/10 transition-colors cursor-pointer text-xs"
                title="下载服务端（一键下载官方 server.jar 并生成开服脚本）"
                @click.stop="handleDownloadServer(v)"
              >
                服务端
              </button>
            </div>

            <div
              v-if="isVersionDownloading(v.id) && store.downloadTask"
              class="flex items-center gap-2 text-xs font-medium text-pcl-primary px-3 py-1.5 rounded-md bg-pcl-primary/10"
            >
              <PclIcon name="refresh" :size="12" class="animate-spin" />
              <span class="font-mono tabular-nums">{{ store.downloadTask.percentage.toFixed(0) }}% {{ store.downloadTask.speed_text }}</span>
            </div>
            <template v-else-if="isInstalled(v.id)">
              <button
                class="px-3 py-1.5 rounded-md text-xs font-medium cursor-pointer bg-emerald-600 hover:bg-emerald-500 text-white shadow-xs transition-colors flex items-center gap-1"
                @click="goToLaunch(v.id)"
              >
                <PclIcon name="launch" :size="12" />
                <span>启动</span>
              </button>
              <button
                :disabled="store.isDownloading"
                class="p-1.5 rounded-md text-xs text-pcl-subtext hover:text-pcl-text hover:bg-black/5 dark:hover:bg-white/10 transition-colors disabled:opacity-50 cursor-pointer"
                title="重新配置并安装此版本"
                @click="emit('configure', v)"
              >
                <PclIcon name="refresh" :size="12" />
              </button>
            </template>
            <template v-else>
              <button
                :disabled="store.isDownloading"
                class="px-3.5 py-1.5 rounded-md text-xs font-medium cursor-pointer bg-pcl-primary hover:bg-pcl-hover text-white shadow-xs transition-colors disabled:opacity-50 flex items-center gap-1.5"
                @click="emit('configure', v)"
              >
                <PclIcon name="download" :size="12" />
                <span>安装</span>
              </button>
            </template>
          </div>
        </div>

        <InlineProgress
          v-if="isVersionDownloading(v.id) && store.downloadTask"
          :percentage="store.downloadTask.percentage"
          :step="store.downloadTask.step"
          :speed-text="store.downloadTask.speed_text"
        />
      </div>
    </div>
  </div>
</template>
