<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useAppStore } from '@/stores/app'
import MyCard from '@/components/base/MyCard.vue'
import MyButton from '@/components/base/MyButton.vue'
import MyListItem from '@/components/base/MyListItem.vue'
import PclIcon from '@/components/base/PclIcon.vue'
import { backend } from '@/services/backend'
import type { EndpointPingResult, CacheCleanResult } from '@/services/backend'

const store = useAppStore()
const otherSubTab = ref<'all' | 'help' | 'test'>('all')
const pingResults = ref<EndpointPingResult[]>([])
const isPinging = ref(false)
const isCleaning = ref(false)
const cleanResult = ref<CacheCleanResult | null>(null)
const searchQuery = ref('')

interface HelpArticle {
  id: string
  category: string
  title: string
  summary: string
  solution: string
}

const helpArticles: HelpArticle[] = [
  {
    id: 'java-version',
    category: '运行环境',
    title: '不同 Minecraft 版本应该选择哪个版本的 Java？',
    summary: 'UnsupportedClassVersionError / ClassFileVersion 报错速查对照表',
    solution:
      '• Minecraft 1.20.5 及以上（含 1.21+）：必须使用 Java 21 或更高版本（64 位）\n• Minecraft 1.18 ~ 1.20.4：推荐使用 Java 17\n• Minecraft 1.17.x：需要使用 Java 16 或 Java 17\n• Minecraft 1.16.5 及以下老版本（如 1.12.2 / 1.7.10）：强烈建议使用 Java 8（1.8.0），高版本 Java 会导致老版 Forge 反射报错崩溃。',
  },
  {
    id: 'mod-conflict',
    category: '模组排错',
    title: '安装 Mod 后游戏启动即崩溃（Mixin / DuplicateMod）怎么办？',
    summary: '快速定位冲突模组与缺失前置库的方法',
    solution:
      '1. 检查是否缺少必装前置库：Fabric 端几乎所有模组都需要安装【Fabric API】；许多模组还需要 Architectury API、Cloth Config 或 GeckoLib。\n2. 检查渲染优化模组冲突：Sodium（钠）不能与 OptiFine 同时安装；在高版本中需要搭配 Indium 才能兼容部分渲染类模组。\n3. 遇到 DuplicateModsFoundException：说明 mods 文件夹中存在两个相同 Mod ID 的不同版本 jar 文件，进入「版本管理」删掉旧版即可。',
  },
  {
    id: 'macos-arm64',
    category: '跨平台支持',
    title: '在 Apple Silicon (M1/M2/M3/M4) Mac 上如何获得最佳帧率？',
    summary: '原生 arm64 JDK 与 natives-macos-arm64 自动匹配说明',
    solution:
      '本启动器已原生适配 macOS arm64 架构：只需安装 Apple Silicon 原生版本的 OpenJDK（如通过 Homebrew 安装或 Azul Zulu ARM64 JDK），启动器会自动按 Mojang 规则选择 `natives-macos-arm64` 动态库并注入 `-XstartOnFirstThread` 参数，无需 Rosetta 转译即可满血运行。',
  },
  {
    id: 'auth-yggdrasil',
    category: '账号与皮肤',
    title: '如何使用微软正版、LittleSkin 外置登录与离线模式？',
    summary: '三种登录模式的切换与凭据安全保存机制',
    solution:
      '• 微软正版登录：在启动页选择「微软登录」，复制 8 位设备码并在浏览器授权即可完成 Xbox Live -> XSTS -> Minecraft Profile 全链路认证。\n• 外置登录 (LittleSkin)：选择「外置登录」，输入皮肤站邮箱与密码（验证服务器地址留空即默认使用 LittleSkin 官方 Yggdrasil 接口）。\n• 离线模式：未登录正版时，直接修改离线昵称即可进入游戏。',
  },
  {
    id: 'memory-fps',
    category: '性能优化',
    title: '游戏内存分配越大越好吗？为什么分配 16GB 反而会周期性卡顿？',
    summary: 'JVM 垃圾回收 (GC) 与合理堆内存上限建议',
    solution:
      '并非越大越好！JVM 堆内存分配过大（如超过 12GB）会导致垃圾回收（Full GC）单次停顿时间变长，表现为每隔几秒突然掉帧卡死一下。\n• 原版或轻量优化包：推荐分配 2048 MB ~ 4096 MB\n• 中大型模组与光影：推荐分配 6144 MB ~ 8192 MB\n• 超大型 300+ Mod 整合包：推荐分配 8192 MB ~ 10240 MB。',
  },
]

const filteredArticles = computed(() => {
  const q = searchQuery.value.trim().toLowerCase()
  if (!q) return helpArticles
  return helpArticles.filter(
    (a) =>
      a.title.toLowerCase().includes(q) ||
      a.summary.toLowerCase().includes(q) ||
      a.solution.toLowerCase().includes(q) ||
      a.category.toLowerCase().includes(q)
  )
})

const runPingTest = async () => {
  isPinging.value = true
  try {
    pingResults.value = await backend.pingDownloadEndpoints()
  } finally {
    isPinging.value = false
  }
}

const handleCleanCache = async () => {
  if (!store.gameDir) return
  isCleaning.value = true
  try {
    cleanResult.value = await backend.cleanGameTempCache(store.gameDir)
  } catch (err: any) {
    alert(`清理缓存失败: ${err}`)
  } finally {
    isCleaning.value = false
  }
}

const formatBytes = (bytes: number) => {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / 1024 / 1024).toFixed(2)} MB`
}

onMounted(() => {
  runPingTest()
})
</script>

<template>
  <div class="h-full flex overflow-hidden" :class="store.uiStyle === 'classic' ? 'flex-row gap-0' : 'flex-col gap-4'">
    <div
      v-if="store.uiStyle === 'classic'"
      class="w-[152px] h-full bg-white dark:bg-[#1e232b] border-r border-black/8 dark:border-white/10 pt-3 flex flex-col shrink-0 select-none"
    >
      <MyListItem
        title="全部工具"
        icon="more"
        :checked="otherSubTab === 'all'"
        @click="otherSubTab = 'all'"
      />
      <MyListItem
        title="帮助百科"
        icon="datapack"
        :checked="otherSubTab === 'help'"
        @click="otherSubTab = 'help'"
      />
      <MyListItem
        title="百宝箱"
        icon="setup"
        :checked="otherSubTab === 'test'"
        action-title="重新测速"
        @click="otherSubTab = 'test'"
        @action="runPingTest"
      />
    </div>

    <div
      class="flex-1 h-full flex flex-col gap-4 overflow-y-auto"
      :class="store.uiStyle === 'classic' ? 'p-5' : 'pr-1'"
    >
    <div class="flex items-center justify-between gap-4 shrink-0">
      <div class="flex items-center gap-3">
        <div class="w-9 h-9 rounded-lg bg-black/5 dark:bg-white/5 border border-pcl-border text-pcl-primary flex items-center justify-center shrink-0">
          <PclIcon name="more" :size="18" />
        </div>
        <div>
          <h2 class="text-sm font-semibold text-pcl-text">百宝箱与疑难百科</h2>
          <p class="text-xs text-pcl-subtext mt-0.5">
            下载源节点实时测速、游戏临时缓存清理与离线排障知识百科
          </p>
        </div>
      </div>

      <div class="flex items-center gap-2">
        <MyButton type="secondary" size="small" :loading="isCleaning" @click="handleCleanCache">
          清理临时缓存
        </MyButton>
        <MyButton type="primary" size="small" :loading="isPinging" @click="runPingTest">
          <PclIcon name="refresh" :size="12" class="mr-1" />
          重新测速
        </MyButton>
      </div>
    </div>

    <div
      v-if="cleanResult"
      class="px-3.5 py-2 rounded-lg bg-emerald-500/10 border border-emerald-500/20 text-xs text-emerald-700 dark:text-emerald-300 flex items-center justify-between"
    >
      <span class="flex items-center gap-1.5">
        <PclIcon name="check" :size="13" />
        <span>已清理 <b>{{ cleanResult.removed_items }}</b> 项临时目录与日志，释放空间 <b>{{ formatBytes(cleanResult.freed_bytes) }}</b>。</span>
      </span>
      <button class="underline cursor-pointer ml-2 text-xs" @click="cleanResult = null">关闭</button>
    </div>

    <MyCard
      v-show="store.uiStyle === 'modern' || otherSubTab === 'all' || otherSubTab === 'test'"
      title="官方源与镜像源网络测速"
      subtitle="实时测量当前网络到各大下载节点的 HTTP 响应延迟"
    >
      <div class="grid grid-cols-3 gap-2.5">
        <div
          v-for="item in pingResults"
          :key="item.name"
          class="p-3 rounded-lg border border-pcl-border bg-black/5 dark:bg-white/5 flex flex-col justify-between gap-1.5"
        >
          <div class="flex items-center justify-between">
            <span class="text-[10px] px-1.5 py-0.5 rounded bg-black/5 dark:bg-white/10 text-pcl-subtext font-mono">
              {{ item.category }}
            </span>
            <span
              class="text-xs font-mono font-medium"
              :class="[
                !item.status_ok
                  ? 'text-red-500'
                  : (item.latency_ms ?? 9999) < 300
                    ? 'text-emerald-600 dark:text-emerald-400'
                    : (item.latency_ms ?? 9999) < 800
                      ? 'text-amber-500'
                      : 'text-orange-500'
              ]"
            >
              {{ item.status_ok && item.latency_ms !== undefined ? `${item.latency_ms} ms` : '不可达' }}
            </span>
          </div>
          <div class="text-xs font-medium text-pcl-text truncate">{{ item.name }}</div>
        </div>
      </div>
    </MyCard>

    <MyCard
      v-show="store.uiStyle === 'modern' || otherSubTab === 'all' || otherSubTab === 'help'"
      title="疑难杂症排障百科"
      subtitle="内置离线排障词条，支持关键字即时检索"
    >
      <template #extra>
        <input
          v-model="searchQuery"
          placeholder="搜索排障知识库..."
          class="w-56 px-3 py-1 rounded-md bg-black/5 dark:bg-white/5 border border-pcl-border text-xs text-pcl-text outline-none focus:border-pcl-primary"
        />
      </template>

      <div class="space-y-3">
        <div
          v-for="art in filteredArticles"
          :key="art.id"
          class="p-3.5 rounded-xl border border-pcl-border bg-black/5 dark:bg-white/5 space-y-1.5"
        >
          <div class="flex items-center gap-2">
            <span class="text-[10px] px-2 py-0.5 rounded bg-pcl-primary text-white font-semibold">
              {{ art.category }}
            </span>
            <h4 class="text-xs font-bold text-pcl-text">{{ art.title }}</h4>
          </div>
          <p class="text-[11px] text-pcl-subtext">{{ art.summary }}</p>
          <pre
            class="text-xs text-pcl-text whitespace-pre-wrap font-sans bg-black/5 dark:bg-white/5 p-2.5 rounded-lg border border-pcl-border/60 leading-relaxed"
          >{{ art.solution }}</pre>
        </div>
      </div>
    </MyCard>
    </div>
  </div>
</template>
