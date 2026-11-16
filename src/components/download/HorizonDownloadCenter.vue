<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useAppStore } from '@/stores/app'
import { backend } from '@/services/backend'
import type { HorizonBuildItem, HorizonCheckResult } from '@/services/backend'
import MyButton from '@/components/base/MyButton.vue'
import MyHint from '@/components/base/MyHint.vue'
import PclIcon from '@/components/base/PclIcon.vue'
import InlineProgress from './InlineProgress.vue'

const store = useAppStore()

const builds = ref<HorizonBuildItem[]>([])
const isLoading = ref(false)
const loadError = ref('')
const checkMap = ref<Record<string, HorizonCheckResult>>({})
const preparingTag = ref<string | null>(null)
const installingTag = ref<string | null>(null)

const creatingForTag = ref<string | null>(null)
const newInstanceName = ref('')
const isCreatingInstance = ref(false)
const openMenuTag = ref<string | null>(null)
const deletingTag = ref<string | null>(null)

const closeMenu = () => {
  openMenuTag.value = null
}

const toggleMenu = (tag: string) => {
  openMenuTag.value = openMenuTag.value === tag ? null : tag
}

const handleKeydown = (e: KeyboardEvent) => {
  if (e.key === 'Escape') closeMenu()
}

const getBoundInstances = (tag: string) => {
  return store.horizonInstances.filter((inst) => inst.build_tag === tag)
}

const isResourceReady = (tag: string) => {
  return !!checkMap.value[tag]?.is_ready
}

const hasPreparedResourcesDir = (tag: string) => {
  const chk = checkMap.value[tag]
  if (!chk) return false
  if (chk.is_ready) return true
  return !(chk.gaps.length === 1 && chk.gaps[0] === '资源尚未准备')
}

const isTaskForTag = (tag: string) => {
  const task = store.downloadTask
  return !!task && task.version_id === `horizon-${tag}`
}

const getResourceStatus = (tag: string) => {
  if (preparingTag.value === tag) {
    return { label: '准备中', cls: 'bg-pcl-primary/15 text-pcl-primary', title: '正在准备资源...' }
  }
  const chk = checkMap.value[tag]
  if (!chk) {
    return { label: '未准备', cls: 'bg-black/5 dark:bg-white/10 text-pcl-subtext', title: '尚未检查或准备资源' }
  }
  if (chk.is_ready) {
    return { label: '已就绪', cls: 'bg-emerald-500/15 text-emerald-600 dark:text-emerald-400', title: '资源完整且可用' }
  }
  if (chk.gaps.length === 1 && chk.gaps[0] === '资源尚未准备') {
    return { label: '未准备', cls: 'bg-black/5 dark:bg-white/10 text-pcl-subtext', title: '尚未准备资源' }
  }
  return {
    label: '有缺口',
    cls: 'bg-amber-500/15 text-amber-600 dark:text-amber-400',
    title: chk.gaps.join('\n') || chk.error || '部分资源缺失',
  }
}

const formatDate = (iso?: string | null) => {
  if (!iso) return '未知时间'
  try {
    const d = new Date(iso)
    if (Number.isNaN(d.getTime())) return iso
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
  } catch {
    return iso
  }
}

const formatSize = (bytes?: number | null) => {
  if (!bytes || bytes <= 0) return ''
  return `${(bytes / 1_048_576).toFixed(1)} MB`
}

const checkInstalledBuild = async (tag: string) => {
  try {
    const res = await backend.checkHorizonResources(tag)
    checkMap.value[tag] = res
  } catch (e: any) {
    checkMap.value[tag] = {
      is_ready: false,
      gaps: [],
      error: String(e),
    }
  }
}

const loadBuilds = async () => {
  isLoading.value = true
  loadError.value = ''
  try {
    const localBuilds = await backend.listLocalHorizonBuilds()
    if (localBuilds.length > 0 && builds.value.length === 0) {
      builds.value = localBuilds
    }
    try {
      const list = await backend.fetchHorizonBuilds()
      builds.value = list
      void store.checkHorizonUpdates()
    } catch (err: any) {
      builds.value = localBuilds
      loadError.value = `拉取远端 Horizon 构建失败（已保留本地列表）: ${err}`
    }
    await Promise.all(
      builds.value.filter((b) => b.is_installed && b.launchable).map((b) => checkInstalledBuild(b.tag))
    )
    await store.refreshInstalledVersions()
  } finally {
    isLoading.value = false
  }
}

const handleInstallBuild = async (tag: string) => {
  if (store.isDownloading) {
    store.showNotification('已有下载或准备任务正在进行，请等待完成后再试')
    return
  }
  installingTag.value = tag
  store.updateDownloadProgress({
    version_id: `horizon-${tag}`,
    step: `正在初始化 Horizon ${tag} 下载...`,
    percentage: 5,
    speed_text: '',
    is_finished: false,
  })
  try {
    await backend.installHorizonBuild(tag)
    store.showNotification(`Horizon ${tag} 安装并验签成功`)
    await loadBuilds()
  } catch (err: any) {
    store.updateDownloadProgress({
      version_id: `horizon-${tag}`,
      step: '安装失败',
      percentage: 0,
      speed_text: '',
      is_finished: true,
      error: String(err),
    })
    alert(`安装 Horizon 构建失败: ${err}`)
  } finally {
    installingTag.value = null
  }
}

const handlePrepareResources = async (tag: string) => {
  if (store.isDownloading) {
    store.showNotification('已有下载或准备任务正在进行，请等待完成后再试')
    return
  }
  preparingTag.value = tag
  store.updateDownloadProgress({
    version_id: `horizon-${tag}`,
    step: `正在初始化资源准备 (${tag})...`,
    percentage: 5,
    speed_text: '',
    is_finished: false,
  })
  try {
    await backend.prepareHorizonResources(tag)
    await checkInstalledBuild(tag)
    store.showNotification(`Horizon ${tag} 资源已就绪`)
  } catch (err: any) {
    store.updateDownloadProgress({
      version_id: `horizon-${tag}`,
      step: '资源准备失败',
      percentage: 0,
      speed_text: '',
      is_finished: true,
      error: String(err),
    })
    alert(`准备资源失败: ${err}`)
    await checkInstalledBuild(tag)
  } finally {
    preparingTag.value = null
  }
}

const handleDeleteBuild = async (tag: string) => {
  closeMenu()
  const bound = getBoundInstances(tag)
  if (bound.length > 0) {
    const names = bound.map((i) => i.name).join('、')
    alert(`构建「${tag}」当前正被实例【${names}】绑定使用。\n\n请先前往「版本/实例」页或启动页删除相关实例后，再移除该构建。`)
    return
  }
  if (!confirm(`确定删除 Horizon 构建「${tag}」及其已准备资源吗？\n\n将移除 builds/${tag} 与 resources/${tag}（不影响原版 versions 目录）。`)) {
    return
  }
  deletingTag.value = tag
  try {
    await backend.deleteHorizonBuild(tag)
    delete checkMap.value[tag]
    await loadBuilds()
    store.showNotification(`已删除 Horizon 构建 ${tag}`)
  } catch (err: any) {
    alert(`删除失败: ${err}`)
  } finally {
    deletingTag.value = null
  }
}

const handleOpenBuildFolder = async (tag: string, kind: 'build' | 'resources') => {
  closeMenu()
  try {
    await backend.openHorizonBuildFolder(tag, kind)
  } catch (err: any) {
    alert(`打开目录失败: ${err}`)
  }
}

const goToLaunchHorizon = (tag: string) => {
  const bound = getBoundInstances(tag)
  if (bound.length === 0) return
  const current = store.currentHorizonInstance
  const preferred = (current && bound.find((i) => i.name === current.name)) || bound[0]
  store.setCurrentHorizonInstance(preferred)
  store.currentTab = 'launch'
}

const openCreateInstance = (tag: string) => {
  creatingForTag.value = tag
  newInstanceName.value = `Horizon-${tag}`
}

const handleCreateInstance = async (tag: string) => {
  const name = newInstanceName.value.trim()
  if (!name) return
  isCreatingInstance.value = true
  try {
    const inst = await backend.createHorizonInstance(name, tag)
    creatingForTag.value = null
    newInstanceName.value = ''
    await store.refreshInstalledVersions()
    store.setCurrentHorizonInstance(inst)
    store.showNotification(`已创建 Horizon 实例「${inst.name}」并设为当前启动目标`)
  } catch (err: any) {
    alert(`创建实例失败: ${err}`)
  } finally {
    isCreatingInstance.value = false
  }
}

onMounted(() => {
  window.addEventListener('click', closeMenu)
  window.addEventListener('keydown', handleKeydown)
  loadBuilds()
})

onUnmounted(() => {
  window.removeEventListener('click', closeMenu)
  window.removeEventListener('keydown', handleKeydown)
})

defineExpose({
  loadBuilds,
})
</script>

<template>
  <div class="flex-1 flex flex-col gap-3 overflow-hidden">
    <div class="flex items-center justify-between gap-3 px-4 py-3 rounded-xl bg-pcl-card border border-pcl-border shadow-pcl-card shrink-0">
      <div class="min-w-0">
        <div class="text-sm font-semibold text-pcl-text flex items-center gap-2">
          <span>Horizon 构建与资源中心</span>
          <span class="text-[11px] font-normal px-2 py-0.5 rounded bg-pcl-primary/10 text-pcl-primary">
            官方 Launch API
          </span>
        </div>
        <p class="text-xs text-pcl-subtext mt-0.5 truncate">
          构建下载均经过 Ed25519 签名与 SHA-256 校验；安装后请先「准备资源」并创建实例
        </p>
      </div>

      <MyButton type="secondary" size="small" :loading="isLoading" @click="loadBuilds">
        <PclIcon name="refresh" :size="12" class="mr-1" />
        刷新构建
      </MyButton>
    </div>

    <div
      v-if="store.horizonUpdateStatus?.has_update && store.horizonUpdateStatus.latest_tag"
      class="px-4 py-2.5 rounded-xl bg-pcl-primary/10 border border-pcl-primary/30 flex items-center justify-between gap-3 shrink-0"
    >
      <div class="text-xs text-pcl-text space-y-0.5 min-w-0">
        <div class="font-semibold text-pcl-primary">
          有新构建：{{ store.horizonUpdateStatus.latest_tag }}
          <span v-if="store.horizonUpdateStatus.latest_published_at" class="font-normal text-pcl-subtext ml-1">
            ({{ formatDate(store.horizonUpdateStatus.latest_published_at) }})
          </span>
        </div>
        <div class="text-pcl-subtext">
          {{ store.horizonUpdateStatus.message }}
        </div>
      </div>
      <MyButton
        type="primary"
        size="small"
        class="shrink-0"
        :disabled="store.isDownloading"
        @click="handleInstallBuild(store.horizonUpdateStatus.latest_tag!)"
      >
        安装新构建
      </MyButton>
    </div>

    <MyHint v-if="loadError" type="error" @close="loadError = ''">
      {{ loadError }}
    </MyHint>

    <div class="flex-1 overflow-y-auto space-y-2.5 pr-1 pb-24">
      <div
        v-if="isLoading && builds.length === 0"
        class="h-44 flex flex-col items-center justify-center text-xs text-pcl-subtext gap-2"
      >
        <PclIcon name="refresh" :size="20" class="animate-spin text-pcl-primary" />
        <span>正在拉取 Horizon 构建列表...</span>
      </div>

      <div
        v-else-if="builds.length === 0"
        class="h-44 flex flex-col items-center justify-center text-xs text-pcl-subtext gap-2"
      >
        <span>暂无可用的 Horizon 构建</span>
      </div>

      <div
        v-for="item in builds"
        :key="item.tag"
        class="relative p-3.5 rounded-xl bg-pcl-card border border-pcl-border shadow-pcl-card space-y-2.5"
        :class="openMenuTag === item.tag ? 'z-30' : 'z-0'"
      >
        <div class="flex items-center justify-between gap-3">
          <div class="min-w-0 space-y-1">
            <div class="flex items-center gap-2 flex-wrap">
              <span class="font-semibold text-sm text-pcl-text font-mono">
                {{ item.display_name || item.tag }}
              </span>
              <span
                v-if="item.display_name && item.display_name !== item.tag"
                class="px-1.5 py-0.5 rounded text-[10px] font-mono bg-pcl-primary/10 text-pcl-primary"
              >
                {{ item.tag }}
              </span>
              <span
                class="px-1.5 py-0.5 rounded text-[10px] font-medium"
                :class="item.is_installed ? 'bg-emerald-500/15 text-emerald-600 dark:text-emerald-400' : 'bg-black/5 dark:bg-white/10 text-pcl-subtext'"
              >
                {{ item.is_installed ? '已安装' : '未安装' }}
              </span>
              <span
                v-if="item.is_installed && !item.launchable"
                class="px-1.5 py-0.5 rounded text-[10px] font-medium bg-red-500/15 text-red-500"
                :title="item.unlaunchable_reason || '构建损坏或 API 不兼容'"
              >
                不可启动
              </span>
              <span
                v-else-if="item.is_installed"
                class="px-1.5 py-0.5 rounded text-[10px] font-medium"
                :class="getResourceStatus(item.tag).cls"
                :title="getResourceStatus(item.tag).title"
              >
                资源：{{ getResourceStatus(item.tag).label }}
              </span>
            </div>

            <div v-if="item.is_installed && !item.launchable && item.unlaunchable_reason" class="text-[11px] text-red-500">
              {{ item.unlaunchable_reason }}
            </div>

            <div class="text-xs text-pcl-subtext flex items-center gap-3 flex-wrap">
              <span>发布日期：{{ formatDate(item.published_at) }}</span>
              <span v-if="formatSize(item.package_size)">包大小：{{ formatSize(item.package_size) }}</span>
              <template v-if="item.installed_meta">
                <span>Horizon {{ item.installed_meta.horizon_version }}</span>
                <span>MC {{ item.installed_meta.minecraft_version }}</span>
                <span>Java {{ item.installed_meta.java_major }}</span>
              </template>
              <span v-if="item.is_installed && item.launchable">
                实例：{{ getBoundInstances(item.tag).length > 0 ? getBoundInstances(item.tag).map((i) => i.name).join('、') : '暂无实例' }}
              </span>
            </div>
          </div>

          <div class="relative flex items-center gap-1.5 shrink-0">
            <template v-if="item.is_installed">
              <!-- 已安装但损坏/不兼容 -->
              <template v-if="!item.launchable">
                <MyButton
                  type="primary"
                  size="small"
                  :loading="installingTag === item.tag"
                  :disabled="store.isDownloading && !isTaskForTag(item.tag)"
                  @click="handleInstallBuild(item.tag)"
                >
                  重新安装
                </MyButton>
              </template>

              <!-- 已安装、可启动、资源未就绪 -->
              <template v-else-if="!isResourceReady(item.tag)">
                <MyButton
                  type="primary"
                  size="small"
                  :loading="preparingTag === item.tag"
                  :disabled="store.isDownloading && !isTaskForTag(item.tag)"
                  @click="handlePrepareResources(item.tag)"
                >
                  准备资源
                </MyButton>
                <MyButton
                  type="secondary"
                  size="small"
                  @click="openCreateInstance(item.tag)"
                >
                  创建实例
                </MyButton>
              </template>

              <!-- 已安装、可启动、资源已就绪、尚无实例 -->
              <template v-else-if="getBoundInstances(item.tag).length === 0">
                <MyButton
                  type="primary"
                  size="small"
                  @click="openCreateInstance(item.tag)"
                >
                  + 创建实例
                </MyButton>
                <MyButton
                  type="secondary"
                  size="small"
                  :loading="preparingTag === item.tag"
                  :disabled="store.isDownloading && !isTaskForTag(item.tag)"
                  @click="handlePrepareResources(item.tag)"
                >
                  重新检查
                </MyButton>
              </template>

              <!-- 已安装、可启动、资源已就绪、已有实例（日常态） -->
              <template v-else>
                <button
                  class="px-3 py-1.5 rounded-md text-xs font-medium cursor-pointer bg-emerald-600 hover:bg-emerald-500 text-white shadow-xs transition-colors flex items-center gap-1"
                  @click="goToLaunchHorizon(item.tag)"
                >
                  <PclIcon name="launch" :size="12" />
                  <span>去启动</span>
                </button>
                <MyButton
                  type="secondary"
                  size="small"
                  @click="openCreateInstance(item.tag)"
                >
                  + 新建实例
                </MyButton>
              </template>

              <MyButton
                type="secondary"
                size="small"
                :loading="deletingTag === item.tag"
                title="更多操作"
                @click.stop="toggleMenu(item.tag)"
              >
                <span>⋯</span>
              </MyButton>

              <div
                v-if="openMenuTag === item.tag"
                class="absolute right-0 top-full mt-1.5 w-48 rounded-lg border border-pcl-border bg-pcl-card shadow-lg py-1 text-xs z-40"
                @click.stop
              >
                <button
                  v-if="item.launchable && isResourceReady(item.tag) && getBoundInstances(item.tag).length > 0"
                  :disabled="store.isDownloading && !isTaskForTag(item.tag)"
                  class="w-full px-3 py-1.5 text-left text-pcl-text hover:bg-black/5 dark:hover:bg-white/10 flex items-center gap-2 cursor-pointer disabled:opacity-50"
                  @click="closeMenu(); handlePrepareResources(item.tag)"
                >
                  <PclIcon name="refresh" :size="12" class="text-pcl-subtext" />
                  <span>重新检查 / 准备资源</span>
                </button>
                <button
                  class="w-full px-3 py-1.5 text-left text-pcl-text hover:bg-black/5 dark:hover:bg-white/10 flex items-center gap-2 cursor-pointer"
                  @click="handleOpenBuildFolder(item.tag, 'build')"
                >
                  <PclIcon name="folder" :size="12" class="text-pcl-subtext" />
                  <span>打开构建目录</span>
                </button>
                <button
                  v-if="hasPreparedResourcesDir(item.tag)"
                  class="w-full px-3 py-1.5 text-left text-pcl-text hover:bg-black/5 dark:hover:bg-white/10 flex items-center gap-2 cursor-pointer"
                  @click="handleOpenBuildFolder(item.tag, 'resources')"
                >
                  <PclIcon name="folder" :size="12" class="text-pcl-subtext" />
                  <span>打开资源目录</span>
                </button>
                <div class="my-1 border-t border-pcl-border" />
                <button
                  class="w-full px-3 py-1.5 text-left text-red-500 hover:bg-red-500/10 flex items-center justify-between gap-2 font-medium cursor-pointer"
                  @click="handleDeleteBuild(item.tag)"
                >
                  <span class="flex items-center gap-2">
                    <PclIcon name="close" :size="12" />
                    <span>删除此构建...</span>
                  </span>
                  <span
                    v-if="getBoundInstances(item.tag).length > 0"
                    class="text-[10px] font-normal text-pcl-subtext"
                  >
                    需先解绑
                  </span>
                </button>
              </div>
            </template>

            <MyButton
              v-else
              type="primary"
              size="small"
              :loading="installingTag === item.tag"
              :disabled="store.isDownloading && !isTaskForTag(item.tag)"
              @click="handleInstallBuild(item.tag)"
            >
              <PclIcon name="download" :size="12" />
              <span>安装构建</span>
            </MyButton>
          </div>
        </div>

        <div
          v-if="creatingForTag === item.tag"
          class="pt-2 border-t border-pcl-border flex items-center gap-2"
        >
          <input
            v-model="newInstanceName"
            placeholder="输入实例名称..."
            class="flex-1 h-8 px-2.5 rounded-md text-xs text-pcl-text bg-black/5 dark:bg-black/30 border border-pcl-border focus:border-pcl-primary outline-none"
            @keyup.enter="handleCreateInstance(item.tag)"
          />
          <MyButton
            type="primary"
            size="small"
            :loading="isCreatingInstance"
            @click="handleCreateInstance(item.tag)"
          >
            确定创建
          </MyButton>
          <MyButton
            type="secondary"
            size="small"
            @click="creatingForTag = null"
          >
            取消
          </MyButton>
        </div>

        <InlineProgress
          v-if="isTaskForTag(item.tag) && store.downloadTask"
          :percentage="store.downloadTask.percentage"
          :step="store.downloadTask.step"
          :speed-text="store.downloadTask.speed_text"
        />
      </div>
    </div>
  </div>
</template>
