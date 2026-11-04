<script setup lang="ts">
import { ref } from 'vue'
import { useAppStore } from '@/stores/app'
import { backend } from '@/services/backend'
import MyCard from '@/components/base/MyCard.vue'
import MyButton from '@/components/base/MyButton.vue'
import MyHint from '@/components/base/MyHint.vue'
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
            <MyButton type="text" size="small" :loading="isScanning" @click="emit('refresh')">
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
              @click="emit('openLoginModal', 'msa')"
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
              @click="emit('openLoginModal', 'yggdrasil')"
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
              @click="emit('killGame')"
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
        @click="emit('launch')"
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
