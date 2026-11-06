<script setup lang="ts">
import { ref } from 'vue'
import { useAppStore } from '@/stores/app'
import MyCard from '@/components/base/MyCard.vue'
import MyButton from '@/components/base/MyButton.vue'
import MyHint from '@/components/base/MyHint.vue'
import PclIcon from '@/components/base/PclIcon.vue'
import VersionPicker from '@/components/launch/VersionPicker.vue'
import JavaSelectPanel from '@/components/launch/JavaSelectPanel.vue'
import type { JavaInfo, LaunchProgress } from '@/services/backend'

const props = defineProps<{
  selectedJava: JavaInfo | null
  isLaunching: boolean
  runningPid: number | null
  launchProgress: LaunchProgress | null
  launchMessage: { type: 'success' | 'error' | 'info'; text: string } | null
}>()

const emit = defineEmits<{
  (e: 'update:selectedJava', val: JavaInfo | null): void
  (e: 'openLoginModal', mode: 'msa' | 'yggdrasil'): void
  (e: 'launch'): void
  (e: 'killGame'): void
  (e: 'closeMessage'): void
}>()

const store = useAppStore()
const showVersionPicker = ref(false)
const javaPanelRef = ref<InstanceType<typeof JavaSelectPanel> | null>(null)

defineExpose({
  scanJavaRuntime: () => javaPanelRef.value?.scanJavaRuntime(),
})
</script>

<template>
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
          @click="emit('openLoginModal', 'msa')"
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
          @click="emit('openLoginModal', 'yggdrasil')"
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
          @click="emit('launch')"
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
          <MyButton type="danger" size="small" class="w-full flex items-center justify-center gap-1.5" @click="emit('killGame')">
            <PclIcon name="stop" :size="12" />
            <span>强制关闭 Minecraft (PID: {{ runningPid }})</span>
          </MyButton>
        </div>
      </div>
    </template>
  </div>

  <div class="flex-1 h-full overflow-y-auto p-5 space-y-3.5">
    <MyHint
      v-if="launchMessage"
      :type="launchMessage.type"
      @close="emit('closeMessage')"
    >
      {{ launchMessage.text }}
    </MyHint>

    <MyCard
      :title="store.customHomepageNotice ? '主页公告' : 'Pure Build 工作台'"
      subtitle="可在「设置 → 个性化」中自定义此处的主页看板文案与背景壁纸"
    >
      <div class="text-xs text-pcl-text whitespace-pre-wrap leading-relaxed">
        {{
          store.customHomepageNotice ||
          `当前工作目录：${store.gameDir}\n已扫描到本地版本：${store.installedVersions.length} 个 | 可用 Java 运行时：${javaPanelRef?.javaList.length || 0} 个。\n点击左侧面板底部的齿轮按钮可进入单版本 Mod、存档与整合包管理。`
        }}
      </div>
    </MyCard>

    <JavaSelectPanel
      ref="javaPanelRef"
      :model-value="selectedJava"
      layout="classic"
      @update:model-value="emit('update:selectedJava', $event)"
    />

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
