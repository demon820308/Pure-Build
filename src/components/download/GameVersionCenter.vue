<script setup lang="ts">
import { ref, computed } from 'vue'
import { useAppStore } from '@/stores/app'
import { backend } from '@/services/backend'
import type { RemoteVersion } from '@/services/backend'
import MyButton from '@/components/base/MyButton.vue'
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
          placeholder="搜索版本号 (如 1.21.4)..."
          class="px-3.5 py-1.5 rounded-xl bg-black/5 dark:bg-white/5 border border-pcl-border text-xs text-pcl-text outline-none focus:border-pcl-primary transition-colors w-36 min-w-[90px]"
        />

        <MyButton type="secondary" size="small" :loading="isLoading" @click="emit('refresh')">
          🔄
        </MyButton>
      </div>
    </div>

    <div class="flex-1 overflow-y-auto pr-1 space-y-2">
      <div v-if="isLoading && versions.length === 0" class="h-64 flex flex-col items-center justify-center text-pcl-subtext text-xs gap-3">
        <span class="animate-spin text-2xl">⏳</span>
        <span>正在从镜像源拉取官方全量版本清单...</span>
      </div>

      <div
        v-for="v in filteredVersions"
        :key="v.id"
        class="flex flex-col p-3.5 rounded-2xl border transition-all duration-200 hover:shadow-md hover:-translate-y-0.5"
        :class="[
          isVersionDownloading(v.id)
            ? 'border-pcl-primary bg-pcl-primary/5 ring-2 ring-pcl-primary/20'
            : 'border-pcl-border bg-pcl-card shadow-sm'
        ]"
      >
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-3.5 overflow-hidden">
            <div
              class="w-10 h-10 rounded-xl flex items-center justify-center text-xl shrink-0"
              :class="v.type === 'release' ? 'bg-emerald-500/10 text-emerald-600' : 'bg-amber-500/10 text-amber-600'"
            >
              {{ v.type === 'release' ? '🌱' : '⚡' }}
            </div>
            <div class="truncate">
              <div class="font-semibold text-sm text-pcl-text flex items-center gap-2">
                <span>Minecraft {{ v.id }}</span>
                <span
                  class="text-[10px] px-2 py-0.2 rounded font-normal"
                  :class="v.type === 'release' ? 'bg-emerald-500/10 text-emerald-600' : 'bg-amber-500/10 text-amber-600'"
                >
                  {{ v.type === 'release' ? '正式版' : '快照/测试' }}
                </span>
                <span
                  v-if="isInstalled(v.id)"
                  class="text-[10px] bg-emerald-500/15 text-emerald-600 px-2 py-0.2 rounded font-semibold flex items-center gap-1"
                >
                  <span>✓</span> 已安装
                </span>
              </div>
              <div class="text-xs text-pcl-subtext mt-0.5">
                发布日期：{{ formatDate(v.releaseTime) }} · SHA1: {{ v.sha1.substring(0, 8) }}...
              </div>
            </div>
          </div>

          <div class="shrink-0 ml-4 flex items-center gap-2">
            <div class="flex items-center gap-1 mr-1">
              <button
                class="p-2 rounded-xl text-pcl-subtext hover:text-pcl-text hover:bg-black/5 dark:hover:bg-white/10 transition-colors cursor-pointer text-sm"
                title="另存为（将该版本与依赖库下载导出到独立文件夹）"
                @click.stop="handleSaveAs(v)"
              >
                💾
              </button>
              <button
                class="p-2 rounded-xl text-pcl-subtext hover:text-pcl-text hover:bg-black/5 dark:hover:bg-white/10 transition-colors cursor-pointer text-sm"
                title="更新日志（查看官方版本更新说明）"
                @click.stop="emit('showChangelog', v)"
              >
                ℹ️
              </button>
              <button
                class="p-2 rounded-xl text-pcl-subtext hover:text-pcl-text hover:bg-black/5 dark:hover:bg-white/10 transition-colors cursor-pointer text-sm"
                title="下载服务端（一键下载官方 server.jar 并生成开服脚本）"
                @click.stop="handleDownloadServer(v)"
              >
                🗄️
              </button>
            </div>

            <div
              v-if="isVersionDownloading(v.id) && store.downloadTask"
              class="flex items-center gap-2 text-xs font-bold text-pcl-primary px-3 py-1.5 rounded-xl bg-pcl-primary/10 animate-pulse"
            >
              <span class="animate-spin text-sm">⏳</span>
              <span>{{ store.downloadTask.percentage.toFixed(0) }}% {{ store.downloadTask.speed_text }}</span>
            </div>
            <template v-else-if="isInstalled(v.id)">
              <button
                class="px-3.5 py-1.5 rounded-xl text-xs font-semibold cursor-pointer bg-emerald-500 hover:bg-emerald-600 text-white shadow-sm transition-all duration-200 active:scale-95 flex items-center gap-1"
                @click="goToLaunch(v.id)"
              >
                <span>🚀</span>
                <span>前往启动</span>
              </button>
              <button
                :disabled="store.isDownloading"
                class="px-2.5 py-1.5 rounded-xl text-xs text-pcl-subtext hover:text-pcl-text hover:bg-black/5 dark:hover:bg-white/10 transition-colors disabled:opacity-50 cursor-pointer"
                title="重新配置并安装此版本"
                @click="emit('configure', v)"
              >
                🔄
              </button>
            </template>
            <template v-else>
              <button
                :disabled="store.isDownloading"
                class="px-4 py-2 rounded-xl text-xs font-semibold cursor-pointer bg-pcl-primary hover:bg-pcl-hover active:bg-pcl-active text-white shadow-sm transition-all duration-200 ease-[var(--pcl-ease-bounce)] active:scale-95 disabled:opacity-50"
                @click="emit('configure', v)"
              >
                📥 安装
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
