<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import { useAppStore } from '@/stores/app'
import { backend } from '@/services/backend'
import type { InstalledCommunitySnapshot, ModrinthProject } from '@/services/backend'
import MyButton from '@/components/base/MyButton.vue'
import PclIcon from '@/components/base/PclIcon.vue'
import InlineProgress from './InlineProgress.vue'

const props = defineProps<{
  section: 'mod' | 'modpack' | 'shader' | 'resourcepack'
}>()

const store = useAppStore()

const communityProjects = ref<ModrinthProject[]>([])
const isLoadingCommunity = ref(false)
const communitySearchQuery = ref('')
const selectedLoader = ref<'all' | 'fabric' | 'forge' | 'neoforge'>('all')
const filterByCurrentVersion = ref(true)
const selectedJavaVersionId = ref('')
const installedSnapshot = ref<InstalledCommunitySnapshot | null>(null)

const targetVersionObj = computed(
  () => store.installedVersions.find((v) => v.id === selectedJavaVersionId.value) || null
)

const syncTargetVersion = () => {
  if (
    selectedJavaVersionId.value &&
    store.installedVersions.some((v) => v.id === selectedJavaVersionId.value)
  ) {
    return
  }
  selectedJavaVersionId.value =
    store.currentVersion?.id ||
    store.installedVersions[0]?.id ||
    ''
}

const normalizeKey = (s: string) => s.toLowerCase().replace(/[^a-z0-9]/g, '')

const refreshInstalledSnapshot = async () => {
  if (!store.gameDir || props.section === 'modpack') {
    installedSnapshot.value = null
    return
  }
  try {
    installedSnapshot.value = await backend.getInstalledCommunitySnapshot({
      gameDir: store.gameDir,
      versionPath: targetVersionObj.value?.path,
      projectType: props.section,
    })
  } catch (e) {
    console.warn('获取社区资源已安装快照失败:', e)
  }
}

const installedNormalizedSet = computed(() => {
  const snap = installedSnapshot.value
  const set = new Set<string>()
  if (!snap) return set
  for (const id of snap.mod_ids) {
    const n = normalizeKey(id)
    if (n.length >= 3) set.add(n)
  }
  for (const fname of snap.file_names) {
    const stem = fname.replace(/\.(jar|zip|disabled)$/gi, '')
    const n = normalizeKey(stem)
    if (n.length >= 3) set.add(n)
  }
  return set
})

const isCommunityInstalled = (proj: ModrinthProject): boolean => {
  const snap = installedSnapshot.value
  if (!snap || props.section === 'modpack') return false
  if (snap.project_ids.includes(proj.project_id)) return true
  const slugKey = normalizeKey(proj.slug)
  const titleKey = normalizeKey(proj.title)
  if (slugKey.length >= 3 && installedNormalizedSet.value.has(slugKey)) return true
  if (titleKey.length >= 3 && installedNormalizedSet.value.has(titleKey)) return true
  for (const localKey of installedNormalizedSet.value) {
    if (slugKey.length >= 4 && localKey.startsWith(slugKey)) return true
  }
  return false
}

const searchCommunity = async () => {
  syncTargetVersion()
  isLoadingCommunity.value = true
  try {
    const targetVer = filterByCurrentVersion.value && targetVersionObj.value
      ? targetVersionObj.value.id.replace(/-.*$/, '') // 提取如 1.21.4
      : undefined

    const results = await backend.searchModrinthResources({
      query: communitySearchQuery.value,
      projectType: props.section,
      gameVersion: targetVer,
      loader: props.section === 'mod' || props.section === 'modpack' ? selectedLoader.value : undefined,
    })
    communityProjects.value = results
  } catch (err: any) {
    console.error('搜索社区资源异常:', err)
  } finally {
    isLoadingCommunity.value = false
  }
}

watch(
  [() => props.section, selectedLoader, filterByCurrentVersion],
  () => {
    refreshInstalledSnapshot()
    searchCommunity()
  }
)

watch(selectedJavaVersionId, () => {
  refreshInstalledSnapshot()
  if (filterByCurrentVersion.value) {
    searchCommunity()
  }
})

watch(
  () => store.installedVersions,
  () => {
    syncTargetVersion()
    refreshInstalledSnapshot()
  }
)

onMounted(() => {
  syncTargetVersion()
  refreshInstalledSnapshot()
  searchCommunity()
})

defineExpose({
  searchCommunity,
})

const formatDownloads = (num: number) => {
  if (num >= 1000000) return `${(num / 1000000).toFixed(1)}M`
  if (num >= 10000) return `${(num / 10000).toFixed(1)}万`
  return num.toString()
}

const handleInstallCommunity = async (proj: ModrinthProject) => {
  if (props.section !== 'modpack' && store.installedVersions.length === 0) {
    alert('请先在「游戏版本」安装至少一个 Java 游戏版本后再安装社区资源！')
    return
  }
  try {
    const activeTarget = targetVersionObj.value
    const targetVer = activeTarget ? activeTarget.id.replace(/-.*$/, '') : undefined
    const targetLoader = activeTarget?.loader || 'fabric'

    store.updateDownloadProgress({
      version_id: proj.project_id,
      vanilla_version: targetVer,
      step: `准备下载 ${proj.title}...`,
      percentage: 5,
      speed_text: '请求中',
      is_finished: false,
    })

    const msg = await backend.installModrinthResource({
      gameDir: store.gameDir,
      versionPath: activeTarget?.path,
      projectId: proj.project_id,
      projectType: props.section,
      gameVersion: targetVer,
      loader: props.section === 'mod' ? targetLoader : undefined,
    })

    console.log(msg)
    await refreshInstalledSnapshot()
  } catch (err: any) {
    alert(`安装失败: ${err}`)
    store.clearDownloadTask()
  }
}
</script>

<template>
  <div class="flex flex-col gap-3.5 flex-1 overflow-hidden">
    <div class="flex items-center justify-between gap-3 shrink-0 flex-wrap">
      <div class="flex items-center gap-2 shrink-0 flex-wrap">
        <select
          v-if="store.installedVersions.length > 0"
          v-model="selectedJavaVersionId"
          class="px-2.5 py-1.5 rounded-xl bg-black/5 dark:bg-white/5 border border-pcl-border text-xs text-pcl-text outline-none max-w-[190px]"
          title="选择社区资源要匹配及安装的 Java 目标版本"
        >
          <option v-for="v in store.installedVersions" :key="v.id" :value="v.id">
            目标版本：{{ v.name }}
          </option>
        </select>

        <span
          v-if="section !== 'modpack' && targetVersionObj && installedSnapshot"
          class="text-[11px] px-2 py-1 rounded-lg border shrink-0 whitespace-nowrap"
          :class="
            installedSnapshot.is_isolated
              ? 'bg-emerald-500/10 border-emerald-500/25 text-emerald-600 dark:text-emerald-400'
              : 'bg-amber-500/10 border-amber-500/25 text-amber-600 dark:text-amber-400'
          "
          :title="installedSnapshot.is_isolated ? `独立版本隔离目录：${installedSnapshot.effective_dir}` : `通用共享目录：${installedSnapshot.effective_dir}（所有未隔离 Java 版本共用该目录下的资源与模组）`"
        >
          {{ installedSnapshot.is_isolated ? '独立隔离目录' : '通用共享目录' }}
        </span>

        <label class="flex items-center gap-1.5 px-2.5 py-1.5 rounded-xl bg-black/5 dark:bg-white/5 border border-pcl-border text-xs text-pcl-text cursor-pointer hover:border-pcl-primary select-none whitespace-nowrap shrink-0">
          <input v-model="filterByCurrentVersion" type="checkbox" class="accent-[var(--pcl-color-primary)] cursor-pointer shrink-0" />
          <span class="font-medium text-[11px] whitespace-nowrap">匹配目标版本</span>
        </label>

        <div v-if="section === 'mod'" class="flex items-center p-1 rounded-xl bg-black/5 dark:bg-white/5 border border-pcl-border text-xs shrink-0 whitespace-nowrap">
          <button
            class="px-2.5 py-1 rounded-lg transition-all cursor-pointer whitespace-nowrap shrink-0"
            :class="selectedLoader === 'all' ? 'bg-pcl-card text-pcl-text font-bold shadow-sm' : 'text-pcl-subtext hover:text-pcl-text'"
            @click="selectedLoader = 'all'"
          >
            全部
          </button>
          <button
            class="px-2.5 py-1 rounded-lg transition-all cursor-pointer whitespace-nowrap shrink-0"
            :class="selectedLoader === 'fabric' ? 'bg-pcl-card text-pcl-text font-bold shadow-sm' : 'text-pcl-subtext hover:text-pcl-text'"
            @click="selectedLoader = 'fabric'"
          >
            Fabric
          </button>
          <button
            class="px-2.5 py-1 rounded-lg transition-all cursor-pointer whitespace-nowrap shrink-0"
            :class="selectedLoader === 'forge' ? 'bg-pcl-card text-pcl-text font-bold shadow-sm' : 'text-pcl-subtext hover:text-pcl-text'"
            @click="selectedLoader = 'forge'"
          >
            Forge
          </button>
          <button
            class="px-2.5 py-1 rounded-lg transition-all cursor-pointer whitespace-nowrap shrink-0"
            :class="selectedLoader === 'neoforge' ? 'bg-pcl-card text-pcl-text font-bold shadow-sm' : 'text-pcl-subtext hover:text-pcl-text'"
            @click="selectedLoader = 'neoforge'"
          >
            NeoForge
          </button>
        </div>
      </div>

      <div class="flex items-center gap-2 flex-1 max-w-sm justify-end min-w-0">
        <input
          v-model="communitySearchQuery"
          :placeholder="`在 Modrinth 搜索${section === 'mod' ? '模组' : section === 'shader' ? '光影' : '材质包'}...`"
          class="px-3 py-1.5 rounded-md bg-black/5 dark:bg-white/5 border border-pcl-border text-xs text-pcl-text outline-none focus:border-pcl-primary transition-colors flex-1 min-w-[120px] max-w-xs"
          @keyup.enter="searchCommunity"
        />
        <MyButton type="primary" size="small" :loading="isLoadingCommunity" @click="searchCommunity">
          <PclIcon name="search" :size="12" class="mr-1" />
          搜索
        </MyButton>
      </div>
    </div>

    <div class="flex-1 overflow-y-auto pr-1 space-y-2">
      <div v-if="isLoadingCommunity" class="h-64 flex flex-col items-center justify-center text-pcl-subtext text-xs gap-3">
        <PclIcon name="refresh" :size="24" class="animate-spin text-pcl-primary" />
        <span>正在检索 Modrinth 社区资源...</span>
      </div>

      <div v-else-if="communityProjects.length === 0" class="h-64 flex flex-col items-center justify-center text-pcl-subtext text-xs gap-2">
        <PclIcon :name="section === 'mod' ? 'mod' : section === 'shader' ? 'shader' : 'resourcepack'" :size="32" class="text-pcl-subtext opacity-40" />
        <span>输入关键词并按回车搜索</span>
      </div>

      <div
        v-for="p in communityProjects"
        :key="p.project_id"
        class="flex flex-col p-3.5 rounded-xl border transition-colors"
        :class="[
          store.downloadTask?.version_id === p.project_id
            ? 'border-pcl-primary/60 bg-pcl-primary/5'
            : 'border-pcl-border bg-pcl-card shadow-xs'
        ]"
      >
        <div class="flex items-start justify-between gap-4">
          <div class="flex items-start gap-3.5 overflow-hidden flex-1">
            <img
              v-if="p.icon_url"
              :src="p.icon_url"
              class="w-11 h-11 rounded-lg object-cover shrink-0 border border-pcl-border/50 bg-black/5"
              alt="icon"
            />
            <div
              v-else
              class="w-11 h-11 rounded-lg bg-black/5 dark:bg-white/5 text-pcl-subtext flex items-center justify-center shrink-0"
            >
              <PclIcon :name="section === 'mod' ? 'mod' : section === 'shader' ? 'shader' : 'resourcepack'" :size="20" />
            </div>

            <div class="flex-1 min-w-0">
              <div class="font-semibold text-sm text-pcl-text flex items-center gap-2 flex-wrap">
                <span class="truncate">{{ p.title }}</span>
                <span
                  v-if="isCommunityInstalled(p)"
                  class="text-[10px] px-1.5 py-0.2 rounded font-medium bg-emerald-500/15 text-emerald-600 dark:text-emerald-400 flex items-center gap-0.5 shrink-0"
                >
                  <PclIcon name="check" :size="10" /> 已安装
                </span>
                <span class="text-[10px] text-pcl-subtext font-normal">by {{ p.author }}</span>
                <span class="text-[10px] px-1.5 py-0.2 rounded bg-black/5 dark:bg-white/10 text-pcl-subtext font-mono">
                  {{ formatDownloads(p.downloads) }} 下载
                </span>
              </div>

              <p class="text-xs text-pcl-subtext mt-1 line-clamp-2 leading-relaxed">
                {{ p.description }}
              </p>

              <div class="flex flex-wrap gap-1 mt-2">
                <span
                  v-for="cat in p.categories.slice(0, 5)"
                  :key="cat"
                  class="text-[10px] px-1.5 py-0.2 rounded bg-black/5 dark:bg-white/5 text-pcl-subtext font-mono"
                >
                  #{{ cat }}
                </span>
              </div>
            </div>
          </div>

          <div class="shrink-0 flex items-center">
            <div
              v-if="store.downloadTask?.version_id === p.project_id"
              class="flex items-center gap-2 text-xs font-medium text-pcl-primary px-3 py-1.5 rounded-md bg-pcl-primary/10"
            >
              <PclIcon name="refresh" :size="12" class="animate-spin" />
              <span class="font-mono tabular-nums">{{ store.downloadTask.percentage.toFixed(0) }}% {{ store.downloadTask.speed_text }}</span>
            </div>

            <button
              v-else
              :disabled="store.isDownloading"
              class="px-3.5 py-1.5 rounded-md text-xs font-medium cursor-pointer shadow-xs transition-colors disabled:opacity-50 whitespace-nowrap shrink-0 flex items-center gap-1.5"
              :class="
                isCommunityInstalled(p)
                  ? 'bg-black/5 dark:bg-white/10 hover:bg-black/10 dark:hover:bg-white/15 text-pcl-text'
                  : 'bg-pcl-primary hover:bg-pcl-hover active:bg-pcl-active text-white'
              "
              @click="handleInstallCommunity(p)"
            >
              <PclIcon :name="isCommunityInstalled(p) ? 'refresh' : 'download'" :size="12" />
              <span>{{ isCommunityInstalled(p) ? '重新安装' : `安装到 ${section === 'mod' ? 'Mods' : section === 'shader' ? 'Shaders' : 'Textures'}` }}</span>
            </button>
          </div>
        </div>

        <InlineProgress
          v-if="store.downloadTask?.version_id === p.project_id"
          :percentage="store.downloadTask.percentage"
          :step="store.downloadTask.step"
          :speed-text="store.downloadTask.speed_text"
        />
      </div>
    </div>
  </div>
</template>
