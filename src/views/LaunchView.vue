<script setup lang="ts">
import { useAppStore } from '@/stores/app'
import { storeToRefs } from 'pinia'
import MyCard from '@/components/base/MyCard.vue'
import MyButton from '@/components/base/MyButton.vue'
import MyHint from '@/components/base/MyHint.vue'
import PclIcon from '@/components/base/PclIcon.vue'
import LoginModal from '@/components/auth/LoginModal.vue'
import VersionPicker from '@/components/launch/VersionPicker.vue'
import { backend } from '@/services/backend'
import type { JavaInfo } from '@/services/backend'
import { ref, onMounted, onUnmounted, watch } from 'vue'

const store = useAppStore()
const { isLaunching, launchProgress, runningPid } = storeToRefs(store)
const javaList = ref<JavaInfo[]>([])
const selectedJava = ref<JavaInfo | null>(null)
const isScanning = ref(false)
const showVersionPicker = ref(false)
const launchMessage = ref<{ type: 'success' | 'error' | 'info'; text: string } | null>(null)
let launchMessageTimer: ReturnType<typeof setTimeout> | null = null

const showLaunchMessage = (
  msg: { type: 'success' | 'error' | 'info'; text: string } | null,
  durationMs = 5000
) => {
  if (launchMessageTimer) {
    clearTimeout(launchMessageTimer)
    launchMessageTimer = null
  }
  launchMessage.value = msg
  if (msg && durationMs > 0) {
    launchMessageTimer = setTimeout(() => {
      launchMessage.value = null
      launchMessageTimer = null
    }, durationMs)
  }
}

// 游戏运行与启动生命周期事件监听器
let unlistenLaunch: (() => void) | null = null
let unlistenExit: (() => void) | null = null
let unlistenCrash: (() => void) | null = null

const showLoginModal = ref(false)
const loginMode = ref<'msa' | 'yggdrasil'>('msa')

const scanJavaRuntime = async () => {
  isScanning.value = true
  try {
    const javas = await backend.getInstalledJava()
    javaList.value = javas
    const reqMajor = store.currentVersion?.java_version || 8
    const matched = javas.find((j) => j.major_version === reqMajor)
    if (matched) {
      selectedJava.value = matched
    } else if (javas.length > 0 && !selectedJava.value) {
      selectedJava.value = javas[0]
    }
  } finally {
    isScanning.value = false
  }
}

const loadData = async () => {
  isScanning.value = true
  try {
    if (!store.gameDir) {
      await store.initConfig()
    }
    await Promise.all([scanJavaRuntime(), store.refreshInstalledVersions()])
  } finally {
    isScanning.value = false
  }
}

watch(
  () => store.gameDir,
  (newDir, oldDir) => {
    if (newDir && oldDir && newDir !== oldDir) {
      loadData()
    }
  }
)

onMounted(async () => {
  await store.initConfig()
  void scanJavaRuntime()

  // 监听后端派发的启动进度事件
  unlistenLaunch = await backend.onLaunchProgress((p) => {
    launchProgress.value = p
    if (p.pid) {
      runningPid.value = p.pid
    }
    if (p.is_finished) {
      isLaunching.value = false
      if (p.error) {
        showLaunchMessage({ type: 'error', text: p.error }, 8000)
        runningPid.value = null
      }
    }
  })

  // 监听游戏正常退出
  unlistenExit = await backend.onGameExitNormal((exitCode) => {
    store.resetGameState()
    const text =
      exitCode !== undefined && exitCode !== 0
        ? `Minecraft 游戏进程已退出 (代码: ${exitCode})`
        : 'Minecraft 游戏进程已正常退出'
    showLaunchMessage({ type: 'info', text }, 3500)
  })

  // 监听游戏异常退出或崩溃
  unlistenCrash = await backend.onGameCrash(() => {
    store.resetGameState()
    showLaunchMessage({ type: 'error', text: 'Minecraft 游戏进程异常退出 (已生成崩溃分析)' }, 6000)
  })
})

onUnmounted(() => {
  if (launchMessageTimer) {
    clearTimeout(launchMessageTimer)
    launchMessageTimer = null
  }
  if (unlistenLaunch) unlistenLaunch()
  if (unlistenExit) unlistenExit()
  if (unlistenCrash) unlistenCrash()
})

// 强制终止卡死的游戏进程
const handleKillGame = async () => {
  if (!runningPid.value) return
  if (confirm(`确定要强制终止当前 Minecraft 进程 (PID: ${runningPid.value}) 吗？`)) {
    try {
      await backend.killGameProcess(runningPid.value)
      store.resetGameState()
      showLaunchMessage({ type: 'info', text: '已向游戏进程发送强制终止信号' }, 3500)
    } catch (e: any) {
      alert(`终止进程失败: ${e}`)
    }
  }
}

const openLoginModal = (mode: 'msa' | 'yggdrasil') => {
  loginMode.value = mode
  showLoginModal.value = true
}

const handleLaunch = async () => {
  if (!store.currentVersion) {
    alert('请先选择一个 Minecraft 版本！')
    return
  }
  const versionExists = await store.validateTargetExists(
    store.currentVersion.path,
    `Minecraft 版本【${store.currentVersion.name || store.currentVersion.id}】`
  )
  if (!versionExists) return

  isLaunching.value = true
  showLaunchMessage(null)
  launchProgress.value = {
    step: '正在初始化启动环境...',
    percentage: 5,
    is_finished: false,
  }

  try {
    const res = await backend.launchMinecraft({
      gameDir: store.gameDir,
      versionId: store.currentVersion.id,
      javaPath: selectedJava.value?.path || '',
      playerName: store.playerName || 'Steve',
      maxMemoryMb: store.maxMemoryMb,
      uuid: store.userProfile?.uuid,
      accessToken: store.userProfile?.access_token,
    })
    showLaunchMessage({ type: 'success', text: res }, 5000)
  } catch (err: any) {
    showLaunchMessage({ type: 'error', text: `启动失败: ${err}` }, 8000)
    store.resetGameState()
  } finally {
    isLaunching.value = false
  }
}
</script>

<template>
  <div class="h-full flex relative" :class="store.uiStyle === 'classic' ? 'gap-0' : 'gap-5'">
    <template v-if="store.uiStyle === 'classic'">
      <!-- 左侧固定 300px 白底控制面板 (PageLaunchLeft.xaml) -->
      <div
        class="w-[300px] h-full bg-white dark:bg-[#1e232b] border-r border-black/8 dark:border-white/10 flex flex-col justify-between px-5 py-5 shrink-0 select-none"
      >
        <!-- 状态 A：正常待机输入态 (PanInput) -->
        <template v-if="!isLaunching && !runningPid">
          <!-- 顶部：登录方式单选胶囊 (PanType: 正版 / 外置 / 离线) -->
          <div class="flex items-center justify-center gap-1.5 pt-1">
            <button
              class="h-[27px] px-3 rounded-[13.5px] flex items-center gap-1.5 text-[12.5px] transition-all cursor-pointer"
              :class="[
                store.userProfile && store.userProfile.auth_type !== 'yggdrasil'
                  ? 'bg-[var(--pcl-brush-3)] text-white font-medium shadow-xs'
                  : 'text-[var(--pcl-brush-3)] hover:bg-[var(--pcl-brush-7)]'
              ]"
              @click="openLoginModal('msa')"
            >
              <PclIcon name="msa" :size="13" />
              <span>正版</span>
            </button>
            <button
              v-if="!store.hideYggdrasilLogin"
              class="h-[27px] px-3 rounded-[13.5px] flex items-center gap-1.5 text-[12.5px] transition-all cursor-pointer"
              :class="[
                store.userProfile?.auth_type === 'yggdrasil'
                  ? 'bg-[var(--pcl-brush-3)] text-white font-medium shadow-xs'
                  : 'text-[var(--pcl-brush-3)] hover:bg-[var(--pcl-brush-7)]'
              ]"
              @click="openLoginModal('yggdrasil')"
            >
              <PclIcon name="yggdrasil" :size="13" />
              <span>外置</span>
            </button>
            <button
              class="h-[27px] px-3 rounded-[13.5px] flex items-center gap-1.5 text-[12.5px] transition-all cursor-pointer"
              :class="[
                !store.userProfile
                  ? 'bg-[var(--pcl-brush-3)] text-white font-medium shadow-xs'
                  : 'text-[var(--pcl-brush-3)] hover:bg-[var(--pcl-brush-7)]'
              ]"
              @click="store.setUserProfile(null)"
            >
              <PclIcon name="offline" :size="13" />
              <span>离线</span>
            </button>
          </div>

          <!-- 中部：账号头像与输入区 (PanLogin) -->
          <div class="my-auto flex flex-col items-center gap-3 py-4">
            <img
              v-if="store.userProfile"
              :src="store.userProfile.avatar_url"
              class="w-16 h-16 rounded-[6px] shadow-md border border-black/10"
            />
            <div
              v-else
              class="w-16 h-16 rounded-[6px] bg-[var(--pcl-brush-7)] border border-[var(--pcl-brush-5)] flex items-center justify-center text-[var(--pcl-brush-3)]"
            >
              <PclIcon name="vanilla" :size="32" />
            </div>

            <div v-if="store.userProfile" class="text-center space-y-1">
              <div class="text-[15px] font-bold text-[var(--pcl-brush-1)] dark:text-gray-100">
                {{ store.userProfile.name }}
              </div>
              <div class="text-[11px] text-[#737373]">
                {{ store.userProfile.auth_type === 'yggdrasil' ? '外置皮肤站角色' : '微软正版授权角色' }}
              </div>
              <button
                class="text-[11px] text-[var(--pcl-brush-3)] hover:underline cursor-pointer mt-1"
                @click="store.setUserProfile(null)"
              >
                退出当前账号
              </button>
            </div>

            <div v-else class="w-full px-2 space-y-2">
              <label class="block text-center text-[11px] text-[#737373]">离线玩家名称</label>
              <input
                v-model="store.playerName"
                placeholder="输入游戏昵称..."
                class="w-full h-[32px] px-3 rounded-[4px] bg-white dark:bg-white/5 border border-[var(--pcl-brush-5)] focus:border-[var(--pcl-brush-3)] text-center text-[13px] text-[var(--pcl-brush-1)] dark:text-gray-100 outline-none transition-colors"
                @change="store.saveCurrentConfig"
              />
            </div>
          </div>

          <!-- 底部：54px 双行「启动游戏」主按钮 + 版本选择框 + 35x35 齿轮按钮 -->
          <div class="space-y-2.5">
            <button
              :disabled="!store.currentVersion"
              class="w-full h-[54px] rounded-[4px] border border-[var(--pcl-brush-3)] bg-[var(--pcl-brush-8)] hover:bg-[var(--pcl-brush-6)] active:scale-[0.99] transition-all flex flex-col items-center justify-center cursor-pointer disabled:opacity-50"
              @click="handleLaunch"
            >
              <span class="text-[15px] font-bold text-[var(--pcl-brush-2)] leading-tight">
                启动游戏
              </span>
              <span class="text-[11px] text-[#737373] truncate max-w-[230px] mt-0.5">
                {{ store.currentVersion ? store.currentVersion.name : '未检测到游戏版本，请先下载' }}
              </span>
            </button>

            <div class="flex items-center gap-2">
              <button
                class="flex-1 h-[35px] px-3 rounded-[4px] border border-black/25 hover:border-[var(--pcl-brush-3)] bg-white text-[12.5px] text-[var(--pcl-brush-1)] flex items-center justify-between cursor-pointer transition-colors"
                title="直接切换要启动的 Minecraft 版本"
                @click="showVersionPicker = !showVersionPicker"
              >
                <span class="truncate">{{ store.currentVersion?.name || '选择版本...' }}</span>
                <span class="text-[10px] text-[#8c8c8c] ml-1">▲</span>
              </button>

              <button
                class="w-[35px] h-[35px] rounded-[4px] border border-black/25 hover:border-[var(--pcl-brush-3)] hover:text-[var(--pcl-brush-3)] bg-white text-[var(--pcl-brush-1)] flex items-center justify-center cursor-pointer transition-colors shrink-0"
                title="版本管理与设置（Mod / 存档 / 整合包管理）"
                @click="store.currentTab = 'instance'"
              >
                <PclIcon name="gear" :size="17" />
              </button>
            </div>

            <VersionPicker
              v-if="showVersionPicker"
              layout="classic"
              @select="showVersionPicker = false"
            />
          </div>
        </template>

        <template v-else>
          <div class="my-auto flex flex-col items-center text-center px-2">
            <div class="w-11 h-11 rounded-full border-3 border-[var(--pcl-brush-6)] border-t-[var(--pcl-brush-3)] animate-spin mb-3" />

            <div
              class="text-[20px] font-bold text-[var(--pcl-brush-3)]"
              style="transform: skewX(-3deg)"
            >
              {{ runningPid ? '游戏正在运行' : '正在启动游戏' }}
            </div>
            <div
              class="text-[13.5px] text-[var(--pcl-brush-3)] mt-1 truncate max-w-[240px]"
              style="transform: skewX(-3deg)"
            >
              {{ store.currentVersion?.name }}
            </div>

            <div class="w-full h-[4px] bg-[var(--pcl-brush-6)] my-4 overflow-hidden rounded-xs">
              <div
                class="h-full transition-all duration-300"
                style="background: linear-gradient(90deg, var(--pcl-brush-4) 0%, var(--pcl-brush-3) 60%)"
                :style="{ width: `${launchProgress?.percentage || 100}%` }"
              />
            </div>

            <div class="w-full text-[12.5px] space-y-1.5 text-left px-3">
              <div class="grid grid-cols-[72px_1fr] gap-3">
                <span class="text-right text-[var(--pcl-brush-1)] opacity-50">当前步骤</span>
                <span class="text-[var(--pcl-brush-1)] truncate">{{ launchProgress?.step || '进程守护中' }}</span>
              </div>
              <div class="grid grid-cols-[72px_1fr] gap-3">
                <span class="text-right text-[var(--pcl-brush-1)] opacity-50">登录方式</span>
                <span class="text-[var(--pcl-brush-1)]">
                  {{
                    store.userProfile
                      ? store.userProfile.auth_type === 'yggdrasil'
                        ? '外置登录'
                        : '正版登录'
                      : '离线登录'
                  }}
                </span>
              </div>
              <div class="grid grid-cols-[72px_1fr] gap-3">
                <span class="text-right text-[var(--pcl-brush-1)] opacity-50">启动进度</span>
                <span class="text-[var(--pcl-brush-1)] font-mono">
                  {{ (launchProgress?.percentage ?? 100).toFixed(2) }} %
                </span>
              </div>
            </div>

            <div v-if="runningPid" class="mt-6 w-full">
              <MyButton type="danger" size="small" class="w-full" @click="handleKillGame">
                ⏹️ 强制关闭 Minecraft (PID: {{ runningPid }})
              </MyButton>
            </div>
          </div>
        </template>
      </div>

      <div class="flex-1 h-full overflow-y-auto p-5 space-y-3.5">
        <MyHint
          v-if="launchMessage"
          :type="launchMessage.type"
          @close="launchMessage = null"
        >
          {{ launchMessage.text }}
        </MyHint>

        <MyCard
          :title="store.customHomepageNotice ? '📌 自定义主页公告' : '👋 欢迎使用 Pure Build Launcher (PBL)'"
          subtitle="可在「设置 → 个性化」中自定义此处的主页看板文案与背景壁纸"
        >
          <div class="text-xs text-pcl-text whitespace-pre-wrap leading-relaxed">
            {{
              store.customHomepageNotice ||
              `当前工作目录：${store.gameDir}\n已扫描到本地版本：${store.installedVersions.length} 个 | 可用 Java 运行时：${javaList.length} 个。\n点击左侧面板底部的齿轮按钮可进入单版本 Mod、存档与整合包管理。`
            }}
          </div>
        </MyCard>

        <MyCard title="Java 运行环境选择" subtitle="点击切换用于启动当前版本的 Java 虚拟机">
          <template #extra>
            <MyButton type="text" size="small" :loading="isScanning" @click="loadData">
              刷新
            </MyButton>
          </template>
          <div class="space-y-1.5">
            <div
              v-for="(j, idx) in javaList"
              :key="j.path"
              class="flex items-center justify-between px-3 py-2 rounded-[4px] border text-xs cursor-pointer transition-colors"
              :class="
                selectedJava?.path === j.path
                  ? 'border-[var(--pcl-brush-3)] bg-[var(--pcl-brush-7)] text-[var(--pcl-brush-2)]'
                  : 'border-black/8 bg-white/60 hover:bg-[var(--pcl-brush-8)]'
              "
              @click="selectedJava = j"
            >
              <div class="truncate pr-2">
                <span class="font-semibold">Java {{ j.version }}</span>
                <span class="ml-2 text-[11px] text-[#737373]">({{ j.arch || '64-Bit' }})</span>
                <span v-if="idx === 0" class="ml-1.5 text-[10px] text-[var(--pcl-brush-3)] font-bold">[推荐]</span>
                <div class="text-[11px] text-[#8c8c8c] truncate font-mono">{{ j.path }}</div>
              </div>
              <span v-if="selectedJava?.path === j.path" class="font-bold text-[var(--pcl-brush-3)] shrink-0">
                已选中
              </span>
            </div>
          </div>
        </MyCard>

        <MyCard title="PBL 经典主题色盘" subtitle="点击标题栏展开 · 实时变换全局 8 阶动态色板" :default-collapsed="true">
          <div class="flex items-center gap-2.5 flex-wrap">
            <button
              v-for="hue in [205, 145, 270, 345, 35, 175]"
              :key="hue"
              class="w-6 h-6 rounded-full border-2 border-white shadow-xs cursor-pointer transition-transform hover:scale-110"
              :style="{ backgroundColor: `hsl(${hue}, 85%, 52%)` }"
              @click="store.setThemeHue(hue)"
            />
          </div>
        </MyCard>
      </div>
    </template>

    <template v-else>
      <div class="flex-1 h-full flex flex-col gap-4 overflow-hidden">
        <div class="flex-1 grid grid-cols-12 gap-4 overflow-hidden">
          <div class="col-span-7 flex flex-col gap-3 overflow-y-auto pr-1">
            <MyHint
              v-if="launchMessage"
              :type="launchMessage.type"
              @close="launchMessage = null"
            >
              {{ launchMessage.text }}
            </MyHint>

            <div class="rounded-2xl bg-pcl-card border border-pcl-border px-4 py-3 flex items-center justify-between shadow-pcl-card shrink-0">
              <div class="space-y-1 min-w-0 pr-3">
                <div class="flex items-center gap-2">
                  <span class="text-sm font-bold text-pcl-text truncate">
                    {{ store.customHomepageNotice ? '📌 自定义主页看板' : '👋 欢迎使用 Pure Build Launcher' }}
                  </span>
                  <span class="px-2 py-0.5 rounded-full bg-emerald-500/15 text-emerald-500 text-[11px] font-semibold shrink-0">
                    已就绪
                  </span>
                </div>
                <div v-if="store.customHomepageNotice" class="text-xs text-pcl-subtext whitespace-pre-wrap leading-relaxed">
                  {{ store.customHomepageNotice }}
                </div>
                <div v-else class="text-xs text-pcl-subtext flex items-center gap-1.5 truncate">
                  <span class="shrink-0">📁 工作目录：</span>
                  <code
                    class="px-1.5 py-0.5 rounded bg-black/5 dark:bg-black/30 text-pcl-text font-mono truncate"
                    :title="store.gameDir"
                  >
                    {{ store.gameDir }}
                  </code>
                </div>
              </div>
              <MyButton
                type="secondary"
                size="small"
                class="shrink-0"
                @click="backend.openFolder(store.gameDir)"
              >
                📂 打开目录
              </MyButton>
            </div>

            <MyCard
              title="当前游戏实例"
              :subtitle="`已扫描到 ${store.installedVersions.length} 个本地客户端 · 点击可切换或管理 Mod/存档`"
              :can-collapse="false"
            >
              <template #extra>
                <MyButton type="text" size="small" :loading="isScanning" @click="loadData">
                  🔄 刷新扫描
                </MyButton>
              </template>

              <div
                v-if="store.currentVersion"
                class="flex items-center justify-between px-3.5 py-2.5 rounded-xl bg-black/5 dark:bg-black/25 border border-pcl-border"
              >
                <div class="flex items-center gap-3 overflow-hidden pr-2">
                  <div class="w-10 h-10 rounded-xl bg-pcl-primary/15 border border-pcl-primary/30 text-pcl-primary flex items-center justify-center text-lg shrink-0">
                    📦
                  </div>
                  <div class="truncate">
                    <div class="font-bold text-sm text-pcl-text truncate">
                      {{ store.currentVersion.name }}
                    </div>
                    <div class="text-xs text-pcl-subtext mt-0.5 flex items-center gap-2">
                      <span class="bg-pcl-primary/15 text-pcl-primary px-1.5 py-0.2 rounded font-semibold">
                        {{ store.currentVersion.loader || 'Vanilla 原版' }}
                      </span>
                      <span class="truncate">{{ store.currentVersion.has_indie ? '独立版本隔离已启用' : '通用目录' }}</span>
                    </div>
                  </div>
                </div>

                <div class="flex items-center gap-1.5 shrink-0">
                  <MyButton
                    type="secondary"
                    size="small"
                    title="管理该版本的 Mod、存档与光影"
                    @click="store.currentTab = 'instance'"
                  >
                    ⚙️ 实例管理
                  </MyButton>
                  <MyButton
                    type="primary"
                    size="small"
                    @click="showVersionPicker = !showVersionPicker"
                  >
                    {{ showVersionPicker ? '收起列表 ▴' : '切换版本 ▾' }}
                  </MyButton>
                </div>
              </div>
              <div v-else class="text-xs text-pcl-subtext p-2.5">
                在该目录下未检测到有效游戏版本，请前往「下载」页安装。
              </div>

              <VersionPicker
                v-if="showVersionPicker"
                layout="modern"
                @select="showVersionPicker = false"
              />
            </MyCard>

            <MyCard
              title="Java 运行时环境"
              subtitle="点击展开切换用于启动该版本的 Java 虚拟机"
              :default-collapsed="true"
            >
              <template #extra>
                <span
                  v-if="selectedJava"
                  class="px-2 py-0.5 rounded-md bg-pcl-primary/15 border border-pcl-primary/30 text-pcl-primary text-[11px] font-semibold"
                >
                  ☕ Java {{ selectedJava.version }} ({{ selectedJava.arch || '64-Bit' }})
                </span>
              </template>

              <div v-if="javaList.length === 0" class="text-xs text-pcl-subtext p-2">
                正在搜寻 Java 运行时...
              </div>
              <div v-else class="space-y-2">
                <div
                  v-for="(j, idx) in javaList"
                  :key="j.path"
                  class="flex items-center justify-between px-3 py-2 rounded-xl border border-pcl-border bg-black/5 dark:bg-black/25 text-xs cursor-pointer transition-all"
                  :class="selectedJava?.path === j.path && 'border-pcl-primary bg-pcl-primary/10 ring-1 ring-pcl-primary/30'"
                  @click="selectedJava = j"
                >
                  <div class="flex items-center gap-2.5 overflow-hidden">
                    <span class="text-base">{{ j.is_jdk ? '☕' : '📦' }}</span>
                    <div class="truncate">
                      <div class="font-semibold text-pcl-text flex items-center gap-1.5">
                        <span>Java {{ j.version }}</span>
                        <span v-if="idx === 0" class="text-[10px] bg-pcl-primary/20 text-pcl-primary px-1.5 py-0.2 rounded font-bold">推荐</span>
                        <span class="text-[10px] bg-black/5 dark:bg-white/10 px-1.5 py-0.2 rounded text-pcl-subtext">
                          {{ j.arch || (j.is_64_bit ? '64-Bit' : '32-Bit') }}
                        </span>
                      </div>
                      <div class="text-[11px] text-pcl-subtext truncate mt-0.5 font-mono" :title="j.path">{{ j.path }}</div>
                    </div>
                  </div>
                  <span v-if="selectedJava?.path === j.path" class="text-pcl-primary font-bold ml-2 shrink-0">✔ 已选</span>
                  <span v-else class="text-emerald-500 font-bold ml-2 shrink-0">可用</span>
                </div>
              </div>
            </MyCard>
          </div>

          <div class="col-span-5 flex flex-col gap-3.5 overflow-y-auto pr-0.5">
            <MyCard
              title="玩家身份"
              :subtitle="store.hideYggdrasilLogin ? '支持离线登录与微软正版授权' : '支持离线登录、微软正版与外置登录'"
              :can-collapse="false"
            >
              <div
                class="grid gap-1 p-1 rounded-xl bg-black/5 dark:bg-black/30 border border-pcl-border mb-3"
                :class="store.hideYggdrasilLogin ? 'grid-cols-2' : 'grid-cols-3'"
              >
                <button
                  class="h-7 rounded-lg text-[11px] font-semibold transition-all cursor-pointer flex items-center justify-center gap-1"
                  :class="
                    !store.userProfile
                      ? 'bg-pcl-primary text-white shadow-xs'
                      : 'text-pcl-subtext hover:text-pcl-text'
                  "
                  @click="store.setUserProfile(null)"
                >
                  👤 离线登录
                </button>
                <button
                  class="h-7 rounded-lg text-[11px] font-semibold transition-all cursor-pointer flex items-center justify-center gap-1"
                  :class="
                    store.userProfile && store.userProfile.auth_type !== 'yggdrasil'
                      ? 'bg-pcl-primary text-white shadow-xs'
                      : 'text-pcl-subtext hover:text-pcl-text'
                  "
                  @click="openLoginModal('msa')"
                >
                  🔑 微软正版
                </button>
                <button
                  v-if="!store.hideYggdrasilLogin"
                  class="h-7 rounded-lg text-[11px] font-semibold transition-all cursor-pointer flex items-center justify-center gap-1"
                  :class="
                    store.userProfile?.auth_type === 'yggdrasil'
                      ? 'bg-pcl-primary text-white shadow-xs'
                      : 'text-pcl-subtext hover:text-pcl-text'
                  "
                  @click="openLoginModal('yggdrasil')"
                >
                  🌐 外置登录
                </button>
              </div>

              <div class="flex items-center gap-3 p-3 rounded-xl bg-black/5 dark:bg-[#181c24] border border-pcl-border">
                <img
                  v-if="store.userProfile"
                  :src="store.userProfile.avatar_url"
                  class="w-11 h-11 rounded-xl bg-black/10 shrink-0 border border-pcl-border shadow-sm"
                />
                <div
                  v-else
                  class="w-11 h-11 rounded-xl bg-pcl-primary/15 border border-pcl-primary/30 text-pcl-primary flex items-center justify-center text-xl shrink-0"
                >
                  👤
                </div>

                <div class="overflow-hidden flex-1">
                  <div v-if="store.userProfile" class="font-bold text-sm text-pcl-text truncate">
                    {{ store.playerName }}
                  </div>
                  <div v-else class="space-y-1">
                    <div class="flex items-center justify-between text-[11px]">
                      <span class="text-pcl-subtext font-medium">离线游戏昵称</span>
                      <span class="text-pcl-primary font-medium">✏️ 点击可直接修改</span>
                    </div>
                    <input
                      v-model="store.playerName"
                      class="w-full h-8 px-3 rounded-lg font-bold text-xs text-pcl-text bg-[var(--pcl-color-bg)] border border-pcl-border focus:border-pcl-primary outline-none shadow-inner transition-colors"
                      placeholder="输入离线游戏昵称 (如 Steve)"
                      @change="store.saveCurrentConfig"
                    />
                  </div>

                  <div v-if="store.userProfile" class="text-[11px] mt-1 flex items-center gap-1.5">
                    <span class="px-1.5 py-0.2 rounded font-medium text-[10px] bg-emerald-500/15 text-emerald-500">
                      {{ store.userProfile.auth_type === 'yggdrasil' ? '外置正版已认证' : '微软正版已认证' }}
                    </span>
                  </div>
                </div>
              </div>

              <div v-if="store.userProfile" class="mt-3 flex gap-2">
                <MyButton
                  type="secondary"
                  size="small"
                  class="flex-1"
                  @click="store.setUserProfile(null)"
                >
                  👤 切回离线登录
                </MyButton>
              </div>
            </MyCard>

            <MyCard title="主题色快速调色盘" subtitle="点击展开 · 动态计算全局 HSL 梯度" :default-collapsed="true">
              <div class="flex gap-2.5 flex-wrap pt-1">
                <button
                  v-for="hue in [205, 145, 270, 350, 35, 175]"
                  :key="hue"
                  class="w-7 h-7 rounded-full border-2 border-white dark:border-gray-800 cursor-pointer shadow-sm transition-transform hover:scale-110 active:scale-95"
                  :style="{ backgroundColor: `hsl(${hue}, 85%, 52%)` }"
                  @click="store.setThemeHue(hue)"
                />
              </div>
            </MyCard>
          </div>
        </div>

        <div
          class="h-[50px] rounded-xl bg-pcl-card border px-3.5 flex items-center justify-between gap-3 shrink-0 shadow-pcl-card transition-all"
          :class="runningPid ? 'border-emerald-500/40' : 'border-pcl-border'"
        >
          <div v-if="launchProgress && !launchProgress.error" class="flex-1 min-w-0 space-y-1">
            <div class="flex items-center justify-between text-xs">
              <span class="font-bold text-pcl-text flex items-center gap-2 truncate">
                <span v-if="!runningPid" class="animate-spin text-xs">⚙️</span>
                <span v-else class="w-2 h-2 rounded-full bg-emerald-500 animate-ping inline-block" />
                <span class="truncate">
                  {{ runningPid ? `Minecraft 运行中 (PID: ${runningPid}) · ${launchProgress.step}` : launchProgress.step }}
                </span>
              </span>
              <div class="flex items-center gap-3 shrink-0">
                <span class="font-bold font-mono" :class="runningPid ? 'text-emerald-500' : 'text-pcl-primary'">
                  {{ launchProgress.percentage.toFixed(0) }}%
                </span>
                <button
                  v-if="runningPid"
                  class="text-xs text-red-500 hover:text-red-400 font-semibold cursor-pointer"
                  @click="handleKillGame"
                >
                  ⏹️ 结束进程
                </button>
              </div>
            </div>
            <div class="w-full h-1 rounded-full bg-black/10 dark:bg-white/10 overflow-hidden">
              <div
                class="h-full transition-all duration-300 ease-out rounded-full"
                :class="runningPid ? 'bg-emerald-500' : 'bg-pcl-primary'"
                :style="{ width: `${launchProgress.percentage}%` }"
              />
            </div>
          </div>

          <div v-else class="flex items-center gap-2 text-xs overflow-hidden">
            <div class="flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-black/5 dark:bg-black/25 border border-pcl-border truncate">
              <span>📦</span>
              <span class="font-bold text-pcl-text truncate max-w-[180px]">
                {{ store.currentVersion?.name || '未选择实例' }}
              </span>
            </div>
            <div class="flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-black/5 dark:bg-black/25 border border-pcl-border text-pcl-subtext shrink-0">
              <span>☕</span>
              <span>{{ selectedJava ? `Java ${selectedJava.version} (${selectedJava.arch || '64-Bit'})` : '未检测 Java' }}</span>
            </div>
            <div class="flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-black/5 dark:bg-black/25 border border-pcl-border text-pcl-subtext truncate">
              <span>👤</span>
              <span class="truncate max-w-[120px]">
                {{ store.playerName || 'Steve' }} ({{ store.userProfile ? '正版' : '离线' }})
              </span>
            </div>
          </div>

          <button
            :disabled="isLaunching || !store.currentVersion"
            class="h-[34px] px-6 rounded-lg text-white text-[13px] font-bold shadow-md transition-all duration-200 active:scale-95 cursor-pointer flex items-center justify-center gap-2 select-none disabled:opacity-50 shrink-0"
            :class="
              runningPid
                ? 'bg-emerald-600 hover:bg-emerald-500 shadow-emerald-500/20'
                : 'bg-pcl-primary hover:bg-pcl-hover active:bg-pcl-active shadow-pcl-primary/30'
            "
            @click="handleLaunch"
          >
            <span v-if="isLaunching" class="animate-spin text-sm">⚙️</span>
            <span v-else-if="runningPid" class="text-sm">🎮</span>
            <span v-else class="text-sm">🚀</span>
            <span>
              {{
                isLaunching
                  ? `启动中 (${launchProgress?.percentage.toFixed(0) || 10}%)`
                  : runningPid
                    ? '运行中 (可多开)'
                    : '进入游戏'
              }}
            </span>
          </button>
        </div>
      </div>
    </template>

    <LoginModal v-model="showLoginModal" :mode="loginMode" />
  </div>
</template>
