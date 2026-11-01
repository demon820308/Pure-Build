<script setup lang="ts">
import { useAppStore } from '@/stores/app'
import MyButton from '@/components/base/MyButton.vue'
import MyListItem from '@/components/base/MyListItem.vue'
import { backend } from '@/services/backend'
import type { RemoteVersion, ModrinthProject } from '@/services/backend'
import { ref, onMounted, computed, watch } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'

const store = useAppStore()

// 当前主分类：'game' (官方游戏核心) | 'mod' (模组) | 'modpack' (整合包) | 'shader' (光影包) | 'resourcepack' (材质包)
const currentSection = ref<'game' | 'mod' | 'modpack' | 'shader' | 'resourcepack'>('game')

// ==================== 1. 官方原版游戏状态 ====================
const versions = ref<RemoteVersion[]>([])
const isLoadingVersions = ref(false)
const filterType = ref<'all' | 'release' | 'snapshot' | 'installed'>('release')
const gameSearchQuery = ref('')

// ==================== 2. 社区资源 (Modrinth) 状态 ====================
const communityProjects = ref<ModrinthProject[]>([])
const isLoadingCommunity = ref(false)
const communitySearchQuery = ref('')
const selectedLoader = ref<'all' | 'fabric' | 'forge' | 'neoforge'>('all')
const filterByCurrentVersion = ref(true) // 是否仅筛选匹配当前选定 Minecraft 版本的资源

// 智能判定某版本是否正在下载/安装中（支持纯净版本、Loader后缀版本、自定义命名、服务端与另存为）
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

// 加载官方版本
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

// 搜索 Modrinth 社区资源
const searchCommunity = async () => {
  isLoadingCommunity.value = true
  try {
    const targetVer = filterByCurrentVersion.value && store.currentVersion
      ? store.currentVersion.id.replace(/-.*$/, '') // 提取如 1.21.4
      : undefined

    const results = await backend.searchModrinthResources({
      query: communitySearchQuery.value,
      projectType: currentSection.value as 'mod' | 'modpack' | 'shader' | 'resourcepack',
      gameVersion: targetVer,
      loader: currentSection.value === 'mod' || currentSection.value === 'modpack' ? selectedLoader.value : undefined,
    })
    communityProjects.value = results
  } catch (err: any) {
    console.error('搜索社区资源异常:', err)
  } finally {
    isLoadingCommunity.value = false
  }
}

// 当主分类切换或过滤条件变化时自动触发搜索
watch(
  [currentSection, selectedLoader, filterByCurrentVersion],
  () => {
    if (currentSection.value !== 'game') {
      searchCommunity()
    }
  }
)

// 检测版本是否已经在本地已安装
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
  return versions.value.filter((v) => isInstalled(v.id)).length
})

const goToLaunch = (versionId: string) => {
  const target = getInstalledObj(versionId)
  if (target) {
    store.currentVersion = target
  }
  store.currentTab = 'launch'
}

onMounted(() => {
  store.refreshInstalledVersions()
  loadRemoteVersions()
})

const filteredVersions = computed(() => {
  let list = versions.value

  if (filterType.value === 'installed') {
    list = list.filter((v) => isInstalled(v.id))
  } else if (filterType.value !== 'all') {
    list = list.filter((v) => v.type === filterType.value)
  }

  const q = gameSearchQuery.value.toLowerCase().trim()
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

const formatDownloads = (num: number) => {
  if (num >= 1000000) return `${(num / 1000000).toFixed(1)}M`
  if (num >= 10000) return `${(num / 10000).toFixed(1)}万`
  return num.toString()
}

// ==================== 1.1 原版游戏安装配置 (完全对齐原版 PCL) ====================
const configuringVersion = ref<RemoteVersion | null>(null)
const targetVersionName = ref('')
const customNameEdited = ref(false)

// 选择的附加 Loader
const selectedLoaderType = ref<'fabric' | 'forge' | 'neoforge' | 'optifine' | null>(null)
const selectedLoaderVer = ref('')

// 折叠展开状态
const openCard = ref<'forge' | 'neoforge' | 'fabric' | 'optifine' | null>(null)

// 异步加载与缓存的 Loader 数据
const fabricMeta = ref<{ supported_games: string[]; loaders: { version: string; stable: boolean; description?: string }[] } | null>(null)
const optifineGames = ref<string[]>([])
const forgeLoaders = ref<{ version: string; stable: boolean; description?: string }[]>([])
const neoForgeLoaders = ref<{ version: string; stable: boolean; description?: string }[]>([])
const optifineLoaders = ref<{ version: string; stable: boolean; description?: string }[]>([])

const isLoadingFabric = ref(false)
const isLoadingForge = ref(false)
const isLoadingNeoForge = ref(false)
const isLoadingOptifine = ref(false)

// 兼容性计算
const isFabricSupported = computed(() => {
  if (!configuringVersion.value || !fabricMeta.value) return true
  return fabricMeta.value.supported_games.includes(configuringVersion.value.id)
})

const isOptiFineSupported = computed(() => {
  if (!configuringVersion.value) return false
  return optifineLoaders.value.length > 0 || optifineGames.value.includes(configuringVersion.value.id)
})

// 切换选择 Loader
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

// 清除选择
const clearLoader = () => {
  selectedLoaderType.value = null
  selectedLoaderVer.value = ''
  updateTargetVersionName()
}

// 自动生成版本名算法（完全对齐原版 PCL 命名机制）
const updateTargetVersionName = () => {
  if (!configuringVersion.value || customNameEdited.value) return
  let name = configuringVersion.value.id
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

// 打开版本配置页
const openVersionConfig = async (v: RemoteVersion) => {
  configuringVersion.value = v
  targetVersionName.value = v.id
  customNameEdited.value = false
  selectedLoaderType.value = null
  selectedLoaderVer.value = ''
  openCard.value = null

  // 1. 获取 Fabric 全量元数据 (BMCLAPI + 官方双备，Rust 后端代理)
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

  // 2. 获取 OptiFine 支持版本与当前版本可用清单
  if (optifineGames.value.length === 0) {
    backend.fetchOptifineGames()
      .then((games) => {
        optifineGames.value = games
      })
      .catch(() => {})
  }

  isLoadingOptifine.value = true
  backend.fetchOptifineLoaders(v.id)
    .then((loaders) => {
      optifineLoaders.value = loaders
    })
    .catch(() => {
      optifineLoaders.value = []
    })
    .finally(() => {
      isLoadingOptifine.value = false
    })

  // 3. 并行拉取 Forge 与 NeoForge 当前可用版本
  isLoadingForge.value = true
  backend.fetchForgeLoaders(v.id)
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
  backend.fetchNeoForgeLoaders(v.id)
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

// 检查目标版本名是否冲突
const isNameConflict = computed(() => {
  if (!targetVersionName.value.trim()) return false
  return store.installedVersions.some(
    (iv) => iv.id.toLowerCase() === targetVersionName.value.trim().toLowerCase()
  )
})

// 执行最终安装下载
const startInstallFromConfig = async () => {
  if (!configuringVersion.value || isNameConflict.value) return
  const v = configuringVersion.value
  const targetName = targetVersionName.value.trim() || v.id
  const loaderType = selectedLoaderType.value || undefined
  const loaderVer = selectedLoaderVer.value || undefined

  configuringVersion.value = null // 返回列表并开始任务

  try {
    store.updateDownloadProgress({
      version_id: targetName,
      vanilla_version: v.id,
      step: '正在初始化安装任务...',
      percentage: 5,
      speed_text: '连接中',
      is_finished: false,
    })
    await backend.startInstallVersion({
      gameDir: store.gameDir,
      vanillaVersion: v.id,
      targetName: targetName,
      metaUrl: v.url,
      loaderType: loaderType,
      loaderVersion: loaderVer,
    })
  } catch (err: any) {
    alert(`安装失败: ${err}`)
    store.clearDownloadTask()
  }
}

// ==================== 1.2 另存为 / 更新日志 / 服务端下载 ====================
const changelogModalVersion = ref<RemoteVersion | null>(null)

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

// 映射官方中文 Minecraft Wiki 地址
const getWikiUrl = (id: string, type: string) => {
  const cleanId = id.toLowerCase()
  if (cleanId === '3d shareware v1.34') return 'https://zh.minecraft.wiki/w/3D_Shareware_v1.34'
  if (cleanId === '2.0') return 'https://zh.minecraft.wiki/w/Java版2.0'
  if (type === 'release') return `https://zh.minecraft.wiki/w/Java版${id}`
  if (type === 'snapshot') return `https://zh.minecraft.wiki/w/${id}`
  return `https://zh.minecraft.wiki/w/Java版${id}`
}

// 显示更新日志与版本特性
const handleShowChangelog = (v: RemoteVersion) => {
  changelogModalVersion.value = v
}

// 打开外部 Wiki 链接 (调用系统默认浏览器)
const openWikiLink = async (v: RemoteVersion) => {
  const url = getWikiUrl(v.id, v.type)
  try {
    await backend.openUrl(url)
  } catch (err) {
    console.warn('调用系统浏览器失败，尝试前端打开:', err)
    window.open(url, '_blank')
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

// 下载社区资源 (Mod / 光影 / 材质)
const handleInstallCommunity = async (proj: ModrinthProject) => {
  try {
    const targetVer = store.currentVersion ? store.currentVersion.id.replace(/-.*$/, '') : undefined
    const targetLoader = store.currentVersion?.loader || 'fabric'

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
      versionPath: store.currentVersion?.path,
      projectId: proj.project_id,
      projectType: currentSection.value,
      gameVersion: targetVer,
      loader: currentSection.value === 'mod' ? targetLoader : undefined,
    })

    console.log(msg)
  } catch (err: any) {
    alert(`安装失败: ${err}`)
    store.clearDownloadTask()
  }
}
</script>

<template>
  <div class="h-full flex overflow-hidden relative" :class="store.uiStyle === 'classic' ? 'flex-row gap-0' : 'flex-col gap-3.5'">
    <!-- ==================== 1A. 原版 PCL2 经典左侧 152px 菜单栏 (PageDownloadLeft.xaml) ==================== -->
    <div
      v-if="store.uiStyle === 'classic'"
      class="w-[152px] h-full bg-white dark:bg-[#1e232b] border-r border-black/8 dark:border-white/10 pt-3 flex flex-col shrink-0 select-none"
    >
      <MyListItem
        title="原版游戏"
        icon="vanilla"
        :checked="currentSection === 'game'"
        action-title="刷新版本列表"
        @click="currentSection = 'game'; configuringVersion = null"
        @action="loadRemoteVersions"
      />

      <div class="px-3.5 pt-5 pb-1 text-[12px] text-[#737373] opacity-80">
        社区资源
      </div>

      <MyListItem
        title="Mod"
        icon="mod"
        :checked="currentSection === 'mod'"
        action-title="刷新 Mod 列表"
        @click="currentSection = 'mod'"
        @action="searchCommunity"
      />
      <MyListItem
        title="整合包"
        icon="modpack"
        :checked="currentSection === 'modpack'"
        action-title="刷新整合包列表"
        @click="currentSection = 'modpack'"
        @action="searchCommunity"
      />
      <MyListItem
        title="资源包"
        icon="resourcepack"
        :checked="currentSection === 'resourcepack'"
        action-title="刷新资源包列表"
        @click="currentSection = 'resourcepack'"
        @action="searchCommunity"
      />
      <MyListItem
        title="光影包"
        icon="shader"
        :checked="currentSection === 'shader'"
        action-title="刷新光影包列表"
        @click="currentSection = 'shader'"
        @action="searchCommunity"
      />
    </div>

    <!-- 右侧主内容区容器 -->
    <div
      class="flex-1 h-full flex flex-col gap-3.5 overflow-hidden relative"
      :class="store.uiStyle === 'classic' && 'p-5'"
    >
    <!-- 顶部主分类导航栏 (仅现代模式显示，经典模式由左侧 152px 栏承载) -->
    <div
      v-if="store.uiStyle === 'modern'"
      class="flex items-center justify-between gap-3 shrink-0 pb-1 border-b border-pcl-border/60"
    >
      <!-- 大类切换 Pill 标签 -->
      <div class="flex items-center gap-1 p-1 rounded-2xl bg-black/5 dark:bg-white/5 border border-pcl-border shrink-0 overflow-x-auto no-scrollbar">
        <button
          class="px-3 py-1.5 rounded-xl text-xs font-semibold cursor-pointer transition-all flex items-center gap-1.5 whitespace-nowrap shrink-0"
          :class="currentSection === 'game' ? 'bg-pcl-primary text-white shadow-md shadow-pcl-primary/20' : 'text-pcl-subtext hover:text-pcl-text'"
          @click="currentSection = 'game'"
        >
          <span>💎</span>
          <span>原版游戏</span>
        </button>

        <button
          class="px-3 py-1.5 rounded-xl text-xs font-semibold cursor-pointer transition-all flex items-center gap-1.5 whitespace-nowrap shrink-0"
          :class="currentSection === 'mod' ? 'bg-pcl-primary text-white shadow-md shadow-pcl-primary/20' : 'text-pcl-subtext hover:text-pcl-text'"
          @click="currentSection = 'mod'"
        >
          <span>🧩</span>
          <span>社区Mod</span>
        </button>

        <button
          class="px-3 py-1.5 rounded-xl text-xs font-semibold cursor-pointer transition-all flex items-center gap-1.5 whitespace-nowrap shrink-0"
          :class="currentSection === 'modpack' ? 'bg-pcl-primary text-white shadow-md shadow-pcl-primary/20' : 'text-pcl-subtext hover:text-pcl-text'"
          @click="currentSection = 'modpack'"
        >
          <span>📦</span>
          <span>整合包</span>
        </button>

        <button
          class="px-3 py-1.5 rounded-xl text-xs font-semibold cursor-pointer transition-all flex items-center gap-1.5 whitespace-nowrap shrink-0"
          :class="currentSection === 'shader' ? 'bg-pcl-primary text-white shadow-md shadow-pcl-primary/20' : 'text-pcl-subtext hover:text-pcl-text'"
          @click="currentSection = 'shader'"
        >
          <span>✨</span>
          <span>光影包</span>
        </button>

        <button
          class="px-3 py-1.5 rounded-xl text-xs font-semibold cursor-pointer transition-all flex items-center gap-1.5 whitespace-nowrap shrink-0"
          :class="currentSection === 'resourcepack' ? 'bg-pcl-primary text-white shadow-md shadow-pcl-primary/20' : 'text-pcl-subtext hover:text-pcl-text'"
          @click="currentSection = 'resourcepack'"
        >
          <span>🎨</span>
          <span>材质包</span>
        </button>
      </div>

      <!-- 右侧附加状态提示 -->
      <div class="text-xs text-pcl-subtext flex items-center gap-2 shrink-0 whitespace-nowrap min-w-0">
        <span
          v-if="store.currentVersion"
          class="px-2 py-0.5 rounded-md bg-black/5 dark:bg-white/5 font-mono text-[11px] max-w-[150px] truncate shrink-0"
          :title="`当前版本: ${store.currentVersion.name}`"
        >
          {{ store.currentVersion.name }}
        </span>
        <span class="text-emerald-500 flex items-center gap-1 text-[11px] whitespace-nowrap shrink-0">
          <span class="w-1.5 h-1.5 rounded-full bg-emerald-500 animate-ping shrink-0" />
          <span>Modrinth & BMCLAPI 在线</span>
        </span>
      </div>
    </div>

    <!-- ==================== 视图 A: 官方游戏版本中心 ==================== -->
    <template v-if="currentSection === 'game'">
      <!-- 搜索栏与过滤开关 -->
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
            v-model="gameSearchQuery"
            placeholder="搜索版本号 (如 1.21.4)..."
            class="px-3.5 py-1.5 rounded-xl bg-black/5 dark:bg-white/5 border border-pcl-border text-xs text-pcl-text outline-none focus:border-pcl-primary transition-colors w-36 min-w-[90px]"
          />

          <MyButton type="secondary" size="small" :loading="isLoadingVersions" @click="loadRemoteVersions">
            🔄
          </MyButton>
        </div>
      </div>

      <!-- 视图 A.2: 图 1 版本安装配置详情页 (完全对齐原版 PCL) -->
      <div v-if="configuringVersion" class="flex flex-col gap-4 max-w-2xl mx-auto w-full py-4 flex-1 overflow-y-auto">
        <!-- 顶部返回栏与名称输入框 -->
        <div class="flex items-center gap-3 p-3 rounded-2xl bg-black/5 dark:bg-white/5 border border-pcl-border shadow-sm">
          <button
            class="p-2 rounded-xl hover:bg-black/10 dark:hover:bg-white/10 text-pcl-subtext hover:text-pcl-text transition-colors cursor-pointer text-base"
            title="返回版本列表"
            @click="configuringVersion = null"
          >
            ←
          </button>
          <!-- 经典原版 Minecraft 草方块图标 -->
          <div class="w-8 h-8 rounded-lg bg-emerald-600/20 border border-emerald-500/30 flex items-center justify-center text-lg shrink-0 select-none shadow-inner">
            🟩
          </div>
          <div class="flex-1 flex flex-col">
            <input
              v-model="targetVersionName"
              type="text"
              class="w-full px-3 py-1.5 rounded-xl bg-pcl-card border border-pcl-border text-sm font-semibold text-pcl-text focus:outline-none focus:border-pcl-primary transition-colors"
              placeholder="请输入安装后的版本名称"
              @input="customNameEdited = true"
            />
            <span v-if="isNameConflict" class="text-xs text-rose-500 mt-1 font-medium">
              ⚠️ 版本已存在，请换一个名称或前往启动！
            </span>
          </div>
        </div>

        <!-- 附加组件选项折叠列表 (Forge / NeoForge / Fabric / OptiFine) -->
        <div class="flex flex-col gap-2.5">
          <!-- Forge 卡片 -->
          <div class="rounded-2xl border border-pcl-border bg-black/5 dark:bg-white/5 overflow-hidden transition-all shadow-sm">
            <div
              class="flex items-center justify-between p-3.5 cursor-pointer hover:bg-black/5 dark:hover:bg-white/5 select-none"
              @click="openCard = openCard === 'forge' ? null : 'forge'"
            >
              <div class="flex items-center gap-2.5 font-bold text-sm text-pcl-text">
                <span>Forge</span>
              </div>
              <div class="flex items-center gap-2 text-xs text-pcl-subtext">
                <span v-if="selectedLoaderType === 'forge'" class="text-pcl-primary font-bold">
                  {{ selectedLoaderVer }}
                </span>
                <span v-else-if="isLoadingForge">查询中...</span>
                <span v-else>{{ forgeLoaders.length > 0 ? '可以添加' : '无' }}</span>
                <span class="text-sm transition-transform duration-200" :class="openCard === 'forge' ? 'rotate-90' : ''">›</span>
              </div>
            </div>
            <div v-if="openCard === 'forge'" class="p-3 border-t border-pcl-border/50 bg-black/5 dark:bg-white/5 flex flex-col gap-2 text-xs">
              <div class="flex items-center justify-between">
                <span class="text-pcl-subtext">选择 Forge 版本（与 Fabric 互斥）：</span>
                <button
                  v-if="selectedLoaderType === 'forge'"
                  class="text-rose-500 hover:underline cursor-pointer"
                  @click="clearLoader"
                >
                  清除选择
                </button>
              </div>
              <div v-if="isLoadingForge" class="py-2 text-pcl-subtext flex items-center gap-2">
                <span class="animate-spin">⏳</span> 正在查询可用的 Forge 版本...
              </div>
              <div v-else-if="forgeLoaders.length === 0" class="py-2 text-pcl-subtext">
                暂未找到匹配该 Minecraft 版本的 Forge
              </div>
              <div v-else class="max-h-56 overflow-y-auto flex flex-col gap-1 pr-1">
                <div
                  v-for="item in forgeLoaders"
                  :key="item.version"
                  class="flex items-center gap-3 px-3 py-2 rounded-xl cursor-pointer transition-colors"
                  :class="selectedLoaderType === 'forge' && selectedLoaderVer === item.version
                    ? 'bg-pcl-primary/15 text-pcl-primary font-semibold border border-pcl-primary/30'
                    : 'hover:bg-black/5 dark:hover:bg-white/5 text-pcl-text'"
                  @click="selectLoader('forge', item.version)"
                >
                  <div class="text-lg shrink-0 opacity-80">📜</div>
                  <div class="flex flex-col flex-1 leading-tight">
                    <span class="text-xs font-bold font-mono">{{ item.version }}</span>
                    <span class="text-[10px] text-pcl-subtext mt-0.5">{{ item.description || '正式版' }}</span>
                  </div>
                  <div v-if="selectedLoaderType === 'forge' && selectedLoaderVer === item.version" class="text-pcl-primary text-xs font-bold">
                    ✓
                  </div>
                </div>
              </div>
            </div>
          </div>

          <!-- NeoForge 卡片 -->
          <div class="rounded-2xl border border-pcl-border bg-black/5 dark:bg-white/5 overflow-hidden transition-all shadow-sm">
            <div
              class="flex items-center justify-between p-3.5 cursor-pointer hover:bg-black/5 dark:hover:bg-white/5 select-none"
              @click="openCard = openCard === 'neoforge' ? null : 'neoforge'"
            >
              <div class="flex items-center gap-2.5 font-bold text-sm text-pcl-text">
                <span>NeoForge</span>
              </div>
              <div class="flex items-center gap-2 text-xs text-pcl-subtext">
                <span v-if="selectedLoaderType === 'neoforge'" class="text-pcl-primary font-bold">
                  {{ selectedLoaderVer }}
                </span>
                <span v-else-if="isLoadingNeoForge">查询中...</span>
                <span v-else>{{ neoForgeLoaders.length > 0 ? '可以添加' : '无' }}</span>
                <span class="text-sm transition-transform duration-200" :class="openCard === 'neoforge' ? 'rotate-90' : ''">›</span>
              </div>
            </div>
            <div v-if="openCard === 'neoforge'" class="p-3 border-t border-pcl-border/50 bg-black/5 dark:bg-white/5 flex flex-col gap-2 text-xs">
              <div class="flex items-center justify-between">
                <span class="text-pcl-subtext">选择 NeoForge 版本：</span>
                <button
                  v-if="selectedLoaderType === 'neoforge'"
                  class="text-rose-500 hover:underline cursor-pointer"
                  @click="clearLoader"
                >
                  清除选择
                </button>
              </div>
              <div v-if="isLoadingNeoForge" class="py-2 text-pcl-subtext flex items-center gap-2">
                <span class="animate-spin">⏳</span> 正在查询可用的 NeoForge 版本...
              </div>
              <div v-else-if="neoForgeLoaders.length === 0" class="py-2 text-pcl-subtext">
                暂未找到匹配该 Minecraft 版本的 NeoForge
              </div>
              <div v-else class="max-h-56 overflow-y-auto flex flex-col gap-1 pr-1">
                <div
                  v-for="item in neoForgeLoaders"
                  :key="item.version"
                  class="flex items-center gap-3 px-3 py-2 rounded-xl cursor-pointer transition-colors"
                  :class="selectedLoaderType === 'neoforge' && selectedLoaderVer === item.version
                    ? 'bg-pcl-primary/15 text-pcl-primary font-semibold border border-pcl-primary/30'
                    : 'hover:bg-black/5 dark:hover:bg-white/5 text-pcl-text'"
                  @click="selectLoader('neoforge', item.version)"
                >
                  <div class="text-lg shrink-0 opacity-80">📜</div>
                  <div class="flex flex-col flex-1 leading-tight">
                    <span class="text-xs font-bold font-mono">{{ item.version }}</span>
                    <span class="text-[10px] text-pcl-subtext mt-0.5">{{ item.description || (item.stable ? '稳定版' : '测试版') }}</span>
                  </div>
                  <div v-if="selectedLoaderType === 'neoforge' && selectedLoaderVer === item.version" class="text-pcl-primary text-xs font-bold">
                    ✓
                  </div>
                </div>
              </div>
            </div>
          </div>

          <!-- Fabric 卡片 (对齐原版 PCL 图 4 界面与全量列表渲染) -->
          <div class="rounded-2xl border border-pcl-border bg-black/5 dark:bg-white/5 overflow-hidden transition-all shadow-sm">
            <div
              class="flex items-center justify-between p-3.5 cursor-pointer hover:bg-black/5 dark:hover:bg-white/5 select-none"
              @click="openCard = openCard === 'fabric' ? null : 'fabric'"
            >
              <div class="flex items-center gap-2.5 font-bold text-sm text-pcl-text">
                <span>Fabric</span>
              </div>
              <div class="flex items-center gap-2 text-xs text-pcl-subtext">
                <span v-if="selectedLoaderType === 'fabric'" class="text-pcl-primary font-bold">
                  {{ selectedLoaderVer }}
                </span>
                <span v-else-if="isLoadingFabric">查询中...</span>
                <span v-else>{{ isFabricSupported ? '可以添加' : '无' }}</span>
                <span class="text-sm transition-transform duration-200" :class="openCard === 'fabric' ? 'rotate-90' : ''">›</span>
              </div>
            </div>
            <div v-if="openCard === 'fabric'" class="p-3 border-t border-pcl-border/50 bg-black/5 dark:bg-white/5 flex flex-col gap-2 text-xs">
              <div class="flex items-center justify-between">
                <span class="text-pcl-subtext">选择 Fabric Loader 版本：</span>
                <button
                  v-if="selectedLoaderType === 'fabric'"
                  class="text-rose-500 hover:underline cursor-pointer font-medium"
                  @click="clearLoader"
                >
                  清除选择
                </button>
              </div>
              <div v-if="isLoadingFabric && !fabricMeta" class="py-3 text-pcl-subtext flex items-center justify-center gap-2">
                <span class="animate-spin">⏳</span> 正在加载 Fabric 加载器元数据...
              </div>
              <div v-else-if="!isFabricSupported" class="py-3 text-pcl-subtext text-center">
                当前 Minecraft 版本暂不受 Fabric 支持
              </div>
              <div v-else class="max-h-60 overflow-y-auto flex flex-col gap-1 pr-1">
                <!-- 还原原版 PCL 图 4 的列表项与卷轴图标 -->
                <div
                  v-for="item in (fabricMeta?.loaders || [])"
                  :key="item.version"
                  class="flex items-center gap-3 px-3.5 py-2.5 rounded-xl cursor-pointer transition-colors"
                  :class="selectedLoaderType === 'fabric' && selectedLoaderVer === item.version
                    ? 'bg-pcl-primary/20 text-pcl-primary font-bold border border-pcl-primary/30'
                    : 'hover:bg-black/5 dark:hover:bg-white/10 text-pcl-text'"
                  @click="selectLoader('fabric', item.version)"
                >
                  <div class="text-lg shrink-0 opacity-80">📜</div>
                  <div class="flex flex-col flex-1 leading-tight">
                    <span class="text-xs font-bold font-mono">{{ item.version }}</span>
                    <span class="text-[10px] text-pcl-subtext mt-0.5">
                      {{ item.description || (item.stable ? '稳定版' : '测试版') }}
                    </span>
                  </div>
                  <div v-if="selectedLoaderType === 'fabric' && selectedLoaderVer === item.version" class="text-pcl-primary text-xs font-bold">
                    ✓
                  </div>
                </div>
              </div>
            </div>
          </div>

          <!-- OptiFine 卡片 -->
          <div class="rounded-2xl border border-pcl-border bg-black/5 dark:bg-white/5 overflow-hidden transition-all shadow-sm">
            <div
              class="flex items-center justify-between p-3.5 cursor-pointer hover:bg-black/5 dark:hover:bg-white/5 select-none"
              :class="isOptiFineSupported ? '' : 'opacity-70'"
              @click="openCard = openCard === 'optifine' ? null : 'optifine'"
            >
              <div class="flex items-center gap-2.5 font-bold text-sm text-pcl-text">
                <span>OptiFine</span>
              </div>
              <div class="flex items-center gap-2 text-xs text-pcl-subtext">
                <span v-if="selectedLoaderType === 'optifine'" class="text-pcl-primary font-bold">
                  {{ selectedLoaderVer }}
                </span>
                <span v-else-if="isLoadingOptifine">查询中...</span>
                <span v-else>{{ isOptiFineSupported ? `可以添加 (${optifineLoaders.length})` : '无' }}</span>
                <span class="text-sm transition-transform duration-200" :class="openCard === 'optifine' ? 'rotate-90' : ''">›</span>
              </div>
            </div>
            <div v-if="openCard === 'optifine'" class="p-3 border-t border-pcl-border/50 bg-black/5 dark:bg-white/5 flex flex-col gap-2 text-xs">
              <div class="flex items-center justify-between">
                <span class="text-pcl-subtext">选择 OptiFine 高清修复版本：</span>
                <button
                  v-if="selectedLoaderType === 'optifine'"
                  class="text-rose-500 hover:underline cursor-pointer font-medium"
                  @click="clearLoader"
                >
                  清除选择
                </button>
              </div>
              <div v-if="isLoadingOptifine" class="py-3 text-pcl-subtext flex items-center justify-center gap-2">
                <span class="animate-spin">⏳</span> 正在加载 OptiFine 版本清单...
              </div>
              <div v-else-if="optifineLoaders.length === 0" class="py-3 text-pcl-subtext text-center">
                当前 Minecraft 版本暂无可用 OptiFine
              </div>
              <div v-else class="max-h-60 overflow-y-auto flex flex-col gap-1 pr-1">
                <div
                  v-for="item in optifineLoaders"
                  :key="item.version"
                  class="flex items-center gap-3 px-3.5 py-2.5 rounded-xl cursor-pointer transition-colors"
                  :class="selectedLoaderType === 'optifine' && selectedLoaderVer === item.version
                    ? 'bg-pcl-primary/20 text-pcl-primary font-bold border border-pcl-primary/30'
                    : 'hover:bg-black/5 dark:hover:bg-white/10 text-pcl-text'"
                  @click="selectLoader('optifine', item.version)"
                >
                  <div class="text-lg shrink-0 opacity-80">✨</div>
                  <div class="flex flex-col flex-1 leading-tight">
                    <span class="text-xs font-bold font-mono">{{ item.version }}</span>
                    <span class="text-[10px] text-pcl-subtext mt-0.5">
                      {{ item.description || (item.stable ? '正式版' : '预览版') }}
                    </span>
                  </div>
                  <div v-if="selectedLoaderType === 'optifine' && selectedLoaderVer === item.version" class="text-pcl-primary text-xs font-bold">
                    ✓
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- 底部胶囊按钮 (对齐原版 PCL 图 4：居中深灰蓝/暗色胶囊按钮) -->
        <div class="sticky bottom-1 flex items-center justify-center pt-3 pb-2 select-none">
          <button
            :disabled="isNameConflict || store.isDownloading"
            class="px-8 py-2.5 rounded-2xl text-xs font-bold cursor-pointer bg-slate-700 hover:bg-slate-600 active:bg-slate-800 text-white shadow-xl transition-all duration-200 active:scale-95 disabled:opacity-40 disabled:cursor-not-allowed flex items-center gap-2"
            @click="startInstallFromConfig"
          >
            <span class="text-sm">{{ store.isDownloading ? '⏳' : '📥' }}</span>
            <span>{{ store.isDownloading ? `正在下载 (${store.downloadTask?.percentage.toFixed(0)}%)` : '开始下载' }}</span>
          </button>
        </div>
      </div>

      <!-- 版本列表视口 (当未处于配置状态时展示) -->
      <div v-else class="flex-1 overflow-y-auto pr-1 space-y-2">
        <div v-if="isLoadingVersions && versions.length === 0" class="h-64 flex flex-col items-center justify-center text-pcl-subtext text-xs gap-3">
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

            <!-- 操作按钮与拓展工具 (对齐原版 PCL: 另存为 / 更新日志 / 下载服务端) -->
            <div class="shrink-0 ml-4 flex items-center gap-2">
              <!-- 原版 3 个扩展图标 -->
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
                  @click.stop="handleShowChangelog(v)"
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
                  @click="openVersionConfig(v)"
                >
                  🔄
                </button>
              </template>
              <template v-else>
                <button
                  :disabled="store.isDownloading"
                  class="px-4 py-2 rounded-xl text-xs font-semibold cursor-pointer bg-pcl-primary hover:bg-pcl-hover active:bg-pcl-active text-white shadow-sm transition-all duration-200 ease-[var(--pcl-ease-bounce)] active:scale-95 disabled:opacity-50"
                  @click="openVersionConfig(v)"
                >
                  📥 安装
                </button>
              </template>
            </div>
          </div>

          <!-- 卡片内联微进度条 -->
          <div v-if="isVersionDownloading(v.id) && store.downloadTask" class="mt-3 pt-2 border-t border-pcl-border/40 flex flex-col gap-1.5">
            <div class="w-full h-1.5 rounded-full bg-black/5 dark:bg-white/10 overflow-hidden">
              <div class="h-full bg-pcl-primary transition-all duration-300 ease-out rounded-full" :style="{ width: `${store.downloadTask.percentage}%` }" />
            </div>
            <div class="text-[11px] text-pcl-primary flex items-center justify-between">
              <span>{{ store.downloadTask.step }}</span>
              <span>{{ store.downloadTask.speed_text }}</span>
            </div>
          </div>
        </div>
      </div>
    </template>

    <!-- ==================== 视图 B: Modrinth 社区资源 (Mod/光影/材质) ==================== -->
    <template v-else>
      <!-- 搜索栏与过滤工具 -->
      <div class="flex items-center justify-between gap-3 shrink-0 flex-wrap">
        <div class="flex items-center gap-2 shrink-0 flex-wrap">
          <!-- 针对当前版本匹配开关 -->
          <label class="flex items-center gap-1.5 px-2.5 py-1.5 rounded-xl bg-black/5 dark:bg-white/5 border border-pcl-border text-xs text-pcl-text cursor-pointer hover:border-pcl-primary select-none whitespace-nowrap shrink-0">
            <input v-model="filterByCurrentVersion" type="checkbox" class="accent-[var(--pcl-color-primary)] cursor-pointer shrink-0" />
            <span class="font-medium text-[11px] whitespace-nowrap">匹配当前版本</span>
            <span
              v-if="store.currentVersion"
              class="text-[10px] text-pcl-subtext max-w-[110px] truncate font-mono"
              :title="store.currentVersion.name"
            >
              ({{ store.currentVersion.name }})
            </span>
          </label>

          <!-- Mod Loader 选择器 (仅在 Mod 分类时显示) -->
          <div v-if="currentSection === 'mod'" class="flex items-center p-1 rounded-xl bg-black/5 dark:bg-white/5 border border-pcl-border text-xs shrink-0 whitespace-nowrap">
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

        <!-- 关键词搜索框 -->
        <div class="flex items-center gap-2 flex-1 max-w-sm justify-end min-w-0">
          <input
            v-model="communitySearchQuery"
            :placeholder="`在 Modrinth 搜索${currentSection === 'mod' ? '模组 (如 Sodium, JEI)' : currentSection === 'shader' ? '光影 (如 Complementary)' : '材质包'}...`"
            class="px-3.5 py-1.5 rounded-xl bg-black/5 dark:bg-white/5 border border-pcl-border text-xs text-pcl-text outline-none focus:border-pcl-primary transition-colors flex-1 min-w-[120px] max-w-xs"
            @keyup.enter="searchCommunity"
          />
          <MyButton type="primary" size="small" :loading="isLoadingCommunity" @click="searchCommunity">
            🔍 搜索
          </MyButton>
        </div>
      </div>

      <!-- 社区资源卡片列表视口 -->
      <div class="flex-1 overflow-y-auto pr-1 space-y-2.5">
        <div v-if="isLoadingCommunity" class="h-64 flex flex-col items-center justify-center text-pcl-subtext text-xs gap-3">
          <span class="animate-spin text-2xl">⏳</span>
          <span>正在连接 Modrinth 全球社区检索资源...</span>
        </div>

        <div v-else-if="communityProjects.length === 0" class="h-64 flex flex-col items-center justify-center text-pcl-subtext text-xs gap-2">
          <span class="text-3xl">🧩</span>
          <span>输入关键词并按回车搜索，或取消勾选“匹配当前版本”查看全量热门资源</span>
        </div>

        <div
          v-for="p in communityProjects"
          :key="p.project_id"
          class="flex flex-col p-4 rounded-2xl border transition-all duration-200 hover:shadow-md hover:-translate-y-0.5"
          :class="[
            store.downloadTask?.version_id === p.project_id
              ? 'border-pcl-primary bg-pcl-primary/5 ring-2 ring-pcl-primary/20'
              : 'border-pcl-border bg-pcl-card shadow-sm'
          ]"
        >
          <div class="flex items-start justify-between gap-4">
            <!-- 图标与核心信息 -->
            <div class="flex items-start gap-3.5 overflow-hidden flex-1">
              <img
                v-if="p.icon_url"
                :src="p.icon_url"
                class="w-12 h-12 rounded-xl object-cover shrink-0 border border-pcl-border/50 bg-black/5"
                alt="icon"
              />
              <div
                v-else
                class="w-12 h-12 rounded-xl bg-pcl-primary/10 text-pcl-primary flex items-center justify-center text-2xl shrink-0"
              >
                {{ currentSection === 'mod' ? '🧩' : currentSection === 'shader' ? '✨' : '🎨' }}
              </div>

              <div class="flex-1 min-w-0">
                <div class="font-bold text-sm text-pcl-text flex items-center gap-2">
                  <span class="truncate">{{ p.title }}</span>
                  <span class="text-[10px] text-pcl-subtext font-normal">by {{ p.author }}</span>
                  <span class="text-[10px] px-1.5 py-0.2 rounded bg-black/5 dark:bg-white/10 text-pcl-subtext font-mono">
                    {{ formatDownloads(p.downloads) }} 下载
                  </span>
                </div>

                <p class="text-xs text-pcl-subtext mt-1 line-clamp-2 leading-relaxed">
                  {{ p.description }}
                </p>

                <!-- 分类标签 -->
                <div class="flex flex-wrap gap-1 mt-2">
                  <span
                    v-for="cat in p.categories.slice(0, 5)"
                    :key="cat"
                    class="text-[10px] px-2 py-0.5 rounded-md bg-black/5 dark:bg-white/5 text-pcl-subtext font-medium"
                  >
                    #{{ cat }}
                  </span>
                </div>
              </div>
            </div>

            <!-- 右侧一键安装按钮 -->
            <div class="shrink-0 flex items-center">
              <div
                v-if="store.downloadTask?.version_id === p.project_id"
                class="flex items-center gap-2 text-xs font-bold text-pcl-primary px-3 py-1.5 rounded-xl bg-pcl-primary/10 animate-pulse"
              >
                <span class="animate-spin text-sm">⏳</span>
                <span>{{ store.downloadTask.percentage.toFixed(0) }}% {{ store.downloadTask.speed_text }}</span>
              </div>

              <button
                v-else
                :disabled="store.isDownloading"
                class="px-4 py-2 rounded-xl text-xs font-semibold cursor-pointer bg-pcl-primary hover:bg-pcl-hover active:bg-pcl-active text-white shadow-sm transition-all duration-200 ease-[var(--pcl-ease-bounce)] active:scale-95 disabled:opacity-50 whitespace-nowrap shrink-0"
                @click="handleInstallCommunity(p)"
              >
                📥 一键安装到 {{ currentSection === 'mod' ? 'Mods' : currentSection === 'shader' ? 'Shaders' : 'Textures' }}
              </button>
            </div>
          </div>

          <!-- 卡片内联微进度条 -->
          <div v-if="store.downloadTask?.version_id === p.project_id" class="mt-3 pt-2 border-t border-pcl-border/40 flex flex-col gap-1.5">
            <div class="w-full h-1.5 rounded-full bg-black/5 dark:bg-white/10 overflow-hidden">
              <div class="h-full bg-pcl-primary transition-all duration-300 ease-out rounded-full" :style="{ width: `${store.downloadTask.percentage}%` }" />
            </div>
            <div class="text-[11px] text-pcl-primary flex items-center justify-between">
              <span>{{ store.downloadTask.step }}</span>
              <span>{{ store.downloadTask.speed_text }}</span>
            </div>
          </div>
        </div>
      </div>
    </template>

    <!-- 模态框: Minecraft 官方版本更新日志 (对齐原版 PCL 详情与中文 Wiki 联动) -->
    <div
      v-if="changelogModalVersion"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4 animate-fade-in"
      @click.self="changelogModalVersion = null"
    >
      <div class="max-w-md w-full bg-pcl-card border border-pcl-border rounded-2xl shadow-2xl p-6 flex flex-col gap-4">
        <div class="flex items-center justify-between pb-3 border-b border-pcl-border/50">
          <div class="flex items-center gap-3">
            <span class="text-2xl">ℹ️</span>
            <div>
              <h3 class="font-bold text-base text-pcl-text">Minecraft {{ changelogModalVersion.id }}</h3>
              <p class="text-xs text-pcl-subtext">官方版本更新说明与元数据</p>
            </div>
          </div>
          <button
            class="p-2 rounded-xl text-pcl-subtext hover:text-pcl-text hover:bg-black/5 dark:hover:bg-white/10 transition-colors cursor-pointer text-sm"
            @click="changelogModalVersion = null"
          >
            ✕
          </button>
        </div>

        <div class="flex flex-col gap-2.5 text-xs text-pcl-text">
          <div class="flex justify-between py-1.5 border-b border-pcl-border/30">
            <span class="text-pcl-subtext">版本编号</span>
            <span class="font-mono font-bold">{{ changelogModalVersion.id }}</span>
          </div>
          <div class="flex justify-between py-1.5 border-b border-pcl-border/30">
            <span class="text-pcl-subtext">版本类型</span>
            <span>{{ changelogModalVersion.type === 'release' ? '正式版 (Release)' : '快照/测试版 (Snapshot)' }}</span>
          </div>
          <div class="flex justify-between py-1.5 border-b border-pcl-border/30">
            <span class="text-pcl-subtext">发布日期</span>
            <span>{{ formatDate(changelogModalVersion.releaseTime) }}</span>
          </div>
          <div class="flex justify-between py-1.5 border-b border-pcl-border/30">
            <span class="text-pcl-subtext">SHA-1 校验指纹</span>
            <span class="font-mono text-[11px] select-all truncate max-w-48">{{ changelogModalVersion.sha1 }}</span>
          </div>
        </div>

        <div class="p-3 rounded-xl bg-black/5 dark:bg-white/5 border border-pcl-border/50 text-xs text-pcl-subtext leading-relaxed">
          💡 点击下方按钮可在浏览器中直达官方中文 Minecraft Wiki，查看该版本完整的新增方块、实体、物品合成及修复日志。
        </div>

        <div class="flex items-center justify-end gap-3 pt-2">
          <button
            class="px-4 py-2 rounded-xl text-xs font-medium text-pcl-subtext hover:text-pcl-text hover:bg-black/5 dark:hover:bg-white/10 cursor-pointer"
            @click="changelogModalVersion = null"
          >
            关闭
          </button>
          <button
            class="px-4 py-2 rounded-xl text-xs font-bold text-white bg-pcl-primary hover:bg-pcl-hover cursor-pointer shadow-sm flex items-center gap-1.5"
            @click="openWikiLink(changelogModalVersion)"
          >
            <span>📖 中文 Wiki 专页</span>
            <span>↗</span>
          </button>
        </div>
      </div>
    </div>
    </div>
  </div>
</template>