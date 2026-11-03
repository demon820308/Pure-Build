<script setup lang="ts">
export interface LoaderItem {
  version: string
  stable: boolean
  description?: string
}

const props = withDefaults(
  defineProps<{
    title: string
    isOpen: boolean
    selectedVersion?: string
    loading?: boolean
    supported?: boolean
    statusText: string
    selectLabel: string
    loadingText: string
    emptyText: string
    itemIcon?: string
    items: LoaderItem[]
  }>(),
  {
    selectedVersion: '',
    loading: false,
    supported: true,
    itemIcon: '📜',
  }
)

const emit = defineEmits<{
  (e: 'toggle'): void
  (e: 'select', version: string): void
  (e: 'clear'): void
}>()
</script>

<template>
  <div class="rounded-2xl border border-pcl-border bg-black/5 dark:bg-white/5 overflow-hidden transition-all shadow-sm">
    <div
      class="flex items-center justify-between p-3.5 cursor-pointer hover:bg-black/5 dark:hover:bg-white/5 select-none"
      :class="!supported ? 'opacity-70' : ''"
      @click="emit('toggle')"
    >
      <div class="flex items-center gap-2.5 font-bold text-sm text-pcl-text">
        <span>{{ title }}</span>
      </div>
      <div class="flex items-center gap-2 text-xs text-pcl-subtext">
        <span v-if="selectedVersion" class="text-pcl-primary font-bold">
          {{ selectedVersion }}
        </span>
        <span v-else-if="loading">查询中...</span>
        <span v-else>{{ statusText }}</span>
        <span class="text-sm transition-transform duration-200" :class="isOpen ? 'rotate-90' : ''">›</span>
      </div>
    </div>

    <div v-if="isOpen" class="p-3 border-t border-pcl-border/50 bg-black/5 dark:bg-white/5 flex flex-col gap-2 text-xs">
      <div class="flex items-center justify-between">
        <span class="text-pcl-subtext">{{ selectLabel }}</span>
        <button
          v-if="selectedVersion"
          class="text-rose-500 hover:underline cursor-pointer font-medium"
          @click="emit('clear')"
        >
          清除选择
        </button>
      </div>

      <div v-if="loading" class="py-3 text-pcl-subtext flex items-center justify-center gap-2">
        <span class="animate-spin">⏳</span> {{ loadingText }}
      </div>

      <div v-else-if="!supported || items.length === 0" class="py-3 text-pcl-subtext text-center">
        {{ emptyText }}
      </div>

      <div v-else class="max-h-60 overflow-y-auto flex flex-col gap-1 pr-1">
        <div
          v-for="item in items"
          :key="item.version"
          class="flex items-center gap-3 px-3.5 py-2.5 rounded-xl cursor-pointer transition-colors"
          :class="selectedVersion === item.version
            ? 'bg-pcl-primary/20 text-pcl-primary font-bold border border-pcl-primary/30'
            : 'hover:bg-black/5 dark:hover:bg-white/10 text-pcl-text'"
          @click="emit('select', item.version)"
        >
          <div class="text-lg shrink-0 opacity-80">{{ itemIcon }}</div>
          <div class="flex flex-col flex-1 leading-tight">
            <span class="text-xs font-bold font-mono">{{ item.version }}</span>
            <span class="text-[10px] text-pcl-subtext mt-0.5">
              {{ item.description || (item.stable ? '稳定版' : '测试版') }}
            </span>
          </div>
          <div v-if="selectedVersion === item.version" class="text-pcl-primary text-xs font-bold">
            ✓
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
