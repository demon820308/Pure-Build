<script setup lang="ts">
import { useAppStore } from '@/stores/app'
import MyCard from '@/components/base/MyCard.vue'
import MyButton from '@/components/base/MyButton.vue'
import { backend } from '@/services/backend'
import type { JavaInfo } from '@/services/backend'
import { open } from '@tauri-apps/plugin-dialog'
import { ref, onMounted } from 'vue'

const store = useAppStore()
const javaList = ref<JavaInfo[]>([])
const isLoadingJava = ref(false)
const saveSuccess = ref(false)

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
</script>

<template>
  <div class="h-full flex flex-col gap-4 overflow-y-auto pr-1">
    <!-- 顶部状态栏 -->
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
      <!-- 游戏目录选择 (支持文件夹浏览弹窗) -->
      <MyCard title="Minecraft 游戏目录 (.minecraft)" subtitle="启动器扫描版本、Mod 与配置的根目录">
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
            💡 支持任意盘符与目录。点击“浏览”可唤起 Windows 原生文件夹选择窗口，修改后主界面与版本管理页面将自动重新扫描。
          </p>
        </div>
      </MyCard>

      <!-- 内存分配滑块 (PCL 经典自研滑块布局) -->
      <MyCard title="游戏最大运行内存 (Java Heap)" subtitle="根据你机器的物理内存配置合理的上限">
        <div class="space-y-4">
          <div class="flex items-center justify-between text-sm">
            <span class="text-pcl-subtext">当前分配</span>
            <span class="font-bold text-base text-pcl-primary">
              {{ store.maxMemoryMb }} MB ({{ (store.maxMemoryMb / 1024).toFixed(1) }} GB)
            </span>
          </div>

          <!-- 自定义滑动条 -->
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

      <!-- 窗口分辨率设置 -->
      <MyCard title="游戏默认窗口分辨率" subtitle="拉起游戏时的初始窗口长宽尺寸">
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

      <!-- 本地可用 Java 检测状态 -->
      <MyCard title="已检测到的系统 Java" subtitle="Rust 扫描到的本地运行库">
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
</template>
