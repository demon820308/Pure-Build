<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
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
const openMenuId = ref<string | null>(null)
const deletingVersionId = ref<string | null>(null)

const closeMenu = () => {
  openMenuId.value = null
}

const toggleMenu = (vId: string) => {
  openMenuId.value = openMenuId.value === vId ? null : vId
}

const handleKeydown = (e: KeyboardEvent) => {
  if (e.key === 'Escape') closeMenu()
}

onMounted(() => {
  window.addEventListener('click', closeMenu)
  window.addEventListener('keydown', handleKeydown)
})

onUnmounted(() => {
  window.removeEventListener('click', closeMenu)
  window.removeEventListener('keydown', handleKeydown)
})

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

const getInstalledObj = (versionId: string) => {
  return (
    store.installedVersions.find((iv) => iv.id === versionId || iv.name === versionId) ||
    store.installedVersions.find(
      (iv) => iv.id.startsWith(`${versionId}-`) || iv.id.startsWith(`${versionId} `)
    )
  )
}

const isInstalled = (versionId: string) => {
  return !!getInstalledObj(versionId)
}

const installedCount = computed(() => {
  return props.versions.filter((v) => isInstalled(v.id)).length
})

const goToLaunch = (versionId: string) => {
  const target = getInstalledObj(versionId)
  if (target) {
    store.setCurrentVersion(target)
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

const handleOpenVersionFolder = async (versionId: string) => {
  closeMenu()
  const target = getInstalledObj(versionId)
  if (!target) return
  try {
    await backend.openFolder(target.path)
  } catch (err: any) {
    alert(`打开版本目录失败: ${err}`)
  }
}

const handleDeleteVersion = async (v: RemoteVersion) => {
  closeMenu()
  const target = getInstalledObj(v.id)
  if (!target) return
  if (store.runningPid && store.currentVersion?.id === target.id) {
    alert('当前版本正在运行中，请先退出游戏后再删除。')
    return
  }
  const indieTip = target.has_indie
    ? '\n注意：该版本已启用独立版本隔离，目录内的存档与配置也会一并清除。'
    : ''
  if (
    !confirm(
      `确定删除本地版本「${target.id}」吗？\n\n将仅删除 versions 目录下的该版本文件夹（不会影响公共资源库与 Horizon 已准备资源）。${indieTip}`
    )
  ) {
    return
  }

  deletingVersionId.value = v.id
  try {
    await backend.deleteMinecraftVersion(store.gameDir, target.id)
    await store.refreshInstalledVersions()
    store.showNotification(`已删除版本 ${target.id}`)
  } catch (err: any) {
    alert(`删除版本失败: ${err}`)
  } finally {
    deletingVersionId.value = null
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

    <div class="flex-1 overflow-y-auto pr-1 pb-24 space-y-2">
      <div v-if="isLoading && versions.length === 0" class="h-64 flex flex-col items-center justify-center text-pcl-subtext text-xs gap-3">
        <PclIcon name="refresh" :size="24" class="animate-spin text-pcl-primary" />
        <span>正在拉取官方版本清单...</span>
      </div>

      <div
        v-for="v in filteredVersions"
        :key="v.id"
        class="relative flex flex-col p-3 rounded-xl border transition-colors"
        :class="[
          openMenuId === v.id ? 'z-30' : 'z-0',
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

          <div class="shrink-0 ml-4 flex items-center gap-1.5 relative">
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
              <MyButton
                type="secondary"
                size="small"
                :disabled="store.isDownloading || deletingVersionId === v.id"
                title="重新配置、修复文件或加装模组加载器"
                @click="emit('configure', v)"
              >
                <PclIcon name="gear" :size="12" />
                <span>配置</span>
              </MyButton>
            </template>
            <template v-else>
              <MyButton
                type="primary"
                size="small"
                :disabled="store.isDownloading"
                @click="emit('configure', v)"
              >
                <PclIcon name="download" :size="12" />
                <span>安装</span>
              </MyButton>
              <MyButton
                type="secondary"
                size="small"
                title="查看官方版本更新日志"
                @click="emit('showChangelog', v)"
              >
                <span>更新日志</span>
              </MyButton>
            </template>

            <MyButton
              v-if="!isVersionDownloading(v.id)"
              type="secondary"
              size="small"
              :loading="deletingVersionId === v.id"
              title="更多操作"
              @click.stop="toggleMenu(v.id)"
            >
              <span>⋯</span>
            </MyButton>

            <div
              v-if="openMenuId === v.id"
              class="absolute right-0 top-full mt-1.5 w-44 rounded-lg border border-pcl-border bg-pcl-card shadow-lg py-1 text-xs z-40"
              @click.stop
            >
              <button
                v-if="isInstalled(v.id)"
                class="w-full px-3 py-1.5 text-left text-pcl-text hover:bg-black/5 dark:hover:bg-white/10 flex items-center gap-2 cursor-pointer"
                @click="closeMenu(); emit('showChangelog', v)"
              >
                <PclIcon name="info" :size="12" class="text-pcl-subtext" />
                <span>更新日志</span>
              </button>
              <button
                v-if="isInstalled(v.id)"
                class="w-full px-3 py-1.5 text-left text-pcl-text hover:bg-black/5 dark:hover:bg-white/10 flex items-center gap-2 cursor-pointer"
                @click="handleOpenVersionFolder(v.id)"
              >
                <PclIcon name="folder" :size="12" class="text-pcl-subtext" />
                <span>打开版本目录</span>
              </button>
              <button
                class="w-full px-3 py-1.5 text-left text-pcl-text hover:bg-black/5 dark:hover:bg-white/10 flex items-center gap-2 cursor-pointer"
                @click="closeMenu(); handleSaveAs(v)"
              >
                <PclIcon name="download" :size="12" class="text-pcl-subtext" />
                <span>另存为独立版本...</span>
              </button>
              <button
                class="w-full px-3 py-1.5 text-left text-pcl-text hover:bg-black/5 dark:hover:bg-white/10 flex items-center gap-2 cursor-pointer"
                @click="closeMenu(); handleDownloadServer(v)"
              >
                <PclIcon name="instance" :size="12" class="text-pcl-subtext" />
                <span>下载服务端...</span>
              </button>
              <template v-if="isInstalled(v.id)">
                <div class="my-1 border-t border-pcl-border" />
                <button
                  class="w-full px-3 py-1.5 text-left text-red-500 hover:bg-red-500/10 flex items-center gap-2 font-medium cursor-pointer"
                  @click="handleDeleteVersion(v)"
                >
                  <PclIcon name="close" :size="12" />
                  <span>删除此版本...</span>
                </button>
              </template>
            </div>
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
