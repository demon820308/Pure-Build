import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { VersionInfo } from '../services/types.generated'
import { backend } from '../services/backend'
import { pruneSelectedVersion } from '../utils/coreLogic'

export const useVersionsStore = defineStore('versions', () => {
  const installedVersions = ref<VersionInfo[]>([])
  const currentVersion = ref<VersionInfo | null>(null)
  const instanceRefreshTrigger = ref(0)

  const refreshInstalledVersions = async (
    gameDir: string,
    onNotice?: (msg: string) => void,
  ) => {
    if (!gameDir) return
    try {
      const list = await backend.getMinecraftVersions(gameDir)
      installedVersions.value = list
      const decision = pruneSelectedVersion(currentVersion.value, list)
      currentVersion.value = decision.nextVersion
      if (decision.notice && onNotice) {
        onNotice(decision.notice)
      }
    } catch (e) {
      console.warn('获取已安装版本失败:', e)
    }
  }

  const setCurrentVersion = (version: VersionInfo | null) => {
    currentVersion.value = version
  }

  const triggerInstanceRefresh = () => {
    instanceRefreshTrigger.value++
  }

  return {
    installedVersions,
    currentVersion,
    instanceRefreshTrigger,
    refreshInstalledVersions,
    setCurrentVersion,
    triggerInstanceRefresh,
  }
})
