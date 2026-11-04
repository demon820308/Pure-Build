<script setup lang="ts">
import { ref, watch } from 'vue'
import MyButton from '@/components/base/MyButton.vue'
import { backend } from '@/services/backend'
import type { DeviceCodeResponse, UserProfile } from '@/services/backend'
import { useSessionStore } from '@/stores/session'

const props = withDefaults(
  defineProps<{
    modelValue: boolean
    mode?: 'msa' | 'yggdrasil'
  }>(),
  {
    mode: 'msa',
  }
)

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void
  (e: 'success', profile: UserProfile): void
}>()

const session = useSessionStore()

const currentMode = ref<'msa' | 'yggdrasil'>('msa')
const isRequestingCode = ref(false)
const isPollingAuth = ref(false)
const deviceAuth = ref<DeviceCodeResponse | null>(null)
const codeCopiedHint = ref(false)

const yggApiRoot = ref('')
const yggUsername = ref('')
const yggPassword = ref('')
const isLoggingInYgg = ref(false)

watch(
  () => props.modelValue,
  (val) => {
    if (val) {
      currentMode.value = props.mode
      codeCopiedHint.value = false
      if (props.mode === 'msa') {
        void startMicrosoftLogin()
      }
    }
  }
)

const closeModal = () => {
  emit('update:modelValue', false)
}

const startMicrosoftLogin = async () => {
  isRequestingCode.value = true
  deviceAuth.value = null
  try {
    const res = await backend.getMicrosoftDeviceCode()
    deviceAuth.value = res
    isRequestingCode.value = false
    isPollingAuth.value = true

    const profile = await backend.pollMicrosoftToken(res.device_code)
    await session.setUserProfile(profile)
    emit('success', profile)
    closeModal()
    alert(`🎉 微软正版账号登录成功！\n欢迎，${profile.name}`)
  } catch (err: any) {
    alert(`登录未完成: ${err}`)
  } finally {
    isRequestingCode.value = false
    isPollingAuth.value = false
  }
}

const copyCodeAndOpenUrl = async () => {
  if (!deviceAuth.value) return
  try {
    await navigator.clipboard.writeText(deviceAuth.value.user_code)
    codeCopiedHint.value = true
  } catch (e) {
    console.warn('剪贴板写入失败:', e)
  }
  const targetUrl = deviceAuth.value.verification_uri
  try {
    await backend.openUrl(targetUrl)
  } catch (err) {
    console.warn('调用系统浏览器打开链接失败:', err)
    alert(`无法自动唤起浏览器，请手动在浏览器访问：\n${targetUrl}\n并输入配对码：${deviceAuth.value.user_code}`)
  }
}

const handleYggdrasilLogin = async () => {
  if (!yggUsername.value.trim() || !yggPassword.value) {
    alert('请输入外置皮肤站邮箱/账号与密码')
    return
  }
  isLoggingInYgg.value = true
  try {
    const profile = await backend.loginYggdrasilAccount({
      apiRoot: yggApiRoot.value,
      username: yggUsername.value.trim(),
      password: yggPassword.value,
    })
    await session.setUserProfile(profile)
    emit('success', profile)
    closeModal()
    yggPassword.value = ''
    alert(`🎉 外置账号登录成功！\n角色：${profile.name}`)
  } catch (err: any) {
    alert(`外置登录失败: ${err}`)
  } finally {
    isLoggingInYgg.value = false
  }
}
</script>

<template>
  <div
    v-if="modelValue"
    class="absolute inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center p-6 z-50 rounded-2xl animate-fade-in"
  >
    <div class="w-96 bg-pcl-card rounded-2xl border border-pcl-border shadow-2xl p-6 flex flex-col gap-4">
      <div class="flex items-center justify-between">
        <h3 class="font-bold text-base text-pcl-text flex items-center gap-2">
          <span>{{ currentMode === 'msa' ? '🔑 微软账号正版登录' : '🌐 第三方外置皮肤站登录' }}</span>
        </h3>
        <button class="text-xs text-pcl-subtext hover:text-pcl-text cursor-pointer" @click="closeModal">
          ✕
        </button>
      </div>

      <div v-if="currentMode === 'yggdrasil'" class="space-y-3 text-xs">
        <div class="flex flex-col gap-1">
          <span class="text-pcl-subtext">Yggdrasil 验证服务器地址 (如 LittleSkin)：</span>
          <input
            v-model="yggApiRoot"
            type="text"
            class="px-3 py-2 rounded-xl bg-black/5 dark:bg-white/5 border border-pcl-border text-pcl-text outline-none focus:border-pcl-primary font-mono"
          />
        </div>
        <div class="flex flex-col gap-1">
          <span class="text-pcl-subtext">邮箱或用户名：</span>
          <input
            v-model="yggUsername"
            type="text"
            placeholder="输入皮肤站注册邮箱"
            class="px-3 py-2 rounded-xl bg-black/5 dark:bg-white/5 border border-pcl-border text-pcl-text outline-none focus:border-pcl-primary"
          />
        </div>
        <div class="flex flex-col gap-1">
          <span class="text-pcl-subtext">密码：</span>
          <input
            v-model="yggPassword"
            type="password"
            placeholder="输入密码"
            class="px-3 py-2 rounded-xl bg-black/5 dark:bg-white/5 border border-pcl-border text-pcl-text outline-none focus:border-pcl-primary"
            @keyup.enter="handleYggdrasilLogin"
          />
        </div>
        <MyButton type="primary" class="w-full py-2.5 mt-2" :loading="isLoggingInYgg" @click="handleYggdrasilLogin">
          登录并绑定角色
        </MyButton>
      </div>

      <div v-else-if="isRequestingCode" class="py-8 flex flex-col items-center justify-center gap-3 text-xs text-pcl-subtext">
        <span class="animate-spin text-2xl">⏳</span>
        <span>正在连接微软认证服务器...</span>
      </div>

      <div v-else-if="deviceAuth" class="space-y-4">
        <p class="text-xs text-pcl-subtext leading-relaxed">
          请点击下方按钮复制配对码，并在打开的微软官方页面中完成登录授权：
        </p>

        <div class="p-3.5 rounded-xl bg-black/5 dark:bg-white/5 border border-pcl-primary/30 flex items-center justify-between">
          <span class="text-xs text-pcl-subtext">配对码：</span>
          <span class="text-lg font-mono font-bold text-pcl-primary tracking-widest">
            {{ deviceAuth.user_code }}
          </span>
        </div>

        <MyButton type="primary" class="w-full py-2.5" @click="copyCodeAndOpenUrl">
          {{ codeCopiedHint ? '✔ 已复制配对码！点击可再次打开微软页面' : '📋 复制配对码并打开微软页面' }}
        </MyButton>

        <div class="text-[11px] text-center text-pcl-subtext">
          验证地址：<span class="font-mono select-all text-pcl-primary">{{ deviceAuth.verification_uri }}</span>
        </div>

        <div v-if="isPollingAuth" class="flex items-center justify-center gap-2 text-[11px] text-pcl-subtext pt-1">
          <span class="animate-spin">⚙️</span>
          <span>等待你在浏览器中完成验证...</span>
        </div>
      </div>
    </div>
  </div>
</template>
