<script setup lang="ts">
import { useAppStore } from '@/stores/app'
import MyCard from '@/components/base/MyCard.vue'
import MyButton from '@/components/base/MyButton.vue'
import MyHint from '@/components/base/MyHint.vue'
import { backend } from '@/services/backend'
import type { JavaInfo, VersionInfo, DeviceCodeResponse, LaunchProgress } from '@/services/backend'
import { ref, onMounted, onUnmounted } from 'vue'

const store = useAppStore()
const isLaunching = ref(false)
const javaList = ref<JavaInfo[]>([])
const selectedJava = ref<JavaInfo | null>(null)
const isScanning = ref(false)
const showVersionPicker = ref(false)
const launchMessage = ref<{ type: 'success' | 'error'; text: string } | null>(null)

// 游戏运行与启动进度状态
const launchProgress = ref<LaunchProgress | null>(null)
const runningPid = ref<number | null>(null)
let unlistenLaunch: (() => void) | null = null
let unlistenExit: (() => void) | null = null

// 微软登录弹窗状态
const showLoginModal = ref(false)
const isRequestingCode = ref(false)
const isPollingAuth = ref(false)
const deviceAuth = ref<DeviceCodeResponse | null>(null)

const loadData = async () => {
  isScanning.value = true
  try {
    const javas = await backend.getInstalledJava()
    javaList.value = javas
    if (javas.length > 0 && !selectedJava.value) {
      selectedJava.value = javas[0]
    }

    const versions = await backend.getMinecraftVersions(store.gameDir)
    store.installedVersions = versions
    if (versions.length > 0 && !store.currentVersion) {
      store.currentVersion = versions[0]
    }
  } finally {
    isScanning.value = false
  }
}

onMounted(async () => {
  loadData()

  // 监听后端派发的启动进度事件
  unlistenLaunch = await backend.onLaunchProgress((p) => {
    launchProgress.value = p
    if (p.pid) {
      runningPid.value = p.pid
    }
    if (p.is_finished) {
      isLaunching.value = false
      if (p.error) {
        launchMessage.value = { type: 'error', text: p.error }
        runningPid.value = null
      }
    }
  })

  // 监听游戏正常退出
  unlistenExit = await backend.onGameExitNormal(() => {
    runningPid.value = null
    launchProgress.value = null
    isLaunching.value = false
  })
})

onUnmounted(() => {
  if (unlistenLaunch) unlistenLaunch()
  if (unlistenExit) unlistenExit()
})

// 强制终止卡死的游戏进程
const handleKillGame = async () => {
  if (!runningPid.value) return
  if (confirm(`确定要强制终止当前 Minecraft 进程 (PID: ${runningPid.value}) 吗？`)) {
    try {
      await backend.killGameProcess(runningPid.value)
      runningPid.value = null
      launchProgress.value = null
      isLaunching.value = false
    } catch (e: any) {
      alert(`终止进程失败: ${e}`)
    }
  }
}

const selectVersion = (v: VersionInfo) => {
  store.currentVersion = v
  showVersionPicker.value = false
}

// 发起微软设备码登录
const startMicrosoftLogin = async () => {
  showLoginModal.value = true
  isRequestingCode.value = true
  deviceAuth.value = null
  try {
    const res = await backend.getMicrosoftDeviceCode()
    deviceAuth.value = res
    isRequestingCode.value = false
    isPollingAuth.value = true

    // 开始后台等待用户授权
    const profile = await backend.pollMicrosoftToken(res.device_code)
    store.setUserProfile(profile)
    showLoginModal.value = false
    alert(`🎉 微软正版账号登录成功！\n欢迎，${profile.name}`)
  } catch (err: any) {
    alert(`登录未完成: ${err}`)
  } finally {
    isRequestingCode.value = false
    isPollingAuth.value = false
  }
}

const copyCodeAndOpenUrl = () => {
  if (!deviceAuth.value) return
  navigator.clipboard.writeText(deviceAuth.value.user_code)
  window.open(deviceAuth.value.verification_uri, '_blank')
}

const handleLaunch = async () => {
  if (!store.currentVersion) {
    alert('请先选择一个 Minecraft 版本！')
    return
  }
  if (!selectedJava.value) {
    alert('未检测到可用的 Java 运行时！')
    return
  }

  isLaunching.value = true
  launchMessage.value = null
  launchProgress.value = {
    step: '正在初始化启动环境...',
    percentage: 5,
    is_finished: false,
  }

  try {
    const res = await backend.launchMinecraft({
      gameDir: store.gameDir,
      versionId: store.currentVersion.id,
      javaPath: selectedJava.value.path,
      playerName: store.playerName || 'Steve',
      maxMemoryMb: store.maxMemoryMb,
      uuid: store.userProfile?.uuid,
      accessToken: store.userProfile?.access_token,
    })
    launchMessage.value = { type: 'success', text: res }
  } catch (err: any) {
    launchMessage.value = { type: 'error', text: `启动失败: ${err}` }
  } finally {
    isLaunching.value = false
  }
}
</script>

<template>
  <div class="h-full flex gap-5 relative">
    <!-- 左侧主操作区 -->
    <div class="flex-1 flex flex-col justify-between overflow-y-auto pr-1">
      <div class="space-y-4">
        <MyHint v-if="launchMessage" :type="launchMessage.type === 'success' ? 'success' : 'error'">
          {{ launchMessage.text }}
        </MyHint>

        <MyHint v-else type="info">
          已成功绑定真实游戏目录：<b>{{ store.gameDir }}</b>
        </MyHint>

        <!-- 当前版本与实例选择卡片 -->
        <MyCard title="当前版本与实例" subtitle="选择你要运行的 Minecraft 实例">
          <template #extra>
            <MyButton type="text" size="small" :loading="isScanning" @click="loadData">
              🔄 刷新扫描
            </MyButton>
          </template>

          <div v-if="store.currentVersion" class="flex items-center justify-between p-3.5 rounded-xl bg-black/5 dark:bg-white/5 border border-pcl-border">
            <div class="flex items-center gap-3.5 overflow-hidden">
              <div class="w-11 h-11 rounded-xl bg-pcl-primary/20 flex items-center justify-center text-xl shrink-0">
                📦
              </div>
              <div class="truncate">
                <div class="font-semibold text-sm text-pcl-text truncate">
                  {{ store.currentVersion.name }}
                </div>
                <div class="text-xs text-pcl-subtext mt-0.5 flex items-center gap-2">
                  <span class="bg-pcl-primary/10 text-pcl-primary px-1.5 py-0.2 rounded font-medium">
                    {{ store.currentVersion.loader || 'Vanilla 原版' }}
                  </span>
                  <span>{{ store.currentVersion.has_indie ? '独立版本隔离已启用' : '通用目录' }}</span>
                </div>
              </div>
            </div>
            <MyButton type="primary" size="small" @click="showVersionPicker = !showVersionPicker">
              {{ showVersionPicker ? '收起列表' : '切换版本' }}
            </MyButton>
          </div>
          <div v-else class="text-xs text-pcl-subtext p-3">
            在该目录下未检测到有效游戏版本，请确认 versions 目录存在。
          </div>

          <!-- 展开的版本选择列表 -->
          <div v-if="showVersionPicker" class="mt-3 space-y-1.5 border-t border-pcl-border pt-3 max-h-52 overflow-y-auto">
            <div
              v-for="v in store.installedVersions"
              :key="v.id"
              class="flex items-center justify-between p-2.5 rounded-xl hover:bg-black/5 dark:hover:bg-white/5 cursor-pointer transition-colors text-xs"
              :class="store.currentVersion?.id === v.id && 'bg-pcl-primary/10 text-pcl-primary font-medium'"
              @click="selectVersion(v)"
            >
              <div class="flex items-center gap-2 truncate">
                <span>{{ v.loader ? '🧩' : '⛏️' }}</span>
                <span class="truncate">{{ v.name }}</span>
              </div>
              <span class="text-pcl-subtext shrink-0 ml-2">{{ v.loader || '原版' }}</span>
            </div>
          </div>
        </MyCard>

        <!-- 真实 Java 运行时环境卡片 -->
        <MyCard title="Java 运行时环境 (Rust 底层扫描)" subtitle="点击可切换用于启动该版本的 Java">
          <div v-if="javaList.length === 0" class="text-xs text-pcl-subtext p-2">
            正在搜寻 Java 运行时...
          </div>
          <div v-else class="space-y-2">
            <div
              v-for="(j, idx) in javaList"
              :key="j.path"
              class="flex items-center justify-between p-2.5 rounded-xl border border-pcl-border bg-black/5 dark:bg-white/5 text-xs cursor-pointer transition-all"
              :class="selectedJava?.path === j.path && 'border-pcl-primary ring-2 ring-pcl-primary/20'"
              @click="selectedJava = j"
            >
              <div class="flex items-center gap-2.5 overflow-hidden">
                <span class="text-base">{{ j.is_jdk ? '☕' : '📦' }}</span>
                <div class="truncate">
                  <div class="font-medium text-pcl-text flex items-center gap-1.5">
                    <span>Java {{ j.version }}</span>
                    <span v-if="idx === 0" class="text-[10px] bg-pcl-primary/20 text-pcl-primary px-1.5 py-0.2 rounded font-bold">推荐</span>
                    <span class="text-[10px] bg-black/5 dark:bg-white/10 px-1.5 py-0.2 rounded text-pcl-subtext">{{ j.is_64_bit ? '64-Bit' : '32-Bit' }}</span>
                  </div>
                  <div class="text-[11px] text-pcl-subtext truncate mt-0.5" :title="j.path">{{ j.path }}</div>
                </div>
              </div>
              <span v-if="selectedJava?.path === j.path" class="text-pcl-primary font-bold ml-2 shrink-0">✔ 已选</span>
              <span v-else class="text-emerald-500 font-bold ml-2 shrink-0">可用</span>
            </div>
          </div>
        </MyCard>
      </div>

      <!-- 底部启动操作区与实时监控卡片 -->
      <div class="pt-3 flex flex-col gap-2.5 shrink-0">
        <!-- 启动与运行进度监测条 (PCL 经典悬浮卡片) -->
        <div
          v-if="launchProgress && !launchProgress.error"
          class="p-3.5 rounded-2xl bg-pcl-card border shadow-md flex flex-col gap-2 animate-fade-in transition-all"
          :class="runningPid ? 'border-emerald-500/40 shadow-emerald-500/10' : 'border-pcl-primary/40 shadow-pcl-primary/10'"
        >
          <div class="flex items-center justify-between text-xs">
            <span class="font-bold text-pcl-text flex items-center gap-2">
              <span v-if="!runningPid" class="animate-spin text-sm">⚙️</span>
              <span v-else class="w-2 h-2 rounded-full bg-emerald-500 animate-ping inline-block" />
              <span>{{ runningPid ? `Minecraft 已就绪 (PID: ${runningPid})` : '正在准备拉起游戏...' }}</span>
            </span>
            <span class="font-bold font-mono" :class="runningPid ? 'text-emerald-500' : 'text-pcl-primary'">
              {{ launchProgress.percentage.toFixed(0) }}%
            </span>
          </div>

          <!-- 平滑进度条 -->
          <div class="w-full h-1.5 rounded-full bg-black/5 dark:bg-white/10 overflow-hidden">
            <div
              class="h-full transition-all duration-300 ease-out rounded-full"
              :class="runningPid ? 'bg-emerald-500' : 'bg-pcl-primary'"
              :style="{ width: `${launchProgress.percentage}%` }"
            />
          </div>

          <div class="text-[11px] text-pcl-subtext flex items-center justify-between">
            <span class="truncate pr-2">{{ launchProgress.step }}</span>
            <button
              v-if="runningPid"
              class="text-red-500 hover:text-red-600 font-semibold cursor-pointer shrink-0 transition-colors flex items-center gap-1"
              @click="handleKillGame"
            >
              <span>⏹️</span>
              <span>结束游戏进程</span>
            </button>
          </div>
        </div>

        <!-- 底部大按钮 -->
        <button
          :disabled="isLaunching || !store.currentVersion"
          class="w-full py-4.5 rounded-2xl text-white text-lg font-bold shadow-lg transition-all duration-300 ease-[var(--pcl-ease-bounce)] active:scale-[0.98] cursor-pointer flex items-center justify-center gap-3 select-none disabled:opacity-50 relative overflow-hidden"
          :class="runningPid ? 'bg-emerald-600 hover:bg-emerald-500 shadow-emerald-500/20' : 'bg-pcl-primary hover:bg-pcl-hover active:bg-pcl-active shadow-pcl-primary/30'"
          @click="handleLaunch"
        >
          <span v-if="isLaunching" class="animate-spin text-xl">⚙️</span>
          <span v-else-if="runningPid" class="text-xl">🎮</span>
          <span v-else class="text-xl">🚀</span>

          <span>
            {{
              isLaunching
                ? `启动中 (${launchProgress?.percentage.toFixed(0) || 10}%)`
                : runningPid
                  ? '游戏运行中 (点击可多开)'
                  : '进入游戏'
            }}
          </span>
        </button>
      </div>
    </div>

    <!-- 右侧用户档案与动态小部件 -->
    <div class="w-76 flex flex-col gap-4">
      <!-- 玩家身份卡片 (支持正版/离线切换) -->
      <MyCard title="玩家身份" subtitle="正版授权或离线游戏昵称">
        <div class="flex items-center gap-3.5 p-3 rounded-2xl bg-black/5 dark:bg-white/5 border border-pcl-border">
          <img
            v-if="store.userProfile"
            :src="store.userProfile.avatar_url"
            class="w-12 h-12 rounded-xl bg-black/10 shrink-0 border border-pcl-border shadow-sm"
          />
          <div v-else class="w-12 h-12 rounded-xl bg-pcl-primary/20 flex items-center justify-center text-2xl shrink-0">
            👤
          </div>

          <div class="overflow-hidden flex-1">
            <div class="font-bold text-sm text-pcl-text truncate">
              {{ store.playerName }}
            </div>
            <div class="text-[11px] mt-0.5 flex items-center gap-1.5">
              <span
                class="px-1.5 py-0.2 rounded font-medium text-[10px]"
                :class="store.userProfile ? 'bg-emerald-500/10 text-emerald-600' : 'bg-black/5 dark:bg-white/10 text-pcl-subtext'"
              >
                {{ store.userProfile ? '微软正版' : '离线模式' }}
              </span>
            </div>
          </div>
        </div>

        <div class="mt-3 flex gap-2">
          <MyButton
            v-if="!store.userProfile"
            type="primary"
            size="small"
            class="flex-1"
            @click="startMicrosoftLogin"
          >
            🔑 微软正版登录
          </MyButton>
          <MyButton
            v-else
            type="secondary"
            size="small"
            class="flex-1"
            @click="store.setUserProfile(null)"
          >
            退出正版
          </MyButton>
        </div>
      </MyCard>

      <MyCard title="主题色快速调色盘" subtitle="动态计算 HSL 梯度">
        <div class="flex gap-2 flex-wrap">
          <button
            v-for="hue in [205, 145, 270, 350, 35, 175]"
            :key="hue"
            class="w-7 h-7 rounded-full border-2 border-white dark:border-gray-800 cursor-pointer shadow-sm transition-transform hover:scale-110 active:scale-95"
            :style="{ backgroundColor: `hsl(${hue}, 85%, 52%)` }"
            @click="store.setThemeHue(hue)"
          />
        </div>
      </MyCard>

      <MyCard title="版本历史记录" subtitle="真实检测到的游戏客户端">
        <div class="space-y-2">
          <div
            v-for="v in store.installedVersions"
            :key="v.id"
            class="flex items-center justify-between p-2 rounded-lg hover:bg-black/5 dark:hover:bg-white/5 text-xs cursor-pointer transition-colors"
            :class="store.currentVersion?.id === v.id && 'font-bold text-pcl-primary'"
            @click="selectVersion(v)"
          >
            <span class="font-medium truncate max-w-[140px]">{{ v.name }}</span>
            <span class="text-pcl-subtext">{{ v.loader || 'Vanilla' }}</span>
          </div>
        </div>
      </MyCard>
    </div>

    <!-- 微软设备码登录弹出层 -->
    <div
      v-if="showLoginModal"
      class="absolute inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center p-6 z-50 rounded-2xl animate-fade-in"
    >
      <div class="w-96 bg-pcl-card rounded-2xl border border-pcl-border shadow-2xl p-6 flex flex-col gap-4">
        <div class="flex items-center justify-between">
          <h3 class="font-bold text-base text-pcl-text flex items-center gap-2">
            <span>🔑</span> 微软账号正版登录
          </h3>
          <button class="text-xs text-pcl-subtext hover:text-pcl-text cursor-pointer" @click="showLoginModal = false">
            ✕
          </button>
        </div>

        <div v-if="isRequestingCode" class="py-8 flex flex-col items-center justify-center gap-3 text-xs text-pcl-subtext">
          <span class="animate-spin text-2xl">⏳</span>
          <span>正在连接微软认证服务器...</span>
        </div>

        <div v-else-if="deviceAuth" class="space-y-4">
          <p class="text-xs text-pcl-subtext leading-relaxed">
            请点击下方按钮复制配对码，并在打开的微软官方页面中完成登录授权：
          </p>

          <!-- 大号高亮配对码 -->
          <div class="p-3.5 rounded-xl bg-black/5 dark:bg-white/5 border border-pcl-primary/30 flex items-center justify-between">
            <span class="text-xs text-pcl-subtext">配对码：</span>
            <span class="text-lg font-mono font-bold text-pcl-primary tracking-widest">
              {{ deviceAuth.user_code }}
            </span>
          </div>

          <MyButton type="primary" class="w-full py-2.5" @click="copyCodeAndOpenUrl">
            📋 复制配对码并打开微软页面
          </MyButton>

          <div v-if="isPollingAuth" class="flex items-center justify-center gap-2 text-[11px] text-pcl-subtext pt-1">
            <span class="animate-spin">⚙️</span>
            <span>等待你在浏览器中完成验证...</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
