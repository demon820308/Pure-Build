<script setup lang="ts">
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
</script>

<template>
  <button
    :disabled="disabled || loading"
    class="relative inline-flex items-center justify-center font-medium rounded-xl transition-all duration-200 cursor-pointer outline-none border-none select-none active:scale-95 disabled:opacity-50 disabled:cursor-not-allowed disabled:active:scale-100 ease-[var(--pcl-ease-bounce)]"
    :class="[
      size === 'small' && 'px-3 py-1.5 text-xs gap-1.5',
      size === 'medium' && 'px-4 py-2 text-sm gap-2',
      size === 'large' && 'px-6 py-3 text-base gap-2.5',

      type === 'primary' && 'bg-pcl-primary text-white hover:bg-pcl-hover active:bg-pcl-active shadow-sm',
      type === 'secondary' && 'bg-black/5 dark:bg-white/10 text-pcl-text hover:bg-black/10 dark:hover:bg-white/15',
      type === 'text' && 'bg-transparent text-pcl-text hover:bg-black/5 dark:hover:bg-white/5',
      type === 'danger' && 'bg-red-500 text-white hover:bg-red-600',
    ]"
    @click="$emit('click', $event)"
  >
    <span v-if="loading" class="animate-spin mr-1">⏳</span>
    <slot name="icon" />
    <slot />
  </button>
</template>
