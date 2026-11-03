<script setup lang="ts">
import type { RemoteVersion } from '@/services/backend'
import { backend } from '@/services/backend'

defineProps<{
  version: RemoteVersion
}>()

const emit = defineEmits<{
  (e: 'close'): void
}>()

const formatDate = (isoStr: string) => {
  try {
    const d = new Date(isoStr)
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
  } catch {
    return isoStr
  }
}

const getWikiUrl = (id: string, type: string) => {
  const cleanId = id.toLowerCase()
  if (cleanId === '3d shareware v1.34') return 'https://zh.minecraft.wiki/w/3D_Shareware_v1.34'
  if (cleanId === '2.0') return 'https://zh.minecraft.wiki/w/Java版2.0'
  if (type === 'release') return `https://zh.minecraft.wiki/w/Java版${id}`
  if (type === 'snapshot') return `https://zh.minecraft.wiki/w/${id}`
  return `https://zh.minecraft.wiki/w/Java版${id}`
}

const openWikiLink = async (v: RemoteVersion) => {
  const url = getWikiUrl(v.id, v.type)
  try {
    await backend.openUrl(url)
  } catch (err) {
    console.warn('调用系统浏览器失败，尝试前端打开:', err)
    window.open(url, '_blank')
  }
}
</script>

<template>
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4 animate-fade-in"
    @click.self="emit('close')"
  >
    <div class="max-w-md w-full bg-pcl-card border border-pcl-border rounded-2xl shadow-2xl p-6 flex flex-col gap-4">
      <div class="flex items-center justify-between pb-3 border-b border-pcl-border/50">
        <div class="flex items-center gap-3">
          <span class="text-2xl">ℹ️</span>
          <div>
            <h3 class="font-bold text-base text-pcl-text">Minecraft {{ version.id }}</h3>
            <p class="text-xs text-pcl-subtext">官方版本更新说明与元数据</p>
          </div>
        </div>
        <button
          class="p-2 rounded-xl text-pcl-subtext hover:text-pcl-text hover:bg-black/5 dark:hover:bg-white/10 transition-colors cursor-pointer text-sm"
          @click="emit('close')"
        >
          ✕
        </button>
      </div>

      <div class="flex flex-col gap-2.5 text-xs text-pcl-text">
        <div class="flex justify-between py-1.5 border-b border-pcl-border/30">
          <span class="text-pcl-subtext">版本编号</span>
          <span class="font-mono font-bold">{{ version.id }}</span>
        </div>
        <div class="flex justify-between py-1.5 border-b border-pcl-border/30">
          <span class="text-pcl-subtext">版本类型</span>
          <span>{{ version.type === 'release' ? '正式版 (Release)' : '快照/测试版 (Snapshot)' }}</span>
        </div>
        <div class="flex justify-between py-1.5 border-b border-pcl-border/30">
          <span class="text-pcl-subtext">发布日期</span>
          <span>{{ formatDate(version.releaseTime) }}</span>
        </div>
        <div class="flex justify-between py-1.5 border-b border-pcl-border/30">
          <span class="text-pcl-subtext">SHA-1 校验指纹</span>
          <span class="font-mono text-[11px] select-all truncate max-w-48">{{ version.sha1 }}</span>
        </div>
      </div>

      <div class="p-3 rounded-xl bg-black/5 dark:bg-white/5 border border-pcl-border/50 text-xs text-pcl-subtext leading-relaxed">
        💡 点击下方按钮可在浏览器中直达官方中文 Minecraft Wiki，查看该版本完整的新增方块、实体、物品合成及修复日志。
      </div>

      <div class="flex items-center justify-end gap-3 pt-2">
        <button
          class="px-4 py-2 rounded-xl text-xs font-medium text-pcl-subtext hover:text-pcl-text hover:bg-black/5 dark:hover:bg-white/10 cursor-pointer"
          @click="emit('close')"
        >
          关闭
        </button>
        <button
          class="px-4 py-2 rounded-xl text-xs font-bold text-white bg-pcl-primary hover:bg-pcl-hover cursor-pointer shadow-sm flex items-center gap-1.5"
          @click="openWikiLink(version)"
        >
          <span>📖 中文 Wiki 专页</span>
          <span>↗</span>
        </button>
      </div>
    </div>
  </div>
</template>
