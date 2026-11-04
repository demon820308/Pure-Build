<script setup lang="ts">
import { ref } from 'vue'
import { storeToRefs } from 'pinia'
import { useAppStore } from '@/stores/app'
import LoginModal from '@/components/auth/LoginModal.vue'
import LaunchClassicLayout from '@/components/launch/LaunchClassicLayout.vue'
import LaunchModernLayout from '@/components/launch/LaunchModernLayout.vue'
import { useLaunch } from '@/composables/useLaunch'
import type { JavaInfo } from '@/services/backend'

const store = useAppStore()
const { isLaunching, launchProgress, runningPid } = storeToRefs(store)

const selectedJava = ref<JavaInfo | null>(null)
const layoutRef = ref<{ scanJavaRuntime: () => Promise<void> } | null>(null)
const showLoginModal = ref(false)
const loginMode = ref<'msa' | 'yggdrasil'>('msa')

const {
  isScanning,
  launchMessage,
  loadData,
  handleLaunch,
  handleKillGame,
} = useLaunch({ selectedJava, javaPanelRef: layoutRef })

const openLoginModal = (mode: 'msa' | 'yggdrasil') => {
  loginMode.value = mode
  showLoginModal.value = true
}
</script>

<template>
  <div class="h-full flex relative" :class="store.uiStyle === 'classic' ? 'gap-0' : 'gap-5'">
    <LaunchClassicLayout
      v-if="store.uiStyle === 'classic'"
      ref="layoutRef"
      v-model:selected-java="selectedJava"
      :is-launching="isLaunching"
      :running-pid="runningPid"
      :launch-progress="launchProgress"
      :launch-message="launchMessage"
      @open-login-modal="openLoginModal"
      @launch="handleLaunch"
      @kill-game="handleKillGame"
      @close-message="launchMessage = null"
    />

    <LaunchModernLayout
      v-else
      ref="layoutRef"
      v-model:selected-java="selectedJava"
      :is-launching="isLaunching"
      :is-scanning="isScanning"
      :running-pid="runningPid"
      :launch-progress="launchProgress"
      :launch-message="launchMessage"
      @open-login-modal="openLoginModal"
      @launch="handleLaunch"
      @kill-game="handleKillGame"
      @refresh="loadData"
      @close-message="launchMessage = null"
    />

    <LoginModal v-model="showLoginModal" :mode="loginMode" />
  </div>
</template>
