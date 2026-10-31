<script setup lang="ts">
import { useAppStore } from '@/stores/app'

withDefaults(
  defineProps<{
    type?: 'primary' | 'secondary' | 'text' | 'danger'
    size?: 'small' | 'medium' | 'large'
    disabled?: boolean
    loading?: boolean
  }>(),
  {
    type: 'primary',
    size: 'medium',
    disabled: false,
    loading: false,
  }
)

defineEmits<{
  (e: 'click', ev: MouseEvent): void
}>()

const store = useAppStore()
</script>

<template>
  <button
    :disabled="disabled || loading"
    class="relative inline-flex items-center justify-center font-medium transition-all duration-150 cursor-pointer outline-none select-none active:scale-95 disabled:opacity-50 disabled:cursor-not-allowed disabled:active:scale-100 ease-[var(--pcl-ease-bounce)] whitespace-nowrap shrink-0"
    :class="[
      store.uiStyle === 'classic' ? 'rounded-[4px]' : 'rounded-xl border-none',

      size === 'small' && (store.uiStyle === 'classic' ? 'px-3 py-1 text-xs gap-1.5' : 'px-3 py-1.5 text-xs gap-1.5'),
      size === 'medium' && (store.uiStyle === 'classic' ? 'px-4 py-1.5 text-[13px] gap-2' : 'px-4 py-2 text-sm gap-2'),
      size === 'large' && 'px-6 py-3 text-base gap-2.5',

      store.uiStyle === 'classic'
        ? [
            type === 'primary' &&
              'border border-[var(--pcl-brush-3)] bg-[var(--pcl-brush-8)] text-[var(--pcl-brush-2)] hover:bg-[var(--pcl-brush-6)] active:bg-[var(--pcl-brush-5)] font-semibold',
            type === 'secondary' &&
              'border border-black/25 bg-white text-[var(--pcl-brush-1)] hover:border-[var(--pcl-brush-3)] hover:text-[var(--pcl-brush-3)] hover:bg-[var(--pcl-brush-7)]',
            type === 'text' &&
              'border border-transparent bg-transparent text-[var(--pcl-brush-1)] hover:text-[var(--pcl-brush-3)] hover:bg-[var(--pcl-brush-7)]',
            type === 'danger' &&
              'border border-[#ce2111] bg-[#80fbdddd] text-[#ce2111] hover:bg-[#ff4c4c] hover:text-white',
          ]
        : [
            type === 'primary' && 'bg-pcl-primary text-white hover:bg-pcl-hover active:bg-pcl-active shadow-sm',
            type === 'secondary' && 'bg-black/5 dark:bg-white/10 text-pcl-text hover:bg-black/10 dark:hover:bg-white/15',
            type === 'text' && 'bg-transparent text-pcl-text hover:bg-black/5 dark:hover:bg-white/5',
            type === 'danger' && 'bg-red-500 text-white hover:bg-red-600',
          ],
    ]"
    @click="$emit('click', $event)"
  >
    <span v-if="loading" class="animate-spin mr-1">⏳</span>
    <slot name="icon" />
    <slot />
  </button>
</template>
