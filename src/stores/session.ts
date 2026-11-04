import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { UserProfile } from '../services/types.generated'
import type { LaunchProgress, DownloadProgress } from '../services/backend'
import { backend } from '../services/backend'

export const useSessionStore = defineStore('session', () => {
  const userProfile = ref<UserProfile | null>(null)
  const isLoginModalOpen = ref(false)
  const currentTab = ref('launch')

  const runningPid = ref<number | null>(null)
  const isLaunching = ref(false)
  const launchProgress = ref<LaunchProgress | null>(null)

  const downloadTask = ref<DownloadProgress | null>(null)
  const isDownloading = computed(() => !!downloadTask.value && !downloadTask.value.is_finished)
  let downloadFinishedTimer: any = null

  const notificationMessage = ref('')
  let notificationTimer: any = null

  const showNotification = (msg: string) => {
    notificationMessage.value = msg
    if (notificationTimer) clearTimeout(notificationTimer)
    notificationTimer = setTimeout(() => {
      notificationMessage.value = ''
    }, 4500)
  }

  const setUserProfile = async (profile: UserProfile | null) => {
    userProfile.value = profile
    try {
      await backend.saveUserProfile(profile)
    } catch (e) {
      console.error('保存用户凭据失败:', e)
    }
  }

  const resetGameState = () => {
    runningPid.value = null
    isLaunching.value = false
    launchProgress.value = null
  }

  const updateDownloadProgress = (
    progress: DownloadProgress,
    onFinished?: () => void,
  ) => {
    downloadTask.value = progress

    if (progress.is_finished) {
      if (onFinished) onFinished()
      if (downloadFinishedTimer) clearTimeout(downloadFinishedTimer)
      downloadFinishedTimer = setTimeout(() => {
        downloadTask.value = null
      }, 4000)
    }
  }

  const clearDownloadTask = () => {
    if (downloadFinishedTimer) clearTimeout(downloadFinishedTimer)
    downloadTask.value = null
  }

  return {
    userProfile,
    isLoginModalOpen,
    currentTab,
    runningPid,
    isLaunching,
    launchProgress,
    downloadTask,
    isDownloading,
    notificationMessage,
    showNotification,
    setUserProfile,
    resetGameState,
    updateDownloadProgress,
    clearDownloadTask,
  }
})
