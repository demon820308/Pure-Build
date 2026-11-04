import { ref, onMounted, onUnmounted, watch } from 'vue'
import type { Ref } from 'vue'
import { storeToRefs } from 'pinia'
import { useAppStore } from '@/stores/app'
import { backend } from '@/services/backend'
import type { JavaInfo } from '@/services/backend'

export interface UseLaunchOptions {
  selectedJava: Ref<JavaInfo | null>
  javaPanelRef?: Ref<{ scanJavaRuntime: () => Promise<void> } | null>
}

export function useLaunch(options: UseLaunchOptions) {
  const store = useAppStore()
  const { isLaunching, launchProgress, runningPid } = storeToRefs(store)

  const isScanning = ref(false)
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

  const loadData = async () => {
    isScanning.value = true
    try {
      if (!store.gameDir) {
        await store.initConfig()
      }
      await Promise.all([
        options.javaPanelRef?.value?.scanJavaRuntime(),
        store.refreshInstalledVersions(),
      ])
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
        javaPath: options.selectedJava.value?.path || '',
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

  return {
    isScanning,
    launchMessage,
    showLaunchMessage,
    loadData,
    handleLaunch,
    handleKillGame,
  }
}
