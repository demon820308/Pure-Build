import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { HorizonInstance, VersionInfo } from '../services/types.generated'
import { backend } from '../services/backend'
import { pruneHorizonInstance, pruneSelectedVersion } from '../utils/coreLogic'

export const useVersionsStore = defineStore('versions', () => {
  const installedVersions = ref<VersionInfo[]>([])
  const currentVersion = ref<VersionInfo | null>(null)
  const horizonInstances = ref<HorizonInstance[]>([])
  const currentHorizonInstance = ref<HorizonInstance | null>(null)
  const instanceRefreshTrigger = ref(0)

  const refreshInstalledVersions = async (
    gameDir: string,
    onNotice?: (msg: string) => void,
  ) => {
    if (!gameDir) return
    try {
      const [list, hList] = await Promise.all([
        backend.getMinecraftVersions(gameDir),
        backend.listHorizonInstances(),
      ])
      installedVersions.value = list
      horizonInstances.value = hList

      if (currentHorizonInstance.value) {
        const hDecision = pruneHorizonInstance(currentHorizonInstance.value, hList)
        currentHorizonInstance.value = hDecision.nextInstance
        if (hDecision.nextInstance) {
          currentVersion.value = null
          return
        }
        if (hDecision.notice && onNotice) {
          onNotice(hDecision.notice)
        }
      }

      const decision = pruneSelectedVersion(currentVersion.value, list)
      currentVersion.value = decision.nextVersion
      if (!currentVersion.value && hList.length > 0) {
        currentHorizonInstance.value = hList[0]
      }
      if (decision.notice && onNotice && !currentHorizonInstance.value) {
        onNotice(decision.notice)
      }
    } catch (e) {
      console.warn('获取已安装版本失败:', e)
    }
  }

  const setCurrentVersion = (version: VersionInfo | null) => {
    currentVersion.value = version
    if (version) {
      currentHorizonInstance.value = null
    }
  }

  const setCurrentHorizonInstance = (instance: HorizonInstance | null) => {
    currentHorizonInstance.value = instance
    if (instance) {
      currentVersion.value = null
    }
  }

  const triggerInstanceRefresh = () => {
    instanceRefreshTrigger.value++
  }

  return {
    installedVersions,
    currentVersion,
    horizonInstances,
    currentHorizonInstance,
    instanceRefreshTrigger,
    refreshInstalledVersions,
    setCurrentVersion,
    setCurrentHorizonInstance,
    triggerInstanceRefresh,
  }
})
