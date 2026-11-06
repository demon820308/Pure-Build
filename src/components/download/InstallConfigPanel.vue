<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useAppStore } from '@/stores/app'
import { backend } from '@/services/backend'
import type { RemoteVersion } from '@/services/backend'
import LoaderCard, { type LoaderItem } from './LoaderCard.vue'
import PclIcon from '@/components/base/PclIcon.vue'

const props = defineProps<{
  version: RemoteVersion
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'startInstall', data: {
    targetName: string
    loaderType?: 'fabric' | 'forge' | 'neoforge' | 'optifine'
    loaderVersion?: string
  }): void
}>()

const store = useAppStore()

const targetVersionName = ref(props.version.id)
const customNameEdited = ref(false)

const selectedLoaderType = ref<'fabric' | 'forge' | 'neoforge' | 'optifine' | null>(null)
const selectedLoaderVer = ref('')

const openCard = ref<'forge' | 'neoforge' | 'fabric' | 'optifine' | null>(null)

const fabricMeta = ref<{ supported_games: string[]; loaders: LoaderItem[] } | null>(null)
const optifineGames = ref<string[]>([])
const forgeLoaders = ref<LoaderItem[]>([])
const neoForgeLoaders = ref<LoaderItem[]>([])
const optifineLoaders = ref<LoaderItem[]>([])

const isLoadingFabric = ref(false)
const isLoadingForge = ref(false)
const isLoadingNeoForge = ref(false)
const isLoadingOptifine = ref(false)

const isFabricSupported = computed(() => {
  if (!props.version || !fabricMeta.value) return true
  return fabricMeta.value.supported_games.includes(props.version.id)
})

const isOptiFineSupported = computed(() => {
  if (!props.version) return false
  return optifineLoaders.value.length > 0 || optifineGames.value.includes(props.version.id)
})

const updateTargetVersionName = () => {
  if (!props.version || customNameEdited.value) return
  let name = props.version.id
  if (selectedLoaderType.value === 'fabric' && selectedLoaderVer.value) {
    const cleanVer = selectedLoaderVer.value.split('+')[0]
    name += `-Fabric ${cleanVer}`
  } else if (selectedLoaderType.value === 'forge' && selectedLoaderVer.value) {
    name += `-Forge_${selectedLoaderVer.value}`
  } else if (selectedLoaderType.value === 'neoforge' && selectedLoaderVer.value) {
    const cleanVer = selectedLoaderVer.value.split('+')[0]
    name += `-NeoForge_${cleanVer}`
  } else if (selectedLoaderType.value === 'optifine' && selectedLoaderVer.value) {
    name += `-OptiFine_${selectedLoaderVer.value}`
  }
  targetVersionName.value = name
}

const selectLoader = (type: 'fabric' | 'forge' | 'neoforge' | 'optifine', ver: string) => {
  if (selectedLoaderType.value === type && selectedLoaderVer.value === ver) {
    selectedLoaderType.value = null
    selectedLoaderVer.value = ''
  } else {
    selectedLoaderType.value = type
    selectedLoaderVer.value = ver
  }
  updateTargetVersionName()
}

const clearLoader = () => {
  selectedLoaderType.value = null
  selectedLoaderVer.value = ''
  updateTargetVersionName()
}

const isNameConflict = computed(() => {
  if (!targetVersionName.value.trim()) return false
  return store.installedVersions.some(
    (iv) => iv.id.toLowerCase() === targetVersionName.value.trim().toLowerCase()
  )
})

const loadLoaderData = () => {
  const vId = props.version.id

  if (!fabricMeta.value) {
    isLoadingFabric.value = true
    backend.fetchFabricMeta()
      .then((meta) => {
        fabricMeta.value = meta
      })
      .catch((err) => {
        console.error('拉取 Fabric 元数据异常:', err)
      })
      .finally(() => {
        isLoadingFabric.value = false
      })
  }

  if (optifineGames.value.length === 0) {
    backend.fetchOptifineGames()
      .then((games) => {
        optifineGames.value = games
      })
      .catch(() => {})
  }

  isLoadingOptifine.value = true
  backend.fetchOptifineLoaders(vId)
    .then((loaders) => {
      optifineLoaders.value = loaders
    })
    .catch(() => {
      optifineLoaders.value = []
    })
    .finally(() => {
      isLoadingOptifine.value = false
    })

  isLoadingForge.value = true
  backend.fetchForgeLoaders(vId)
    .then((loaders) => {
      forgeLoaders.value = loaders
    })
    .catch(() => {
      forgeLoaders.value = []
    })
    .finally(() => {
      isLoadingForge.value = false
    })

  isLoadingNeoForge.value = true
  backend.fetchNeoForgeLoaders(vId)
    .then((loaders) => {
      neoForgeLoaders.value = loaders
    })
    .catch(() => {
      neoForgeLoaders.value = []
    })
    .finally(() => {
      isLoadingNeoForge.value = false
    })
}

onMounted(() => {
  loadLoaderData()
})

const loaderConfigs = computed(() => [
  {
    type: 'forge' as const,
    title: 'Forge',
    supported: true,
    statusText: forgeLoaders.value.length > 0 ? '可以添加' : '无',
    selectLabel: '选择 Forge 版本（与 Fabric 互斥）：',
    loadingText: '正在查询可用的 Forge 版本...',
    emptyText: '暂未找到匹配该 Minecraft 版本的 Forge',
    itemIcon: '',
    items: forgeLoaders.value,
    loading: isLoadingForge.value,
  },
  {
    type: 'neoforge' as const,
    title: 'NeoForge',
    supported: true,
    statusText: neoForgeLoaders.value.length > 0 ? '可以添加' : '无',
    selectLabel: '选择 NeoForge 版本：',
    loadingText: '正在查询可用的 NeoForge 版本...',
    emptyText: '暂未找到匹配该 Minecraft 版本的 NeoForge',
    itemIcon: '',
    items: neoForgeLoaders.value,
    loading: isLoadingNeoForge.value,
  },
  {
    type: 'fabric' as const,
    title: 'Fabric',
    supported: isFabricSupported.value,
    statusText: isFabricSupported.value ? '可以添加' : '无',
    selectLabel: '选择 Fabric Loader 版本：',
    loadingText: '正在加载 Fabric 加载器元数据...',
    emptyText: '当前 Minecraft 版本暂不受 Fabric 支持',
    itemIcon: '',
    items: fabricMeta.value?.loaders || [],
    loading: isLoadingFabric.value && !fabricMeta.value,
  },
  {
    type: 'optifine' as const,
    title: 'OptiFine',
    supported: isOptiFineSupported.value,
    statusText: isOptiFineSupported.value ? `可以添加 (${optifineLoaders.value.length})` : '无',
    selectLabel: '选择 OptiFine 高清修复版本：',
    loadingText: '正在加载 OptiFine 版本清单...',
    emptyText: '当前 Minecraft 版本暂无可用 OptiFine',
    itemIcon: '',
    items: optifineLoaders.value,
    loading: isLoadingOptifine.value,
  },
])

const handleStart = () => {
  if (isNameConflict.value || store.isDownloading) return
  emit('startInstall', {
    targetName: targetVersionName.value.trim() || props.version.id,
    loaderType: selectedLoaderType.value || undefined,
    loaderVersion: selectedLoaderVer.value || undefined,
  })
}
</script>

<template>
  <div class="flex flex-col gap-4 max-w-2xl mx-auto w-full py-4 flex-1 overflow-y-auto">
    <div class="flex items-center gap-3 p-3 rounded-xl bg-black/5 dark:bg-white/5 border border-pcl-border">
      <button
        class="p-1.5 rounded-md hover:bg-black/10 dark:hover:bg-white/10 text-pcl-subtext hover:text-pcl-text transition-colors cursor-pointer"
        title="返回版本列表"
        @click="emit('close')"
      >
        <PclIcon name="back" :size="14" />
      </button>
      <div class="w-8 h-8 rounded-md bg-emerald-600/10 border border-emerald-500/20 text-emerald-600 dark:text-emerald-400 flex items-center justify-center shrink-0 select-none">
        <PclIcon name="vanilla" :size="16" />
      </div>
      <div class="flex-1 flex flex-col">
        <input
          v-model="targetVersionName"
          type="text"
          class="w-full px-3 py-1.5 rounded-md bg-pcl-card border border-pcl-border text-xs font-semibold text-pcl-text focus:outline-none focus:border-pcl-primary transition-colors font-mono"
          placeholder="请输入安装后的版本名称"
          @input="customNameEdited = true"
        />
        <span v-if="isNameConflict" class="text-xs text-rose-500 mt-1 font-medium flex items-center gap-1">
          <PclIcon name="warning" :size="12" />
          版本名称已存在，请换一个名称
        </span>
      </div>
    </div>

    <!-- 附加组件选项折叠列表 (数据驱动卡片渲染) -->
    <div class="flex flex-col gap-2">
      <LoaderCard
        v-for="cfg in loaderConfigs"
        :key="cfg.type"
        :title="cfg.title"
        :is-open="openCard === cfg.type"
        :selected-version="selectedLoaderType === cfg.type ? selectedLoaderVer : ''"
        :loading="cfg.loading"
        :supported="cfg.supported"
        :status-text="cfg.statusText"
        :select-label="cfg.selectLabel"
        :loading-text="cfg.loadingText"
        :empty-text="cfg.emptyText"
        :item-icon="cfg.itemIcon"
        :items="cfg.items"
        @toggle="openCard = openCard === cfg.type ? null : cfg.type"
        @select="(ver) => selectLoader(cfg.type, ver)"
        @clear="clearLoader"
      />
    </div>

    <div class="sticky bottom-1 flex items-center justify-center pt-3 pb-2 select-none">
      <button
        :disabled="isNameConflict || store.isDownloading"
        class="px-6 py-2 rounded-md text-xs font-medium cursor-pointer bg-pcl-primary hover:bg-pcl-hover text-white shadow-xs transition-colors disabled:opacity-40 disabled:cursor-not-allowed flex items-center gap-2"
        @click="handleStart"
      >
        <PclIcon v-if="store.isDownloading" name="refresh" :size="13" class="animate-spin" />
        <PclIcon v-else name="download" :size="13" />
        <span>{{ store.isDownloading ? `正在下载 (${store.downloadTask?.percentage.toFixed(0)}%)` : '开始下载' }}</span>
      </button>
    </div>
  </div>
</template>
