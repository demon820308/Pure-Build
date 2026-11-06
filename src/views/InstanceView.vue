<script setup lang="ts">
import { useAppStore } from '@/stores/app'
import MyButton from '@/components/base/MyButton.vue'
import MyListItem from '@/components/base/MyListItem.vue'
import PclIcon from '@/components/base/PclIcon.vue'
import { backend } from '@/services/backend'
import type { ModInfo, ResourceItem, SaveInfo } from '@/services/backend'
import { ref, onMounted, computed, watch } from 'vue'
import { open, save } from '@tauri-apps/plugin-dialog'

const store = useAppStore()

const activeTab = ref<'mods' | 'shaderpacks' | 'resourcepacks' | 'saves'>('mods')

const mods = ref<ModInfo[]>([])
const shaders = ref<ResourceItem[]>([])
const resourcePacks = ref<ResourceItem[]>([])
const saves = ref<SaveInfo[]>([])
const isLoading = ref(false)
const isImporting = ref(false)
const searchQuery = ref('')

const loadData = async () => {
  if (!store.currentVersion) return
  isLoading.value = true
  try {
    if (activeTab.value === 'mods') {
      mods.value = await backend.getInstalledMods(store.currentVersion.path, store.gameDir)
    } else if (activeTab.value === 'shaderpacks') {
      shaders.value = await backend.getResources(store.currentVersion.path, store.gameDir, 'shaderpacks')
    } else if (activeTab.value === 'resourcepacks') {
      resourcePacks.value = await backend.getResources(store.currentVersion.path, store.gameDir, 'resourcepacks')
    } else if (activeTab.value === 'saves') {
      saves.value = await backend.getInstanceSaves(store.currentVersion.path, store.gameDir)
    }
  } finally {
    isLoading.value = false
  }
}

watch([() => store.currentVersion, activeTab, () => store.instanceRefreshTrigger], () => {
  loadData()
})

onMounted(() => {
  loadData()
})

const filteredMods = computed(() => {
  const q = searchQuery.value.toLowerCase().trim()
  if (!q) return mods.value
  return mods.value.filter(
    (m) =>
      m.name.toLowerCase().includes(q) ||
      m.mod_id.toLowerCase().includes(q) ||
      m.file_name.toLowerCase().includes(q)
  )
})

const filteredResources = computed(() => {
  const q = searchQuery.value.toLowerCase().trim()
  const list = activeTab.value === 'shaderpacks' ? shaders.value : resourcePacks.value
  if (!q) return list
  return list.filter((item) => item.name.toLowerCase().includes(q))
})

const filteredSaves = computed(() => {
  const q = searchQuery.value.toLowerCase().trim()
  if (!q) return saves.value
  return saves.value.filter((s) => s.folder_name.toLowerCase().includes(q))
})

const toggleMod = async (mod: ModInfo) => {
  if (!store.currentVersion) return
  if (!await store.validateTargetExists(store.currentVersion.path, `版本【${store.currentVersion.name || store.currentVersion.id}】`)) return
  try {
    const newName = await backend.toggleInstanceItem(
      store.currentVersion.path,
      store.gameDir,
      'mods',
      mod.file_name
    )
    mod.file_name = newName
    mod.is_enabled = !mod.file_name.endsWith('.disabled')
  } catch (err: any) {
    alert(`切换 Mod 状态失败: ${err}`)
  }
}

const deleteItem = async (folderName: string, fileName: string) => {
  if (!store.currentVersion) return
  if (!await store.validateTargetExists(store.currentVersion.path, `版本【${store.currentVersion.name || store.currentVersion.id}】`)) return
  if (!confirm(`确定要从本地删除 [${fileName}] 吗？`)) return
  try {
    await backend.deleteInstanceItem(
      store.currentVersion.path,
      store.gameDir,
      folderName,
      fileName
    )
    await loadData()
  } catch (err: any) {
    alert(`删除失败: ${err}`)
  }
}

const toggleResource = async (item: ResourceItem) => {
  if (!store.currentVersion) return
  if (!await store.validateTargetExists(store.currentVersion.path, `版本【${store.currentVersion.name || store.currentVersion.id}】`)) return
  try {
    const newName = await backend.toggleInstanceItem(
      store.currentVersion.path,
      store.gameDir,
      activeTab.value,
      item.file_name
    )
    item.file_name = newName
    item.is_enabled = !item.file_name.endsWith('.disabled')
  } catch (err: any) {
    alert(`切换状态失败: ${err}`)
  }
}

const openCurrentFolder = async () => {
  if (!store.currentVersion) return
  if (!await store.validateTargetExists(store.currentVersion.path, `版本【${store.currentVersion.name || store.currentVersion.id}】`)) return
  try {
    await backend.openInstanceFolder(store.currentVersion.path, store.gameDir, activeTab.value)
  } catch (err: any) {
    alert(`打开目录失败: ${err}`)
  }
}

const handleImportModpack = async () => {
  try {
    const selected = await open({
      multiple: false,
      title: '选择要导入的 Modrinth (.mrpack) 整合包文件',
      filters: [{ name: 'Modrinth 整合包', extensions: ['mrpack', 'zip'] }],
    })
    if (!selected || typeof selected !== 'string') return

    isImporting.value = true
    const installedName = await backend.importModpack({
      gameDir: store.gameDir,
      mrpackPath: selected,
    })
    await store.refreshInstalledVersions()
    const target = store.installedVersions.find((v) => v.id === installedName)
    if (target) {
      store.currentVersion = target
    }
    await loadData()
    alert(`整合包 [${installedName}] 导入成功。已自动配置底层核心、加载器与模组。`)
  } catch (err: any) {
    alert(`导入整合包失败: ${err}`)
  } finally {
    isImporting.value = false
  }
}

const handleExportModpack = async () => {
  if (!store.currentVersion) return
  if (!await store.validateTargetExists(store.currentVersion.path, `版本【${store.currentVersion.name || store.currentVersion.id}】`)) return
  try {
    const outPath = await save({
      title: `导出实例 [${store.currentVersion.name}] 为 .mrpack 整合包`,
      defaultPath: `${store.currentVersion.name}.mrpack`,
      filters: [{ name: 'Modrinth 整合包', extensions: ['mrpack'] }],
    })
    if (!outPath) return

    const saved = await backend.exportModpack({
      versionPath: store.currentVersion.path,
      gameDir: store.gameDir,
      versionId: store.currentVersion.id,
      packName: store.currentVersion.name,
      packVersion: '1.0.0',
      outputFile: outPath,
    })
    alert(`整合包已成功导出至：\n${saved}`)
  } catch (err: any) {
    alert(`导出整合包失败: ${err}`)
  }
}
</script>

<template>
  <div class="h-full flex overflow-hidden" :class="store.uiStyle === 'classic' ? 'flex-row gap-0' : 'flex-col gap-4'">
    <div
      v-if="store.uiStyle === 'classic'"
      class="w-[152px] h-full bg-white dark:bg-[#1e232b] border-r border-black/8 dark:border-white/10 pt-3 flex flex-col shrink-0 select-none"
    >
      <MyListItem
        :title="`Mod 管理 (${mods.length})`"
        icon="mod"
        :checked="activeTab === 'mods'"
        action-title="刷新 Mod"
        @click="activeTab = 'mods'"
        @action="loadData"
      />
      <MyListItem
        :title="`世界存档 (${saves.length})`"
        icon="vanilla"
        :checked="activeTab === 'saves'"
        action-title="刷新存档"
        @click="activeTab = 'saves'"
        @action="loadData"
      />
      <MyListItem
        :title="`资源包 (${resourcePacks.length})`"
        icon="resourcepack"
        :checked="activeTab === 'resourcepacks'"
        action-title="刷新资源包"
        @click="activeTab = 'resourcepacks'"
        @action="loadData"
      />
      <MyListItem
        :title="`光影包 (${shaders.length})`"
        icon="shader"
        :checked="activeTab === 'shaderpacks'"
        action-title="刷新光影包"
        @click="activeTab = 'shaderpacks'"
        @action="loadData"
      />
      <div class="px-3.5 pt-4 pb-1 text-[12px] text-[#737373] opacity-80">
        整合包操作
      </div>
      <MyListItem
        title="导入 .mrpack"
        icon="download"
        @click="handleImportModpack"
      />
      <MyListItem
        v-if="store.currentVersion"
        title="导出整合包"
        icon="modpack"
        @click="handleExportModpack"
      />
    </div>

    <div
      class="flex-1 h-full flex flex-col gap-4 overflow-hidden"
      :class="store.uiStyle === 'classic' && 'p-5'"
    >
    <div class="flex items-center justify-between gap-4 shrink-0">
      <div class="flex items-center gap-3">
        <div class="w-9 h-9 rounded-lg bg-black/5 dark:bg-white/5 border border-pcl-border text-pcl-primary flex items-center justify-center shrink-0">
          <PclIcon name="instance" :size="18" />
        </div>
        <div>
          <h2 class="text-sm font-semibold text-pcl-text flex items-center gap-2">
            <span>{{ store.currentVersion?.name || '未选择版本' }}</span>
            <span class="text-xs bg-black/5 dark:bg-white/10 text-pcl-subtext px-1.5 py-0.2 rounded font-mono">
              {{ store.currentVersion?.loader || 'Vanilla' }}
            </span>
          </h2>
          <p class="text-xs text-pcl-subtext mt-0.5">
            版本内容管理、存档查看与整合包导入/导出中心
          </p>
        </div>
      </div>

      <div class="flex items-center gap-2">
        <MyButton type="secondary" size="small" :loading="isImporting" @click="handleImportModpack">
          <PclIcon name="download" :size="12" class="mr-1" />
          导入整合包 (.mrpack)
        </MyButton>
        <MyButton v-if="store.currentVersion" type="secondary" size="small" @click="handleExportModpack">
          导出整合包
        </MyButton>
      </div>
    </div>

    <div class="flex items-center justify-between gap-3 shrink-0 flex-wrap">
      <div class="flex items-center p-1 rounded-lg bg-black/5 dark:bg-white/5 border border-pcl-border text-xs shrink-0 whitespace-nowrap">
        <button
          class="px-2.5 py-1 rounded-md transition-colors cursor-pointer whitespace-nowrap shrink-0 flex items-center gap-1"
          :class="activeTab === 'mods' ? 'bg-pcl-card text-pcl-text font-medium shadow-xs' : 'text-pcl-subtext hover:text-pcl-text'"
          @click="activeTab = 'mods'"
        >
          <PclIcon name="mod" :size="12" />
          <span>模组 ({{ mods.length }})</span>
        </button>
        <button
          class="px-2.5 py-1 rounded-md transition-colors cursor-pointer whitespace-nowrap shrink-0 flex items-center gap-1"
          :class="activeTab === 'shaderpacks' ? 'bg-pcl-card text-pcl-text font-medium shadow-xs' : 'text-pcl-subtext hover:text-pcl-text'"
          @click="activeTab = 'shaderpacks'"
        >
          <PclIcon name="shader" :size="12" />
          <span>光影 ({{ shaders.length }})</span>
        </button>
        <button
          class="px-2.5 py-1 rounded-md transition-colors cursor-pointer whitespace-nowrap shrink-0 flex items-center gap-1"
          :class="activeTab === 'resourcepacks' ? 'bg-pcl-card text-pcl-text font-medium shadow-xs' : 'text-pcl-subtext hover:text-pcl-text'"
          @click="activeTab = 'resourcepacks'"
        >
          <PclIcon name="resourcepack" :size="12" />
          <span>材质 ({{ resourcePacks.length }})</span>
        </button>
        <button
          class="px-2.5 py-1 rounded-md transition-colors cursor-pointer whitespace-nowrap shrink-0 flex items-center gap-1"
          :class="activeTab === 'saves' ? 'bg-pcl-card text-pcl-text font-medium shadow-xs' : 'text-pcl-subtext hover:text-pcl-text'"
          @click="activeTab = 'saves'"
        >
          <span>存档 ({{ saves.length }})</span>
        </button>
      </div>

      <div class="flex items-center gap-2">
        <input
          v-model="searchQuery"
          placeholder="搜索名称..."
          class="px-3 py-1.5 rounded-md bg-black/5 dark:bg-white/5 border border-pcl-border text-xs text-pcl-text outline-none focus:border-pcl-primary transition-colors w-44"
        />

        <MyButton type="secondary" size="small" @click="openCurrentFolder">
          <PclIcon name="folder" :size="12" class="mr-1" />
          打开目录
        </MyButton>

        <MyButton type="secondary" size="small" :loading="isLoading" @click="loadData">
          <PclIcon name="refresh" :size="12" />
        </MyButton>
      </div>
    </div>

    <div class="flex-1 overflow-y-auto pr-1">
      <template v-if="activeTab === 'mods'">
        <div v-if="filteredMods.length === 0 && !isLoading" class="h-64 flex flex-col items-center justify-center text-pcl-subtext text-xs gap-2">
          <PclIcon name="mod" :size="32" class="text-pcl-subtext opacity-40" />
          <span>当前版本暂无已安装的模组</span>
        </div>
        <div v-else class="space-y-1.5">
          <div
            v-for="mod in filteredMods"
            :key="mod.file_name"
            class="flex items-center justify-between p-3 rounded-xl border border-pcl-border bg-pcl-card shadow-xs transition-colors"
            :class="!mod.is_enabled && 'opacity-60 bg-black/2 dark:bg-white/2'"
          >
            <div class="flex items-center gap-3 overflow-hidden">
              <div class="w-8 h-8 rounded-lg bg-black/5 dark:bg-white/5 flex items-center justify-center text-pcl-primary shrink-0">
                <PclIcon name="mod" :size="16" />
              </div>
              <div class="truncate">
                <div class="font-semibold text-sm text-pcl-text flex items-center gap-2">
                  <span class="truncate">{{ mod.name }}</span>
                  <span class="text-[10px] bg-black/5 dark:bg-white/10 px-1.5 py-0.2 rounded text-pcl-subtext font-mono shrink-0">
                    v{{ mod.version }}
                  </span>
                  <span v-if="mod.authors && mod.authors.length > 0" class="text-[10px] text-pcl-subtext font-normal truncate max-w-36">
                    by {{ mod.authors.join(', ') }}
                  </span>
                  <span v-if="!mod.is_enabled" class="text-[10px] bg-red-500/10 text-red-500 px-1.5 py-0.2 rounded font-normal shrink-0">
                    已禁用
                  </span>
                </div>
                <div class="text-xs text-pcl-subtext truncate mt-0.5">
                  {{ mod.description || mod.file_name }}
                </div>
              </div>
            </div>

            <div class="shrink-0 ml-4 flex items-center gap-2">
              <button
                class="px-2.5 py-1 rounded-md text-xs font-medium cursor-pointer transition-colors border"
                :class="[
                  mod.is_enabled
                    ? 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 border-emerald-500/20 hover:bg-emerald-500/20'
                    : 'bg-black/5 dark:bg-white/10 text-pcl-subtext border-pcl-border hover:text-pcl-text'
                ]"
                @click="toggleMod(mod)"
              >
                {{ mod.is_enabled ? '已启用' : '已停用' }}
              </button>
              <button
                class="p-1 rounded-md text-xs text-pcl-subtext hover:text-red-500 hover:bg-red-500/10 transition-colors cursor-pointer"
                title="删除此模组"
                @click="deleteItem('mods', mod.file_name)"
              >
                <PclIcon name="close" :size="11" />
              </button>
            </div>
          </div>
        </div>
      </template>

      <template v-else-if="activeTab === 'saves'">
        <div v-if="filteredSaves.length === 0 && !isLoading" class="h-64 flex flex-col items-center justify-center text-pcl-subtext text-xs gap-2">
          <PclIcon name="instance" :size="32" class="text-pcl-subtext opacity-40" />
          <span>当前版本暂无单人世界存档</span>
        </div>
        <div v-else class="space-y-2">
          <div
            v-for="sv in filteredSaves"
            :key="sv.folder_name"
            class="flex items-center justify-between p-3.5 rounded-2xl border border-pcl-border bg-pcl-card shadow-sm transition-all duration-200 hover:shadow-md"
          >
            <div class="flex items-center gap-3.5 overflow-hidden">
              <div class="w-10 h-10 rounded-xl bg-emerald-500/10 text-emerald-600 flex items-center justify-center shrink-0">
                <PclIcon name="instance" :size="18" />
              </div>
              <div class="truncate">
                <div class="font-semibold text-sm text-pcl-text flex items-center gap-2">
                  <span class="truncate">{{ sv.folder_name }}</span>
                  <span class="text-[10px] bg-black/5 dark:bg-white/10 px-1.5 py-0.2 rounded text-pcl-subtext font-normal shrink-0">
                    {{ sv.size_mb.toFixed(1) }} MB
                  </span>
                </div>
                <div class="text-xs text-pcl-subtext truncate mt-0.5">
                  {{ sv.path }}
                </div>
              </div>
            </div>

            <div class="shrink-0 ml-4 flex items-center gap-2">
              <MyButton type="secondary" size="small" @click="backend.openFolder(sv.path)">
                <PclIcon name="folder" :size="12" class="mr-1" />
                浏览存档
              </MyButton>
              <button
                class="p-1 rounded-md text-xs text-pcl-subtext hover:text-red-500 hover:bg-red-500/10 transition-colors cursor-pointer"
                title="删除此存档"
                @click="deleteItem('saves', sv.folder_name)"
              >
                <PclIcon name="close" :size="11" />
              </button>
            </div>
          </div>
        </div>
      </template>

      <template v-else>
        <div v-if="filteredResources.length === 0 && !isLoading" class="h-64 flex flex-col items-center justify-center text-pcl-subtext text-xs gap-2">
          <PclIcon name="folder" :size="32" class="text-pcl-subtext opacity-40" />
          <span>该目录下暂无任何相关文件</span>
        </div>
        <div v-else class="space-y-1.5">
          <div
            v-for="item in filteredResources"
            :key="item.file_name"
            class="flex items-center justify-between p-3 rounded-xl border border-pcl-border bg-pcl-card shadow-xs transition-colors"
            :class="!item.is_enabled && 'opacity-60 bg-black/2 dark:bg-white/2'"
          >
            <div class="flex items-center gap-3 overflow-hidden">
              <div class="w-8 h-8 rounded-lg bg-black/5 dark:bg-white/5 flex items-center justify-center text-pcl-primary shrink-0">
                <PclIcon :name="activeTab === 'shaderpacks' ? 'shader' : 'resourcepack'" :size="16" />
              </div>
              <div class="truncate">
                <div class="font-semibold text-sm text-pcl-text flex items-center gap-2">
                  <span class="truncate">{{ item.name }}</span>
                  <span v-if="item.size_mb > 0" class="text-[10px] bg-black/5 dark:bg-white/10 px-1.5 py-0.2 rounded text-pcl-subtext font-mono shrink-0">
                    {{ item.size_mb.toFixed(1) }} MB
                  </span>
                  <span v-if="!item.is_enabled" class="text-[10px] bg-red-500/10 text-red-500 px-1.5 py-0.2 rounded font-normal shrink-0">
                    已禁用
                  </span>
                </div>
                <div class="text-xs text-pcl-subtext truncate mt-0.5">
                  {{ item.file_name }}
                </div>
              </div>
            </div>

            <div class="shrink-0 ml-4 flex items-center gap-2">
              <button
                class="px-2.5 py-1 rounded-md text-xs font-medium cursor-pointer transition-colors border"
                :class="[
                  item.is_enabled
                    ? 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 border-emerald-500/20 hover:bg-emerald-500/20'
                    : 'bg-black/5 dark:bg-white/10 text-pcl-subtext border-pcl-border hover:text-pcl-text'
                ]"
                @click="toggleResource(item)"
              >
                {{ item.is_enabled ? '已启用' : '已停用' }}
              </button>
              <button
                class="p-1 rounded-md text-xs text-pcl-subtext hover:text-red-500 hover:bg-red-500/10 transition-colors cursor-pointer"
                title="删除"
                @click="deleteItem(activeTab, item.file_name)"
              >
                <PclIcon name="close" :size="11" />
              </button>
            </div>
          </div>
        </div>
      </template>
    </div>
    </div>
  </div>
</template>
