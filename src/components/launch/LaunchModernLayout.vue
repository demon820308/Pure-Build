<script setup lang="ts">
import { ref } from 'vue'
import { useAppStore } from '@/stores/app'
import { backend } from '@/services/backend'
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
  isScanning: boolean
  runningPid: number | null
  launchProgress: LaunchProgress | null
  launchMessage: { type: 'success' | 'error' | 'info'; text: string } | null
}>()

const emit = defineEmits<{
  (e: 'update:selectedJava', val: JavaInfo | null): void
  (e: 'openLoginModal', mode: 'msa' | 'yggdrasil'): void
  (e: 'launch'): void
  (e: 'killGame'): void
  (e: 'refresh'): void
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
  <div class="flex-1 h-full flex flex-col gap-4 overflow-hidden">
    <div class="flex-1 grid grid-cols-12 gap-4 overflow-hidden">
      <div class="col-span-7 flex flex-col gap-3 overflow-y-auto pr-1">
        <MyHint
          v-if="launchMessage"
          :type="launchMessage.type"
          @close="emit('closeMessage')"
        >
          {{ launchMessage.text }}
        </MyHint>

        <div class="rounded-xl bg-pcl-card border border-pcl-border px-4 py-3 flex items-center justify-between shadow-pcl-card shrink-0">
          <div class="space-y-1 min-w-0 pr-3">
            <div class="flex items-center gap-2">
              <span class="text-sm font-semibold text-pcl-text truncate">
                {{ store.customHomepageNotice ? '主页看板' : 'Pure Build 工作台' }}
              </span>
              <span class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 text-[11px] font-medium shrink-0">
                <span class="w-1.5 h-1.5 rounded-full bg-emerald-500" />
                已就绪
              </span>
            </div>
            <div v-if="store.customHomepageNotice" class="text-xs text-pcl-subtext whitespace-pre-wrap leading-relaxed">
              {{ store.customHomepageNotice }}
            </div>
            <div v-else class="text-xs text-pcl-subtext flex items-center gap-1.5 truncate">
              <span class="shrink-0">工作目录：</span>
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
            <PclIcon name="folder" :size="13" class="mr-1" />
            打开目录
          </MyButton>
        </div>

        <MyCard
          title="当前游戏实例"
          :subtitle="`已扫描到 ${store.installedVersions.length} 个本地客户端`"
          :can-collapse="false"
        >
          <template #extra>
            <MyButton type="text" size="small" :loading="isScanning" @click="emit('refresh')">
              <PclIcon name="refresh" :size="12" class="mr-1" />
              刷新
            </MyButton>
          </template>

          <div
            v-if="store.currentVersion"
            class="flex items-center justify-between px-3.5 py-2.5 rounded-xl bg-black/5 dark:bg-black/25 border border-pcl-border"
          >
            <div class="flex items-center gap-3 overflow-hidden pr-2">
              <div class="w-9 h-9 rounded-lg bg-black/5 dark:bg-white/5 border border-pcl-border text-pcl-primary flex items-center justify-center shrink-0">
                <PclIcon name="instance" :size="18" />
              </div>
              <div class="truncate">
                <div class="font-semibold text-sm text-pcl-text truncate">
                  {{ store.currentVersion.name }}
                </div>
                <div class="text-xs text-pcl-subtext mt-0.5 flex items-center gap-2">
                  <span class="bg-black/5 dark:bg-white/10 text-pcl-text px-1.5 py-0.2 rounded font-mono text-[11px]">
                    {{ store.currentVersion.loader || 'Vanilla' }}
                  </span>
                  <span class="truncate">{{ store.currentVersion.has_indie ? '独立版本隔离' : '通用目录' }}</span>
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
                <PclIcon name="gear" :size="12" class="mr-1" />
                实例管理
              </MyButton>
              <MyButton
                type="primary"
                size="small"
                @click="showVersionPicker = !showVersionPicker"
              >
                {{ showVersionPicker ? '收起列表' : '切换版本' }}
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

        <JavaSelectPanel
          ref="javaPanelRef"
          :model-value="selectedJava"
          layout="modern"
          @update:model-value="emit('update:selectedJava', $event)"
        />
      </div>

      <div class="col-span-5 flex flex-col gap-3.5 overflow-y-auto pr-0.5">
        <MyCard
          title="玩家身份"
          :subtitle="store.hideYggdrasilLogin ? '离线登录与微软正版授权' : '离线登录、微软正版与外置登录'"
          :can-collapse="false"
        >
          <div
            class="grid gap-1 p-1 rounded-lg bg-black/5 dark:bg-black/30 border border-pcl-border mb-3"
            :class="store.hideYggdrasilLogin ? 'grid-cols-2' : 'grid-cols-3'"
          >
            <button
              class="h-7 rounded-md text-[11px] font-medium transition-colors cursor-pointer flex items-center justify-center gap-1.5"
              :class="
                !store.userProfile
                  ? 'bg-pcl-primary text-white shadow-xs'
                  : 'text-pcl-subtext hover:text-pcl-text'
              "
              @click="store.setUserProfile(null)"
            >
              <PclIcon name="offline" :size="13" />
              离线登录
            </button>
            <button
              class="h-7 rounded-md text-[11px] font-medium transition-colors cursor-pointer flex items-center justify-center gap-1.5"
              :class="
                store.userProfile && store.userProfile.auth_type !== 'yggdrasil'
                  ? 'bg-pcl-primary text-white shadow-xs'
                  : 'text-pcl-subtext hover:text-pcl-text'
              "
              @click="emit('openLoginModal', 'msa')"
            >
              <PclIcon name="msa" :size="13" />
              微软登录
            </button>
            <button
              v-if="!store.hideYggdrasilLogin"
              class="h-7 rounded-md text-[11px] font-medium transition-colors cursor-pointer flex items-center justify-center gap-1.5"
              :class="
                store.userProfile?.auth_type === 'yggdrasil'
                  ? 'bg-pcl-primary text-white shadow-xs'
                  : 'text-pcl-subtext hover:text-pcl-text'
              "
              @click="emit('openLoginModal', 'yggdrasil')"
            >
              <PclIcon name="yggdrasil" :size="13" />
              外置登录
            </button>
          </div>

          <div class="flex items-center gap-3 p-3 rounded-xl bg-black/5 dark:bg-[#181c24] border border-pcl-border">
            <img
              v-if="store.userProfile"
              :src="store.userProfile.avatar_url"
              class="w-10 h-10 rounded-lg bg-black/10 shrink-0 border border-pcl-border"
            />
            <div
              v-else
              class="w-10 h-10 rounded-lg bg-black/5 dark:bg-white/5 border border-pcl-border text-pcl-subtext flex items-center justify-center shrink-0"
            >
              <PclIcon name="offline" :size="18" />
            </div>

            <div class="overflow-hidden flex-1">
              <div v-if="store.userProfile" class="font-semibold text-sm text-pcl-text truncate">
                {{ store.playerName }}
              </div>
              <div v-else class="space-y-1">
                <div class="flex items-center justify-between text-[11px]">
                  <span class="text-pcl-subtext font-medium">离线游戏昵称</span>
                </div>
                <input
                  v-model="store.playerName"
                  class="w-full h-8 px-2.5 rounded-md text-xs text-pcl-text bg-[var(--pcl-color-bg)] border border-pcl-border focus:border-pcl-primary outline-none transition-colors font-medium"
                  placeholder="输入游戏昵称 (如 Steve)"
                  @change="store.saveCurrentConfig"
                />
              </div>

              <div v-if="store.userProfile" class="text-[11px] mt-1 flex items-center gap-1.5">
                <span class="px-1.5 py-0.2 rounded font-medium text-[10px] bg-emerald-500/10 text-emerald-600 dark:text-emerald-400">
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
              <PclIcon name="offline" :size="12" class="mr-1" />
              切回离线登录
            </MyButton>
          </div>
        </MyCard>

        <MyCard title="主题色选择" subtitle="动态计算界面调色板" :default-collapsed="true">
          <div class="flex gap-2.5 flex-wrap pt-1">
            <button
              v-for="hue in [205, 145, 270, 350, 35, 175]"
              :key="hue"
              class="w-6 h-6 rounded-full border border-black/10 dark:border-white/20 cursor-pointer transition-transform hover:scale-105 active:scale-95"
              :style="{ backgroundColor: `hsl(${hue}, 85%, 52%)` }"
              @click="store.setThemeHue(hue)"
            />
          </div>
        </MyCard>
      </div>
    </div>

    <div
      class="h-[48px] rounded-xl bg-pcl-card border px-3.5 flex items-center justify-between gap-3 shrink-0 shadow-pcl-card transition-colors"
      :class="runningPid ? 'border-emerald-500/40' : 'border-pcl-border'"
    >
      <div v-if="launchProgress && !launchProgress.error" class="flex-1 min-w-0 space-y-1">
        <div class="flex items-center justify-between text-xs">
          <span class="font-medium text-pcl-text flex items-center gap-2 truncate">
            <PclIcon v-if="!runningPid" name="refresh" :size="12" class="animate-spin text-pcl-primary" />
            <span v-else class="w-2 h-2 rounded-full bg-emerald-500 inline-block" />
            <span class="truncate">
              {{ runningPid ? `运行中 (PID: ${runningPid}) · ${launchProgress.step}` : launchProgress.step }}
            </span>
          </span>
          <div class="flex items-center gap-3 shrink-0">
            <span class="font-medium font-mono tabular-nums" :class="runningPid ? 'text-emerald-500' : 'text-pcl-primary'">
              {{ launchProgress.percentage.toFixed(0) }}%
            </span>
            <button
              v-if="runningPid"
              class="text-xs text-red-500 hover:text-red-400 font-medium cursor-pointer flex items-center gap-1"
              @click="emit('killGame')"
            >
              <PclIcon name="stop" :size="11" />
              结束进程
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
        <div class="flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-black/5 dark:bg-black/25 border border-pcl-border truncate text-pcl-subtext">
          <PclIcon name="instance" :size="12" />
          <span class="font-medium text-pcl-text truncate max-w-[180px]">
            {{ store.currentVersion?.name || '未选择实例' }}
          </span>
        </div>
        <div class="flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-black/5 dark:bg-black/25 border border-pcl-border text-pcl-subtext shrink-0">
          <PclIcon name="gear" :size="12" />
          <span class="font-mono">{{ selectedJava ? `Java ${selectedJava.version} (${selectedJava.arch || '64-Bit'})` : '未检测 Java' }}</span>
        </div>
        <div class="flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-black/5 dark:bg-black/25 border border-pcl-border text-pcl-subtext truncate">
          <PclIcon name="offline" :size="12" />
          <span class="truncate max-w-[120px]">
            {{ store.playerName || 'Steve' }} ({{ store.userProfile ? '正版' : '离线' }})
          </span>
        </div>
      </div>

      <button
        :disabled="isLaunching || !store.currentVersion"
        class="h-8 px-5 rounded-md text-white text-xs font-medium shadow-xs transition-colors duration-150 active:scale-98 cursor-pointer flex items-center justify-center gap-1.5 select-none disabled:opacity-50 shrink-0"
        :class="
          runningPid
            ? 'bg-emerald-600 hover:bg-emerald-500'
            : 'bg-pcl-primary hover:bg-pcl-hover active:bg-pcl-active'
        "
        @click="emit('launch')"
      >
        <PclIcon v-if="isLaunching" name="refresh" :size="13" class="animate-spin" />
        <PclIcon v-else-if="runningPid" name="play" :size="13" />
        <PclIcon v-else name="launch" :size="13" />
        <span>
          {{
            isLaunching
              ? `启动中 (${launchProgress?.percentage.toFixed(0) || 10}%)`
              : runningPid
                ? '运行中 (可多开)'
                : '启动游戏'
          }}
        </span>
      </button>
    </div>
  </div>
</template>
