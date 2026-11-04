<script setup lang="ts">
import { useAppStore } from '@/stores/app'
import MyCard from '@/components/base/MyCard.vue'
import MyButton from '@/components/base/MyButton.vue'
import MyListItem from '@/components/base/MyListItem.vue'
import SetupMusicCard from '@/components/setup/SetupMusicCard.vue'
import { backend } from '@/services/backend'
import type { JavaInfo } from '@/services/backend'
import { open } from '@tauri-apps/plugin-dialog'
import { ref, onMounted } from 'vue'

const store = useAppStore()
const javaList = ref<JavaInfo[]>([])
const isLoadingJava = ref(false)
const saveSuccess = ref(false)
// 原版 PageSetupLeft.xaml 左侧分类 ('launch' 启动 | 'ui' 个性化 | 'system' 其他/环境)
const setupTab = ref<'all' | 'launch' | 'ui' | 'system'>('all')

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

      <SetupMusicCard :visible="store.uiStyle === 'modern' || setupTab === 'all' || setupTab === 'ui'" />

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
