<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { useAppStore } from '@/stores/app'
import { backend } from '@/services/backend'
import type { HorizonModItem } from '@/services/backend'
import MyButton from '@/components/base/MyButton.vue'
import MyHint from '@/components/base/MyHint.vue'
import PclIcon from '@/components/base/PclIcon.vue'
import InlineProgress from './InlineProgress.vue'

type HorizonContentKind = 'mod' | 'shader' | 'resourcepack' | 'native-mod'

const props = defineProps<{
  kind: HorizonContentKind
}>()

const emit = defineEmits<{
  (e: 'go-builds'): void
}>()

const store = useAppStore()

const searchQuery = ref('')
const compatibleOnly = ref(true)
const selectedInstanceName = ref('')
const items = ref<HorizonModItem[]>([])
const isLoading = ref(false)
const loadError = ref('')
const installingId = ref<string | null>(null)
const brokenIcons = ref<Record<string, boolean>>({})

const kindTitleMap: Record<HorizonContentKind, string> = {
  mod: '内容模组 (.hmod)',
  shader: '原生光影 (.hshader)',
  resourcepack: '材质包 (.zip)',
  'native-mod': '原生重编类 (.hnmod)',
}

const activeInstance = computed(() =>
  store.horizonInstances.find((i) => i.name === selectedInstanceName.value) || null
)

const activeBuildTag = computed(() => {
  if (activeInstance.value) return activeInstance.value.build_tag
  const launchable = store.horizonBuilds.find((b) => b.is_installed && b.launchable)
  return launchable?.tag || ''
})

const syncInstanceSelection = () => {
  if (
    !selectedInstanceName.value ||
    !store.horizonInstances.some((i) => i.name === selectedInstanceName.value)
  ) {
    selectedInstanceName.value =
      store.currentHorizonInstance?.name ||
      store.horizonInstances[0]?.name ||
      ''
  }
}

const loadRegistry = async (forceRefresh = false) => {
  syncInstanceSelection()
  isLoading.value = true
  loadError.value = ''
  try {
    items.value = await backend.fetchHorizonModRegistry({
      buildTag: activeBuildTag.value || undefined,
      instanceName: selectedInstanceName.value || undefined,
      kindFilter: props.kind,
      query: searchQuery.value,
      compatibleOnly: props.kind === 'native-mod' ? false : compatibleOnly.value,
      forceRefresh,
    })
  } catch (err: any) {
    loadError.value = String(err)
  } finally {
    isLoading.value = false
  }
}

let searchTimer: ReturnType<typeof setTimeout> | null = null

watch([() => props.kind, compatibleOnly, selectedInstanceName, activeBuildTag], () => {
  loadRegistry(false)
})

watch(searchQuery, () => {
  if (searchTimer) clearTimeout(searchTimer)
  searchTimer = setTimeout(() => {
    loadRegistry(false)
  }, 200)
})

watch(
  () => store.horizonInstances,
  () => {
    syncInstanceSelection()
  }
)

onMounted(() => {
  loadRegistry(false)
})

onUnmounted(() => {
  if (searchTimer) clearTimeout(searchTimer)
})

const kindBadge = (kind: string) => {
  switch (kind) {
    case 'shader':
      return { text: '光影', cls: 'bg-amber-500/15 text-amber-600 dark:text-amber-400' }
    case 'mod':
      return { text: '内容模组', cls: 'bg-pcl-primary/15 text-pcl-primary' }
    case 'resourcepack':
      return { text: '材质包', cls: 'bg-emerald-500/15 text-emerald-600 dark:text-emerald-400' }
    case 'native-mod':
      return { text: '原生重编', cls: 'bg-red-500/15 text-red-500' }
    default:
      return { text: kind, cls: 'bg-black/5 dark:bg-white/10 text-pcl-subtext' }
  }
}

const formatDownloads = (num: number) => {
  if (num >= 1_000_000) return `${(num / 1_000_000).toFixed(1)}M`
  if (num >= 10_000) return `${(num / 10_000).toFixed(1)}万`
  return num.toString()
}

const isItemDownloading = (id: string) => {
  return store.downloadTask?.version_id === `horizon-ext-${id}`
}

const handleInstall = async (item: HorizonModItem) => {
  if (!selectedInstanceName.value) {
    alert('请先在「构建与资源」中安装 Horizon 构建并创建至少一个实例。')
    emit('go-builds')
    return
  }
  const ver = item.latest_version
  if (!ver || !item.installable) {
    alert(item.compatibility_reason || '该条目当前不可直接安装。')
    return
  }
  if (store.isDownloading) {
    store.showNotification('已有下载或准备任务正在进行，请等待完成后再试')
    return
  }

  installingId.value = item.id
  store.updateDownloadProgress({
    version_id: `horizon-ext-${item.id}`,
    step: `正在下载 ${item.name}...`,
    percentage: 10,
    speed_text: '',
    is_finished: false,
  })

  try {
    await backend.installHorizonRegistryItem({
      instanceName: selectedInstanceName.value,
      projectId: item.id,
      kind: item.kind,
      fileName: ver.file.name,
      downloadUrl: ver.file.url,
      expectedSha256: ver.file.sha256,
    })
    store.showNotification(`已将「${item.name}」安装到实例 ${selectedInstanceName.value}`)
    await loadRegistry(false)
  } catch (err: any) {
    alert(`安装失败: ${err}`)
    store.clearDownloadTask()
  } finally {
    installingId.value = null
  }
}

const openInstanceSubdir = async (folder?: string) => {
  if (!selectedInstanceName.value) return
  try {
    await backend.openHorizonInstanceFolder(selectedInstanceName.value, folder)
  } catch (err: any) {
    alert(`打开目录失败: ${err}`)
  }
}

defineExpose({
  loadContentCaps: () => loadRegistry(true),
})
</script>

<template>
  <div class="flex-1 flex flex-col gap-3 overflow-hidden">
    <!-- Search + compatibility filter + quick instance/folder bar -->
    <div class="flex items-center justify-between gap-2.5 shrink-0 flex-wrap">
      <div class="flex items-center gap-2 min-w-0 flex-wrap">
        <span class="text-xs font-semibold text-pcl-text">{{ kindTitleMap[props.kind] }}</span>
        <select
          v-if="store.horizonInstances.length > 0"
          v-model="selectedInstanceName"
          class="px-2 py-1 rounded-md text-xs bg-black/5 dark:bg-white/10 border border-pcl-border text-pcl-text outline-none"
          title="选择要安装扩展的 Horizon 实例"
        >
          <option v-for="inst in store.horizonInstances" :key="inst.name" :value="inst.name">
            目标实例：{{ inst.name }} ({{ inst.build_tag }})
          </option>
        </select>
        <button
          v-else
          class="text-xs text-pcl-primary hover:underline cursor-pointer"
          @click="emit('go-builds')"
        >
          暂无 Horizon 实例，点此创建 →
        </button>

        <MyButton
          v-if="store.horizonInstances.length > 0 && props.kind !== 'native-mod'"
          type="secondary"
          size="small"
          @click="openInstanceSubdir(props.kind === 'shader' ? 'shaderpacks' : props.kind === 'resourcepack' ? 'resourcepacks' : 'horizon-mods')"
          title="打开对应实例扩展目录"
        >
          <PclIcon name="folder" :size="11" />
          <span>打开目录</span>
        </MyButton>
      </div>

      <div class="flex items-center gap-2 shrink-0 flex-wrap">
        <label
          v-if="props.kind !== 'native-mod' && activeBuildTag"
          class="flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-black/5 dark:bg-white/5 border border-pcl-border text-xs text-pcl-text cursor-pointer select-none whitespace-nowrap"
          title="按当前 Horizon 构建协商出的 --launch-api 能力与游戏版本过滤"
        >
          <input v-model="compatibleOnly" type="checkbox" class="accent-[var(--pcl-color-primary)] cursor-pointer" />
          <span>仅显示兼容 ({{ activeBuildTag }})</span>
        </label>

        <input
          v-model="searchQuery"
          placeholder="搜索名称/作者/分类..."
          class="px-3 py-1 rounded-md bg-black/5 dark:bg-white/5 border border-pcl-border text-xs text-pcl-text outline-none focus:border-pcl-primary w-40"
          @keyup.enter="loadRegistry(false)"
        />
        <MyButton type="secondary" size="small" :loading="isLoading" @click="loadRegistry(true)" title="强制刷新官方索引">
          <PclIcon name="refresh" :size="12" />
        </MyButton>
      </div>
    </div>

    <MyHint v-if="props.kind === 'native-mod'" type="warning" :closable="false">
      原生重编类（.hnmod）需本地 Rust 源码重编，预编译构建暂不支持直接加载。
    </MyHint>

    <MyHint v-if="loadError" type="error" @close="loadError = ''">
      {{ loadError }}
    </MyHint>

    <!-- Registry list -->
    <div class="flex-1 overflow-y-auto space-y-2 pr-1 pb-4">
      <div
        v-if="isLoading && items.length === 0"
        class="h-48 flex flex-col items-center justify-center text-xs text-pcl-subtext gap-2"
      >
        <PclIcon name="refresh" :size="20" class="animate-spin text-pcl-primary" />
        <span>正在拉取 Horizon 官方扩展索引...</span>
      </div>

      <div
        v-else-if="items.length === 0"
        class="h-48 flex flex-col items-center justify-center text-xs text-pcl-subtext gap-1.5"
      >
        <span>当前筛选条件下暂无可用条目</span>
      </div>

      <div
        v-for="item in items"
        :key="item.id"
        class="p-3 rounded-xl bg-pcl-card border border-pcl-border shadow-xs space-y-2"
      >
        <div class="flex items-center justify-between gap-3">
          <div class="flex items-center gap-3 min-w-0">
            <img
              v-if="item.icon && !brokenIcons[item.id]"
              :src="item.icon"
              alt=""
              class="w-10 h-10 rounded-lg object-cover bg-black/5 dark:bg-white/5 shrink-0"
              @error="brokenIcons[item.id] = true"
            />
            <div
              v-else
              class="w-10 h-10 rounded-lg bg-pcl-primary/10 text-pcl-primary flex items-center justify-center shrink-0"
            >
              <PclIcon :name="item.kind === 'shader' ? 'shader' : item.kind === 'resourcepack' ? 'resourcepack' : 'mod'" :size="18" />
            </div>

            <div class="min-w-0 space-y-0.5">
              <div class="flex items-center gap-1.5 flex-wrap">
                <span class="font-semibold text-sm text-pcl-text truncate">{{ item.name }}</span>
                <span class="text-[10px] px-1.5 py-0.5 rounded font-medium" :class="kindBadge(item.kind).cls">
                  {{ kindBadge(item.kind).text }}
                </span>
                <span
                  v-if="item.is_installed"
                  class="text-[10px] px-1.5 py-0.5 rounded font-medium bg-emerald-500/15 text-emerald-600 dark:text-emerald-400 flex items-center gap-0.5"
                >
                  <PclIcon name="check" :size="10" /> 已安装
                </span>
                <span
                  v-if="item.latest_version"
                  class="text-[10px] px-1.5 py-0.5 rounded font-mono bg-black/5 dark:bg-white/10 text-pcl-subtext"
                >
                  v{{ item.latest_version.version }} · MC {{ item.latest_version.game || '?' }}
                </span>
              </div>

              <div class="text-xs text-pcl-subtext truncate">
                {{ item.summary || '暂无简介' }}
              </div>

              <div class="text-[11px] text-pcl-subtext flex items-center gap-3 flex-wrap">
                <span v-if="item.authors.length > 0">作者：{{ item.authors.join(', ') }}</span>
                <span>下载：{{ formatDownloads(item.downloads) }}</span>
                <span v-if="item.latest_version?.api" class="font-mono">API: {{ item.latest_version.api }}</span>
              </div>
            </div>
          </div>

          <div class="shrink-0 flex items-center gap-2">
            <span
              v-if="item.kind === 'native-mod'"
              class="px-2.5 py-1 rounded-md text-xs font-medium bg-black/5 dark:bg-white/10 text-pcl-subtext"
              title="需本地 Rust 源码重编，预编译构建暂不支持直接加载"
            >
              需源码重编
            </span>

            <span
              v-else-if="!item.compatible"
              class="px-2.5 py-1 rounded-md text-xs font-medium bg-amber-500/15 text-amber-600 dark:text-amber-400"
              :title="item.compatibility_reason || '与当前构建不兼容'"
            >
              不兼容
            </span>

            <MyButton
              v-else
              :type="item.is_installed ? 'secondary' : 'primary'"
              size="small"
              :loading="installingId === item.id || isItemDownloading(item.id)"
              :disabled="!item.installable || (store.isDownloading && !isItemDownloading(item.id))"
              @click="handleInstall(item)"
            >
              <PclIcon :name="item.is_installed ? 'refresh' : 'download'" :size="12" />
              <span>{{ item.is_installed ? '重新安装' : '安装到实例' }}</span>
            </MyButton>
          </div>
        </div>

        <InlineProgress
          v-if="isItemDownloading(item.id) && store.downloadTask"
          :percentage="store.downloadTask.percentage"
          :step="store.downloadTask.step"
          :speed-text="store.downloadTask.speed_text"
        />
      </div>
    </div>
  </div>
</template>
