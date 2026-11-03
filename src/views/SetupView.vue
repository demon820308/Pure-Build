<script setup lang="ts">
import { useAppStore } from '@/stores/app'
import MyCard from '@/components/base/MyCard.vue'
import MyButton from '@/components/base/MyButton.vue'
import MyListItem from '@/components/base/MyListItem.vue'
import { backend } from '@/services/backend'
import type { JavaInfo } from '@/services/backend'
import { open } from '@tauri-apps/plugin-dialog'
import { ref, onMounted, onUnmounted } from 'vue'
import { EQ_PRESETS, EQ_BANDS } from '@/services/audioEngine'

const store = useAppStore()
const javaList = ref<JavaInfo[]>([])
const isLoadingJava = ref(false)
const saveSuccess = ref(false)
// 原版 PageSetupLeft.xaml 左侧分类 ('launch' 启动 | 'ui' 个性化 | 'system' 其他/环境)
const setupTab = ref<'all' | 'launch' | 'ui' | 'system'>('all')

// 实时 7 频段动能频谱指示器
const spectrumBars = ref<number[]>([12, 12, 12, 12, 12, 12, 12])
let animFrameId: number | null = null
const spectrumRaw = new Uint8Array(32)

const updateSpectrumLoop = () => {
  if (store.isMusicPlaying) {
    store.getSpectrumData(spectrumRaw)
    const indices = [1, 3, 6, 10, 15, 22, 28]
    spectrumBars.value = indices.map((idx) => {
      const val = spectrumRaw[idx] || 0
      return Math.max(12, Math.round((val / 255) * 100))
    })
  } else {
    spectrumBars.value = [12, 12, 12, 12, 12, 12, 12]
  }
  animFrameId = requestAnimationFrame(updateSpectrumLoop)
}

const refreshJava = async () => {
  isLoadingJava.value = true
  try {
    javaList.value = await backend.getInstalledJava()
  } finally {
    isLoadingJava.value = false
  }
}

onMounted(() => {
  refreshJava()
  animFrameId = requestAnimationFrame(updateSpectrumLoop)
})

onUnmounted(() => {
  if (animFrameId !== null) {
    cancelAnimationFrame(animFrameId)
  }
})

const handleSelectGameDir = async () => {
  try {
    const selected = await open({
      directory: true,
      multiple: false,
      title: '选择 .minecraft 游戏目录',
      defaultPath: store.gameDir,
    })

    if (selected && typeof selected === 'string') {
      store.gameDir = selected
      await store.saveCurrentConfig()
      saveSuccess.value = true
      setTimeout(() => {
        saveSuccess.value = false
      }, 2000)
    }
  } catch (err: any) {
    console.warn('选择目录失败:', err)
  }
}

const handleSave = async () => {
  await store.saveCurrentConfig()
  saveSuccess.value = true
  setTimeout(() => {
    saveSuccess.value = false
  }, 2000)
}

const themePresets = [
  { name: '像素蓝', hue: 205 },
  { name: '祖母绿', hue: 145 },
  { name: '极客紫', hue: 270 },
  { name: '樱花粉', hue: 345 },
  { name: '琥珀橙', hue: 35 },
  { name: '青碧色', hue: 175 },
]

const bgFileInput = ref<HTMLInputElement | null>(null)
const handleLocalWallpaperFile = (e: Event) => {
  const input = e.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) return
  const reader = new FileReader()
  reader.onload = () => {
    if (typeof reader.result === 'string') {
      store.customBgUrl = reader.result
      store.saveCurrentConfig()
    }
  }
  reader.readAsDataURL(file)
}

const handleAddMusicFolder = async () => {
  try {
    const selected = await open({
      directory: true,
      multiple: true,
      title: '选择要添加的背景音乐文件夹（自动递归扫描子文件夹）',
    })
    if (!selected) return
    if (Array.isArray(selected)) {
      for (const dir of selected) {
        if (typeof dir === 'string') {
          await store.addMusicFolder(dir)
        }
      }
    } else if (typeof selected === 'string') {
      await store.addMusicFolder(selected)
    }
  } catch (err) {
    console.warn('选择音乐文件夹失败:', err)
  }
}

const handleOpenDefaultMusicDir = async () => {
  const dir = store.platformInfo?.default_music_dir
  if (dir) {
    await backend.openFolder(dir)
  }
}
</script>

<template>
  <div class="h-full flex overflow-hidden" :class="store.uiStyle === 'classic' ? 'flex-row gap-0' : 'flex-col gap-4'">
    <div
      v-if="store.uiStyle === 'classic'"
      class="w-[152px] h-full bg-white dark:bg-[#1e232b] border-r border-black/8 dark:border-white/10 pt-3 flex flex-col shrink-0 select-none"
    >
      <MyListItem
        title="全部设置"
        icon="more"
        :checked="setupTab === 'all'"
        @click="setupTab = 'all'"
      />
      <MyListItem
        title="启动"
        icon="launch"
        :checked="setupTab === 'launch'"
        @click="setupTab = 'launch'"
      />
      <MyListItem
        title="个性化"
        icon="shader"
        :checked="setupTab === 'ui'"
        @click="setupTab = 'ui'"
      />
      <MyListItem
        title="其他与环境"
        icon="setup"
        :checked="setupTab === 'system'"
        @click="setupTab = 'system'"
      />
    </div>

    <div
      class="flex-1 h-full flex flex-col gap-4 overflow-y-auto"
      :class="store.uiStyle === 'classic' ? 'p-5' : 'pr-1'"
    >
    <div class="flex items-center justify-between gap-4 shrink-0">
      <div class="flex items-center gap-3">
        <div class="w-10 h-10 rounded-xl bg-pcl-primary/20 flex items-center justify-center text-lg">
          ⚙️
        </div>
        <div>
          <h2 class="text-base font-bold text-pcl-text flex items-center gap-2">
            <span>全局启动与常规设置</span>
            <span v-if="saveSuccess" class="text-xs bg-emerald-500/10 text-emerald-600 px-2 py-0.5 rounded font-normal animate-pulse">
              ✔ 配置已持久化保存
            </span>
          </h2>
          <p class="text-xs text-pcl-subtext mt-0.5">
            配置自动保存至本地 pcl_config.json，重启启动器后依然生效
          </p>
        </div>
      </div>

      <MyButton type="primary" size="small" @click="handleSave">
        💾 保存设置
      </MyButton>
    </div>

    <div class="space-y-4">
      <MyCard
        v-show="store.uiStyle === 'modern' || setupTab === 'all' || setupTab === 'launch'"
        title="Minecraft 游戏目录 (.minecraft)"
        subtitle="启动器扫描版本、Mod 与配置的根目录"
      >
        <div class="space-y-3">
          <div class="flex items-center gap-2">
            <input
              v-model="store.gameDir"
              class="flex-1 px-3.5 py-2 rounded-xl bg-black/5 dark:bg-white/5 border border-pcl-border text-xs text-pcl-text outline-none focus:border-pcl-primary transition-colors font-mono"
              @change="store.saveCurrentConfig"
            />
            <MyButton type="secondary" size="small" @click="handleSelectGameDir">
              📁 浏览...
            </MyButton>
          </div>
          <p class="text-[11px] text-pcl-subtext">
            💡 支持任意目录位置。点击“浏览”可唤起系统原生文件夹选择窗口，修改后主界面与版本管理页面将自动重新扫描。
          </p>
        </div>
      </MyCard>

      <MyCard
        v-show="store.uiStyle === 'modern' || setupTab === 'all' || setupTab === 'system'"
        title="下载源与镜像加速"
        subtitle="切换官方源与 BMCLAPI 国内高速镜像源优先级"
      >
        <div class="flex items-center justify-between">
          <div class="text-xs text-pcl-subtext">
            启用后优先使用镜像源下载游戏核心与支持库，失败时自动回退至 Mojang/Fabric 官方源
          </div>
          <label class="flex items-center gap-2 text-xs font-medium text-pcl-text cursor-pointer select-none">
            <input
              v-model="store.preferMirror"
              type="checkbox"
              class="accent-[var(--pcl-color-primary)] cursor-pointer"
              @change="store.saveCurrentConfig"
            />
            <span>{{ store.preferMirror ? '优先使用镜像加速 (BMCLAPI)' : '优先使用官方源 (Official)' }}</span>
          </label>
        </div>
      </MyCard>

      <MyCard
        v-show="store.uiStyle === 'modern' || setupTab === 'all' || setupTab === 'launch' || setupTab === 'ui'"
        title="登录方式显示设置"
        subtitle="控制启动页是否显示第三方外置登录（Authlib-Injector / LittleSkin）入口"
      >
        <div class="flex items-center justify-between gap-4">
          <div class="text-xs text-pcl-subtext">
            默认隐藏外置登录入口，使启动页仅保留「离线登录」与「微软正版」；如需使用第三方皮肤站登录可随时取消勾选
          </div>
          <label class="flex items-center gap-2 text-xs font-medium text-pcl-text cursor-pointer select-none shrink-0">
            <input
              v-model="store.hideYggdrasilLogin"
              type="checkbox"
              class="accent-[var(--pcl-color-primary)] cursor-pointer"
              @change="
                () => {
                  if (store.hideYggdrasilLogin && store.userProfile?.auth_type === 'yggdrasil') {
                    store.setUserProfile(null)
                  }
                  store.saveCurrentConfig()
                }
              "
            />
            <span>{{ store.hideYggdrasilLogin ? '已隐藏外置登录 (默认)' : '显示外置登录入口' }}</span>
          </label>
        </div>
      </MyCard>

      <MyCard
        v-show="store.uiStyle === 'modern' || setupTab === 'all' || setupTab === 'launch'"
        title="游戏最大运行内存 (Java Heap)"
        subtitle="根据你机器的物理内存配置合理的上限"
      >
        <div class="space-y-4">
          <div class="flex items-center justify-between text-sm">
            <span class="text-pcl-subtext">当前分配</span>
            <span class="font-bold text-base text-pcl-primary">
              {{ store.maxMemoryMb }} MB ({{ (store.maxMemoryMb / 1024).toFixed(1) }} GB)
            </span>
          </div>

          <div class="space-y-2">
            <input
              type="range"
              min="1024"
              max="16384"
              step="512"
              v-model.number="store.maxMemoryMb"
              class="w-full accent-[var(--pcl-color-primary)] cursor-pointer h-2 bg-black/10 dark:bg-white/10 rounded-lg appearance-none outline-none"
              @change="store.saveCurrentConfig"
            />
            <div class="flex justify-between text-[10px] text-pcl-subtext">
              <span>1 GB</span>
              <span>4 GB (推荐)</span>
              <span>8 GB (重型整合包)</span>
              <span>16 GB</span>
            </div>
          </div>
        </div>
      </MyCard>

      <MyCard
        v-show="store.uiStyle === 'modern' || setupTab === 'all' || setupTab === 'launch'"
        title="游戏默认窗口分辨率"
        subtitle="拉起游戏时的初始窗口长宽尺寸"
      >
        <div class="grid grid-cols-2 gap-4">
          <div class="flex items-center justify-between p-3 rounded-xl bg-black/5 dark:bg-white/5 border border-pcl-border">
            <span class="text-xs text-pcl-subtext">宽度 (Width)</span>
            <input
              v-model.number="store.windowWidth"
              type="number"
              class="w-24 px-2 py-1 rounded bg-transparent border border-pcl-border text-xs text-right text-pcl-text font-mono outline-none focus:border-pcl-primary"
              @change="store.saveCurrentConfig"
            />
          </div>
          <div class="flex items-center justify-between p-3 rounded-xl bg-black/5 dark:bg-white/5 border border-pcl-border">
            <span class="text-xs text-pcl-subtext">高度 (Height)</span>
            <input
              v-model.number="store.windowHeight"
              type="number"
              class="w-24 px-2 py-1 rounded bg-transparent border border-pcl-border text-xs text-right text-pcl-text font-mono outline-none focus:border-pcl-primary"
              @change="store.saveCurrentConfig"
            />
          </div>
        </div>
      </MyCard>

      <MyCard
        v-show="store.uiStyle === 'modern' || setupTab === 'all' || setupTab === 'ui'"
        title="个性化视觉与界面布局风格"
        subtitle="随时切换 1:1 经典布局与现代侧栏布局，定制 8 阶主题色与壁纸"
      >
        <div class="space-y-4 text-xs">
          <div class="space-y-2">
            <span class="text-pcl-subtext font-medium block">UI 界面风格与布局架构（点击立即热切换并自动保存）</span>
            <div class="grid grid-cols-2 gap-3">
              <div
                class="p-3 rounded-[5px] border cursor-pointer transition-all flex items-start gap-3"
                :class="
                  store.uiStyle === 'classic'
                    ? 'border-[var(--pcl-brush-3)] bg-[var(--pcl-brush-7)] ring-1 ring-[var(--pcl-brush-3)]'
                    : 'border-pcl-border bg-black/5 dark:bg-white/5 hover:border-pcl-primary/50'
                "
                @click="store.setUiStyle('classic')"
              >
                <span class="text-lg">🏛️</span>
                <div>
                  <div class="font-bold text-pcl-text flex items-center gap-1.5">
                    <span>PBL 经典布局 (Classic)</span>
                    <span
                      v-if="store.uiStyle === 'classic'"
                      class="text-[10px] px-1.5 py-0.2 rounded bg-[var(--pcl-brush-3)] text-white"
                    >
                      当前使用
                    </span>
                  </div>
                  <p class="text-[11px] text-pcl-subtext mt-0.5 leading-relaxed">
                    100% 经典双栏分屏：52px 矢量顶栏 + 启动页 300px 左控制台/二级页面 152px 左侧栏 + 5px 可折叠卡片与细描边按钮
                  </p>
                </div>
              </div>

              <div
                class="p-3 rounded-[5px] border cursor-pointer transition-all flex items-start gap-3"
                :class="
                  store.uiStyle === 'modern'
                    ? 'border-pcl-primary bg-pcl-primary/10 ring-1 ring-pcl-primary'
                    : 'border-pcl-border bg-black/5 dark:bg-white/5 hover:border-pcl-primary/50'
                "
                @click="store.setUiStyle('modern')"
              >
                <span class="text-lg">✨</span>
                <div>
                  <div class="font-bold text-pcl-text flex items-center gap-1.5">
                    <span>现代 PBL 侧栏布局 (Modern)</span>
                    <span
                      v-if="store.uiStyle === 'modern'"
                      class="text-[10px] px-1.5 py-0.2 rounded bg-pcl-primary text-white"
                    >
                      当前使用
                    </span>
                  </div>
                  <p class="text-[11px] text-pcl-subtext mt-0.5 leading-relaxed">
                    现代桌面端设计：左侧垂直图标导航栏 + 16px 大圆角毛玻璃卡片 + 实心高对比按钮
                  </p>
                </div>
              </div>
            </div>
          </div>

          <div class="space-y-2">
            <div class="flex items-center justify-between">
              <span class="text-pcl-subtext font-medium">主题色彩方案 (HSL Hue: {{ store.themeHue }}°)</span>
              <div class="flex items-center gap-1.5">
                <button
                  v-for="preset in themePresets"
                  :key="preset.hue"
                  class="px-2 py-0.5 rounded-full text-[10px] text-white font-medium cursor-pointer transition-transform hover:scale-105"
                  :style="{ backgroundColor: `hsl(${preset.hue}, 82%, 50%)` }"
                  @click="store.setThemeHue(preset.hue)"
                >
                  {{ preset.name }}
                </button>
              </div>
            </div>
            <input
              v-model.number="store.themeHue"
              type="range"
              min="0"
              max="360"
              class="w-full accent-[var(--pcl-color-primary)] cursor-pointer h-2 bg-black/10 dark:bg-white/10 rounded-lg appearance-none outline-none"
              @input="store.setThemeHue(store.themeHue)"
            />
          </div>

          <div class="space-y-1.5">
            <label class="text-pcl-subtext font-medium block">自定义背景壁纸（支持网络图片 URL 或选择本地图片）</label>
            <div class="flex items-center gap-2">
              <input
                v-model="store.customBgUrl"
                placeholder="输入图片 URL 或点击右侧选择本地图片..."
                class="flex-1 px-3 py-1.5 rounded-xl bg-black/5 dark:bg-white/5 border border-pcl-border text-xs text-pcl-text outline-none focus:border-pcl-primary"
                @change="store.saveCurrentConfig"
              />
              <input
                ref="bgFileInput"
                type="file"
                accept="image/*"
                class="hidden"
                @change="handleLocalWallpaperFile"
              />
              <MyButton type="secondary" size="small" @click="bgFileInput?.click()">
                🖼️ 选择本地图片
              </MyButton>
              <MyButton
                v-if="store.customBgUrl"
                type="text"
                size="small"
                @click="store.customBgUrl = ''; store.saveCurrentConfig()"
              >
                清除
              </MyButton>
            </div>
          </div>

          <div class="grid grid-cols-2 gap-4">
            <div class="space-y-1">
              <div class="flex justify-between text-[11px] text-pcl-subtext">
                <span>壁纸毛玻璃模糊度</span>
                <span class="font-mono font-bold text-pcl-primary">{{ store.bgBlur }} px</span>
              </div>
              <input
                v-model.number="store.bgBlur"
                type="range"
                min="0"
                max="24"
                class="w-full accent-[var(--pcl-color-primary)] cursor-pointer h-1.5"
                @change="store.saveCurrentConfig"
              />
            </div>
            <div class="space-y-1">
              <div class="flex justify-between text-[11px] text-pcl-subtext">
                <span>界面背景遮罩浓度</span>
                <span class="font-mono font-bold text-pcl-primary">{{ store.bgOpacity }}%</span>
              </div>
              <input
                v-model.number="store.bgOpacity"
                type="range"
                min="40"
                max="98"
                class="w-full accent-[var(--pcl-color-primary)] cursor-pointer h-1.5"
                @change="store.saveCurrentConfig"
              />
            </div>
          </div>

          <div class="space-y-1.5">
            <label class="text-pcl-subtext font-medium block">自定义主页看板内容（显示在启动页右侧，留空则隐藏）</label>
            <textarea
              v-model="store.customHomepageNotice"
              rows="2"
              placeholder="输入服务器公告、每日备忘或自定义主页欢迎语..."
              class="w-full px-3 py-2 rounded-xl bg-black/5 dark:bg-white/5 border border-pcl-border text-xs text-pcl-text outline-none focus:border-pcl-primary resize-none"
              @change="store.saveCurrentConfig"
            />
          </div>
        </div>
      </MyCard>

      <MyCard
        v-show="store.uiStyle === 'modern' || setupTab === 'all' || setupTab === 'ui'"
        title="背景音乐 (BGM)"
        subtitle="支持添加多个音乐文件夹（自动扫描子文件夹）与多种循环播放模式"
      >
        <div class="space-y-4 text-xs">
          <div class="flex items-center justify-between gap-3 p-3 rounded-xl bg-black/5 dark:bg-white/5 border border-pcl-border">
            <div class="min-w-0 flex-1">
              <div class="font-bold text-pcl-text flex items-center gap-2 truncate">
                <span>{{ store.isMusicPlaying ? '🎵 正在播放：' : '⏸️ 当前曲目：' }}</span>
                <span class="text-pcl-primary truncate">{{ store.currentTrack?.name || '暂无播放曲目' }}</span>
              </div>
              <div class="text-[11px] text-pcl-subtext mt-0.5">
                已从默认音乐库及 {{ store.musicDirs.length }} 个自定义文件夹中扫描到 {{ store.musicTracks.length }} 首音频曲目
              </div>
            </div>
            <div class="flex items-center gap-1.5 shrink-0">
              <MyButton type="secondary" size="small" @click="store.playPrevTrack()">
                ⏮ 上一首
              </MyButton>
              <MyButton type="primary" size="small" @click="store.toggleMusicPlay()">
                {{ store.isMusicPlaying ? '⏸ 暂停' : '▶ 播放' }}
              </MyButton>
              <MyButton type="secondary" size="small" @click="store.playNextTrack()">
                ⏭ 下一首
              </MyButton>
            </div>
          </div>

          <div class="grid grid-cols-2 gap-4">
            <div class="space-y-1.5">
              <div class="flex justify-between text-[11px] text-pcl-subtext">
                <span>音乐播放音量</span>
                <span class="font-mono font-bold text-pcl-primary">{{ store.musicVolume }}%</span>
              </div>
              <input
                :value="store.musicVolume"
                type="range"
                min="0"
                max="100"
                class="w-full accent-[var(--pcl-color-primary)] cursor-pointer h-1.5"
                @input="(e) => store.setMusicVolume(Number((e.target as HTMLInputElement).value))"
              />
            </div>

            <div class="space-y-1.5">
              <span class="text-[11px] text-pcl-subtext block">循环播放模式</span>
              <div class="grid grid-cols-3 gap-1.5">
                <button
                  v-for="mode in [
                    { id: 'shuffle', label: '🔀 随机循环' },
                    { id: 'list', label: '🔁 列表循环' },
                    { id: 'single', label: '🔂 单曲循环' },
                  ] as const"
                  :key="mode.id"
                  class="py-1 px-2 rounded-lg border text-[11px] font-medium cursor-pointer transition-colors"
                  :class="
                    store.musicPlayMode === mode.id
                      ? 'border-[var(--pcl-brush-3)] bg-[var(--pcl-brush-3)] text-white'
                      : 'border-pcl-border bg-black/5 dark:bg-white/5 text-pcl-text hover:border-pcl-primary/50'
                  "
                  @click="store.setMusicPlayMode(mode.id)"
                >
                  {{ mode.label }}
                </button>
              </div>
            </div>
          </div>

          <div class="space-y-3 pt-3 border-t border-pcl-border">
            <div class="flex items-center justify-between">
              <div>
                <div class="font-bold text-pcl-text flex items-center gap-1.5">
                  <span>🎛️ 音频调音与均衡器 (DSP)</span>
                  <span class="text-[10px] px-1.5 py-0.5 rounded bg-pcl-primary/10 text-pcl-primary font-mono font-medium">Web Audio API</span>
                </div>
                <div class="text-[11px] text-pcl-subtext mt-0.5">
                  基于硬件级数字信号处理，提供 5 频段多阶滤波、动态防破音与声学过渡
                </div>
              </div>

              <div class="flex items-end gap-1 h-5 px-2 py-1 rounded bg-black/5 dark:bg-white/5 border border-pcl-border" title="实时频段动能分析">
                <span
                  v-for="(height, i) in spectrumBars"
                  :key="i"
                  class="w-1 rounded-full transition-all duration-75"
                  :class="store.isMusicPlaying ? 'bg-[var(--pcl-color-primary)]' : 'bg-pcl-subtext/40'"
                  :style="{ height: `${height}%` }"
                />
              </div>
            </div>

            <div class="grid grid-cols-3 gap-2">
              <button
                v-for="preset in EQ_PRESETS"
                :key="preset.id"
                class="p-2.5 rounded-xl border text-left cursor-pointer transition-all flex flex-col justify-between gap-1"
                :class="
                  store.musicEqPreset === preset.id
                    ? 'border-pcl-primary bg-pcl-primary/10 shadow-sm'
                    : 'border-pcl-border bg-black/5 dark:bg-white/5 hover:border-pcl-primary/40'
                "
                @click="store.setMusicEqPreset(preset.id)"
              >
                <div class="flex items-center justify-between">
                  <span class="font-bold text-pcl-text text-xs flex items-center gap-1">
                    <span>{{ preset.icon }}</span>
                    <span>{{ preset.name }}</span>
                  </span>
                  <span
                    v-if="store.musicEqPreset === preset.id"
                    class="text-[10px] text-pcl-primary font-bold"
                  >✓</span>
                </div>
                <div class="text-[10px] text-pcl-subtext line-clamp-1">
                  {{ preset.desc }}
                </div>
              </button>
            </div>

            <div
              v-if="store.musicEqPreset === 'custom'"
              class="p-3 rounded-xl bg-black/5 dark:bg-white/5 border border-pcl-border space-y-2.5"
            >
              <div class="flex items-center justify-between text-[11px]">
                <span class="font-bold text-pcl-text">5 频段精确增益 (-12dB ~ +12dB)</span>
                <button
                  class="text-pcl-primary hover:underline cursor-pointer text-[11px]"
                  @click="store.resetMusicEqCustom"
                >
                  重置为 0dB
                </button>
              </div>
              <div class="grid grid-cols-5 gap-2">
                <div
                  v-for="(band, idx) in EQ_BANDS"
                  :key="band.freq"
                  class="space-y-1 text-center"
                >
                  <div class="text-[10px] text-pcl-subtext truncate" :title="band.label">{{ band.label }}</div>
                  <input
                    type="range"
                    min="-12"
                    max="12"
                    step="0.5"
                    :value="store.musicEqCustom[idx]"
                    class="w-full accent-[var(--pcl-color-primary)] cursor-pointer h-1.5"
                    @input="(e) => store.setMusicEqCustomBand(idx, Number((e.target as HTMLInputElement).value))"
                  />
                  <div
                    class="text-[10px] font-mono font-medium"
                    :class="
                      store.musicEqCustom[idx] > 0
                        ? 'text-pcl-primary'
                        : store.musicEqCustom[idx] < 0
                          ? 'text-orange-500'
                          : 'text-pcl-subtext'
                    "
                  >
                    {{ store.musicEqCustom[idx] > 0 ? `+${store.musicEqCustom[idx]}` : store.musicEqCustom[idx] }}dB
                  </div>
                </div>
              </div>
            </div>

            <div class="flex flex-wrap items-center gap-6 pt-1">
              <label class="flex items-center gap-2 text-pcl-text cursor-pointer select-none" title="通过动态范围压缩器自动抑制瞬态过载，杜绝破音">
                <input
                  v-model="store.musicCompressor"
                  type="checkbox"
                  class="accent-[var(--pcl-color-primary)] cursor-pointer"
                  @change="() => store.setMusicCompressor(store.musicCompressor)"
                />
                <span class="font-medium">智能防破音 (动态压缩 Limiter)</span>
              </label>

              <label class="flex items-center gap-2 text-pcl-text cursor-pointer select-none" title="在切歌、播放/暂停时进行指数平滑淡入淡出，消除切歌爆音">
                <input
                  v-model="store.musicCrossfade"
                  type="checkbox"
                  class="accent-[var(--pcl-color-primary)] cursor-pointer"
                  @change="() => store.setMusicCrossfade(store.musicCrossfade)"
                />
                <span class="font-medium">丝滑平滑过渡 (防爆音淡入淡出)</span>
              </label>
            </div>
          </div>

          <div class="flex flex-wrap items-center gap-6 pt-1">
            <label class="flex items-center gap-2 text-pcl-text cursor-pointer select-none">
              <input
                v-model="store.musicAutoPlay"
                type="checkbox"
                class="accent-[var(--pcl-color-primary)] cursor-pointer"
                @change="store.saveCurrentConfig"
              />
              <span>打开启动器时自动开始播放</span>
            </label>
            <label class="flex items-center gap-2 text-pcl-text cursor-pointer select-none">
              <input
                v-model="store.musicPauseOnLaunch"
                type="checkbox"
                class="accent-[var(--pcl-color-primary)] cursor-pointer"
                @change="store.saveCurrentConfig"
              />
              <span>游戏启动后自动暂停播放，游戏退出后自动恢复播放</span>
            </label>
          </div>

          <div class="space-y-2 pt-1 border-t border-pcl-border">
            <div class="flex items-center justify-between">
              <span class="text-pcl-subtext font-medium">
                已挂载的音乐文件夹（支持递归扫描所有子文件夹内的 .mp3 / .flac / .wav / .ogg / .m4a）
              </span>
              <div class="flex items-center gap-2">
                <MyButton type="primary" size="small" @click="handleAddMusicFolder">
                  ➕ 添加音乐文件夹
                </MyButton>
                <MyButton type="secondary" size="small" @click="handleOpenDefaultMusicDir">
                  📂 打开默认音乐库
                </MyButton>
                <MyButton type="secondary" size="small" @click="store.refreshMusicPlaylist(true)">
                  🔄 刷新背景音乐
                </MyButton>
              </div>
            </div>

            <div class="space-y-1.5">
              <div
                v-if="store.platformInfo?.default_music_dir"
                class="flex items-center justify-between px-3 py-2 rounded-lg bg-black/5 dark:bg-white/5 border border-pcl-border text-[11px]"
              >
                <div class="flex items-center gap-2 min-w-0 flex-1">
                  <span class="px-1.5 py-0.2 rounded bg-pcl-primary/15 text-pcl-primary font-semibold shrink-0">默认库</span>
                  <span class="font-mono text-pcl-subtext truncate" :title="store.platformInfo.default_music_dir">
                    {{ store.platformInfo.default_music_dir }}
                  </span>
                </div>
                <button
                  class="text-pcl-primary hover:underline ml-2 shrink-0 cursor-pointer"
                  @click="handleOpenDefaultMusicDir"
                >
                  打开目录
                </button>
              </div>

              <div
                v-for="dir in store.musicDirs"
                :key="dir"
                class="flex items-center justify-between px-3 py-2 rounded-lg bg-black/5 dark:bg-white/5 border border-pcl-border text-[11px]"
              >
                <div class="flex items-center gap-2 min-w-0 flex-1">
                  <span class="px-1.5 py-0.2 rounded bg-emerald-500/15 text-emerald-600 dark:text-emerald-400 font-semibold shrink-0">
                    自定义
                  </span>
                  <span class="font-mono text-pcl-text truncate" :title="dir">{{ dir }}</span>
                </div>
                <div class="flex items-center gap-3 ml-2 shrink-0">
                  <button
                    class="text-pcl-primary hover:underline cursor-pointer"
                    @click="backend.openFolder(dir)"
                  >
                    打开目录
                  </button>
                  <button
                    class="text-red-500 hover:underline cursor-pointer"
                    @click="store.removeMusicFolder(dir)"
                  >
                    移除
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>
      </MyCard>

      <MyCard
        v-show="store.uiStyle === 'modern' || setupTab === 'all' || setupTab === 'system'"
        title="已检测到的系统 Java"
        subtitle="Rust 扫描到的本地运行库"
      >
        <template #extra>
          <MyButton type="text" size="small" :loading="isLoadingJava" @click="refreshJava">
            🔄 重新扫描
          </MyButton>
        </template>
        <div class="space-y-2">
          <div
            v-for="(j, idx) in javaList"
            :key="j.path"
            class="flex items-center justify-between p-2.5 rounded-xl border border-pcl-border bg-black/5 dark:bg-white/5 text-xs"
          >
            <div class="truncate">
              <div class="font-medium text-pcl-text flex items-center gap-2">
                <span>Java {{ j.version }}</span>
                <span v-if="idx === 0" class="text-[10px] bg-pcl-primary/20 text-pcl-primary px-1.5 py-0.2 rounded font-bold">默认</span>
              </div>
              <div class="text-[11px] text-pcl-subtext truncate mt-0.5" :title="j.path">{{ j.path }}</div>
            </div>
            <span class="text-emerald-500 font-bold ml-2">可用</span>
          </div>
        </div>
      </MyCard>
    </div>
    </div>
  </div>
</template>
