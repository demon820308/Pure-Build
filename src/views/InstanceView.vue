<script setup lang="ts">
import { useAppStore } from '@/stores/app'
import MyButton from '@/components/base/MyButton.vue'
import { backend } from '@/services/backend'
import type { ModInfo, ResourceItem } from '@/services/backend'
import { ref, onMounted, computed, watch } from 'vue'

const store = useAppStore()

// 当前子标签页：'mods' | 'shaderpacks' | 'resourcepacks'
const activeTab = ref<'mods' | 'shaderpacks' | 'resourcepacks'>('mods')

const mods = ref<ModInfo[]>([])
const shaders = ref<ResourceItem[]>([])
const resourcePacks = ref<ResourceItem[]>([])
const isLoading = ref(false)
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
    }
  } finally {
    isLoading.value = false
  }
}

watch([() => store.currentVersion, activeTab], () => {
  loadData()
})

onMounted(() => {
  loadData()
})

// 过滤后的列表
const filteredList = computed(() => {
  const q = searchQuery.value.toLowerCase().trim()
  if (activeTab.value === 'mods') {
    if (!q) return mods.value
    return mods.value.filter(
      (m) =>
        m.name.toLowerCase().includes(q) ||
        m.mod_id.toLowerCase().includes(q) ||
        m.file_name.toLowerCase().includes(q)
    )
  } else if (activeTab.value === 'shaderpacks') {
    if (!q) return shaders.value
    return shaders.value.filter((s) => s.name.toLowerCase().includes(q))
  } else {
    if (!q) return resourcePacks.value
    return resourcePacks.value.filter((r) => r.name.toLowerCase().includes(q))
  }
})

const toggleMod = async (mod: ModInfo) => {
  if (!store.currentVersion) return
  const modsDir = `${store.currentVersion.path}\\mods`
  try {
    const newName = await backend.toggleModState(modsDir, mod.file_name)
    mod.file_name = newName
    mod.is_enabled = !mod.file_name.endsWith('.disabled')
  } catch (err: any) {
    alert(`切换 Mod 状态失败: ${err}`)
  }
}

const toggleResource = async (item: ResourceItem) => {
  if (!store.currentVersion) return
  const folder = `${store.currentVersion.path}\\${activeTab.value}`
  try {
    const newName = await backend.toggleModState(folder, item.file_name)
    item.file_name = newName
    item.is_enabled = !item.file_name.endsWith('.disabled')
  } catch (err: any) {
    alert(`切换状态失败: ${err}`)
  }
}

const openCurrentFolder = async () => {
  if (!store.currentVersion) return
  const target = `${store.currentVersion.path}\\${activeTab.value}`
  await backend.openInExplorer(target)
}
</script>

<template>
  <div class="h-full flex flex-col gap-4 overflow-hidden">
    <!-- 顶部状态指示与操作栏 -->
    <div class="flex items-center justify-between gap-4 shrink-0">
      <div class="flex items-center gap-3">
        <div class="w-10 h-10 rounded-xl bg-pcl-primary/20 flex items-center justify-center text-lg">
          📦
        </div>
        <div>
          <h2 class="text-base font-bold text-pcl-text flex items-center gap-2">
            <span>{{ store.currentVersion?.name || '未选择版本' }}</span>
            <span class="text-xs bg-pcl-primary/10 text-pcl-primary px-2 py-0.5 rounded font-normal">
              {{ store.currentVersion?.loader || '原版' }}
            </span>
          </h2>
          <p class="text-xs text-pcl-subtext mt-0.5">
            版本内容深度管理中心
          </p>
        </div>
      </div>

      <!-- 子标签页切换 (Mods / 光影 / 材质) -->
      <div class="flex items-center gap-2">
        <div class="flex items-center p-1 rounded-xl bg-black/5 dark:bg-white/5 border border-pcl-border text-xs">
          <button
            class="px-3 py-1 rounded-lg transition-all cursor-pointer"
            :class="activeTab === 'mods' ? 'bg-pcl-card text-pcl-text font-bold shadow-sm' : 'text-pcl-subtext hover:text-pcl-text'"
            @click="activeTab = 'mods'"
          >
            🧩 模组 (Mods)
          </button>
          <button
            class="px-3 py-1 rounded-lg transition-all cursor-pointer"
            :class="activeTab === 'shaderpacks' ? 'bg-pcl-card text-pcl-text font-bold shadow-sm' : 'text-pcl-subtext hover:text-pcl-text'"
            @click="activeTab = 'shaderpacks'"
          >
            ✨ 光影 (Shaders)
          </button>
          <button
            class="px-3 py-1 rounded-lg transition-all cursor-pointer"
            :class="activeTab === 'resourcepacks' ? 'bg-pcl-card text-pcl-text font-bold shadow-sm' : 'text-pcl-subtext hover:text-pcl-text'"
            @click="activeTab = 'resourcepacks'"
          >
            🎨 材质 (Textures)
          </button>
        </div>

        <input
          v-model="searchQuery"
          placeholder="搜索名称..."
          class="px-3.5 py-1.5 rounded-xl bg-black/5 dark:bg-white/5 border border-pcl-border text-xs text-pcl-text outline-none focus:border-pcl-primary transition-colors w-40"
        />

        <MyButton type="secondary" size="small" @click="openCurrentFolder">
          📂 打开目录
        </MyButton>

        <MyButton type="secondary" size="small" :loading="isLoading" @click="loadData">
          🔄
        </MyButton>
      </div>
    </div>

    <!-- 列表主视口 -->
    <div class="flex-1 overflow-y-auto pr-1">
      <div v-if="filteredList.length === 0 && !isLoading" class="h-64 flex flex-col items-center justify-center text-pcl-subtext text-xs gap-2">
        <span class="text-3xl">📂</span>
        <span>该目录下暂无任何相关文件</span>
      </div>

      <!-- Mods 模式列表 -->
      <div v-else-if="activeTab === 'mods'" class="space-y-2">
        <div
          v-for="mod in (filteredList as ModInfo[])"
          :key="mod.file_name"
          class="flex items-center justify-between p-3.5 rounded-2xl border border-pcl-border bg-pcl-card shadow-sm transition-all duration-200 hover:shadow-md"
          :class="!mod.is_enabled && 'opacity-60 bg-black/2 dark:bg-white/2'"
        >
          <div class="flex items-center gap-3.5 overflow-hidden">
            <div class="w-10 h-10 rounded-xl bg-black/5 dark:bg-white/5 flex items-center justify-center text-xl shrink-0">
              🧩
            </div>
            <div class="truncate">
              <div class="font-semibold text-sm text-pcl-text flex items-center gap-2">
                <span class="truncate">{{ mod.name }}</span>
                <span class="text-[10px] bg-black/5 dark:bg-white/10 px-1.5 py-0.2 rounded text-pcl-subtext font-normal shrink-0">
                  v{{ mod.version }}
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

          <div class="shrink-0 ml-4">
            <button
              class="px-3 py-1.5 rounded-xl text-xs font-medium cursor-pointer transition-all duration-200 ease-[var(--pcl-ease-bounce)] active:scale-95 border"
              :class="[
                mod.is_enabled
                  ? 'bg-emerald-500/10 text-emerald-600 border-emerald-500/20 hover:bg-emerald-500/20'
                  : 'bg-black/5 dark:bg-white/10 text-pcl-subtext border-pcl-border hover:text-pcl-text'
              ]"
              @click="toggleMod(mod)"
            >
              {{ mod.is_enabled ? '✔ 已启用' : '○ 已停用' }}
            </button>
          </div>
        </div>
      </div>

      <!-- 光影包与材质包模式列表 -->
      <div v-else class="space-y-2">
        <div
          v-for="item in (filteredList as ResourceItem[])"
          :key="item.file_name"
          class="flex items-center justify-between p-3.5 rounded-2xl border border-pcl-border bg-pcl-card shadow-sm transition-all duration-200 hover:shadow-md"
          :class="!item.is_enabled && 'opacity-60 bg-black/2 dark:bg-white/2'"
        >
          <div class="flex items-center gap-3.5 overflow-hidden">
            <div class="w-10 h-10 rounded-xl bg-black/5 dark:bg-white/5 flex items-center justify-center text-xl shrink-0">
              {{ activeTab === 'shaderpacks' ? '✨' : '🎨' }}
            </div>
            <div class="truncate">
              <div class="font-semibold text-sm text-pcl-text flex items-center gap-2">
                <span class="truncate">{{ item.name }}</span>
                <span v-if="item.size_mb > 0" class="text-[10px] bg-black/5 dark:bg-white/10 px-1.5 py-0.2 rounded text-pcl-subtext font-normal shrink-0">
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

          <div class="shrink-0 ml-4">
            <button
              class="px-3 py-1.5 rounded-xl text-xs font-medium cursor-pointer transition-all duration-200 ease-[var(--pcl-ease-bounce)] active:scale-95 border"
              :class="[
                item.is_enabled
                  ? 'bg-emerald-500/10 text-emerald-600 border-emerald-500/20 hover:bg-emerald-500/20'
                  : 'bg-black/5 dark:bg-white/10 text-pcl-subtext border-pcl-border hover:text-pcl-text'
              ]"
              @click="toggleResource(item)"
            >
              {{ item.is_enabled ? '✔ 已启用' : '○ 已停用' }}
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
