<script setup lang="ts">
import PclIcon from './PclIcon.vue'

defineProps<{
  title: string
  icon?:
    | 'launch'
    | 'download'
    | 'link'
    | 'setup'
    | 'more'
    | 'instance'
    | 'vanilla'
    | 'mod'
    | 'modpack'
    | 'datapack'
    | 'resourcepack'
    | 'shader'
    | 'gear'
  checked?: boolean
  actionTitle?: string
}>()

const emit = defineEmits<{
  (e: 'click'): void
  (e: 'action'): void
}>()
</script>

<template>
  <div
    class="group relative h-9 flex items-center px-3 cursor-pointer select-none transition-all duration-150"
    :class="[
      checked
        ? 'text-[var(--pcl-brush-3)] font-semibold bg-[var(--pcl-brush-7)]'
        : 'text-[var(--pcl-brush-1)] hover:bg-[var(--pcl-brush-7)]'
    ]"
    @click="emit('click')"
  >
    <!-- 原版 MyListItem 左侧选中竖条 (ColumnCheck) -->
    <div
      class="absolute left-0 top-1.5 bottom-1.5 w-[3px] rounded-r transition-all duration-200"
      :class="checked ? 'bg-[var(--pcl-brush-3)] opacity-100' : 'opacity-0'"
    />

    <!-- 左侧矢量图标 -->
    <PclIcon v-if="icon" :name="icon" :size="15" class="mr-2.5 ml-1" />

    <!-- 标题文本 -->
    <span class="flex-1 text-[13px] truncate">{{ title }}</span>

    <!-- 右侧悬停显示的快捷图标按钮 (如刷新) -->
    <button
      v-if="actionTitle"
      :title="actionTitle"
      class="opacity-0 group-hover:opacity-100 w-6 h-6 rounded flex items-center justify-center text-[var(--pcl-brush-1)] hover:text-[var(--pcl-brush-3)] hover:bg-black/5 transition-all cursor-pointer"
      @click.stop="emit('action')"
    >
      <PclIcon name="refresh" :size="13" />
    </button>
  </div>
</template>
