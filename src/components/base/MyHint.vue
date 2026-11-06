<script setup lang="ts">
import PclIcon from './PclIcon.vue'

withDefaults(
  defineProps<{
    type?: 'info' | 'warning' | 'error' | 'success'
    closable?: boolean
  }>(),
  {
    type: 'info',
    closable: true,
  }
)

const emit = defineEmits<{
  (e: 'close'): void
}>()
</script>

<template>
  <div
    class="flex items-start gap-3 p-3.5 rounded-xl text-xs leading-relaxed transition-all"
    :class="[
      type === 'info' && 'bg-blue-500/10 text-blue-600 dark:text-blue-400 border border-blue-500/20',
      type === 'warning' && 'bg-amber-500/10 text-amber-600 dark:text-amber-400 border border-amber-500/20',
      type === 'error' && 'bg-red-500/10 text-red-600 dark:text-red-400 border border-red-500/20',
      type === 'success' && 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 border border-emerald-500/20',
    ]"
  >
    <div class="shrink-0 mt-0.5 select-none">
      <PclIcon v-if="type === 'info'" name="info" :size="14" />
      <PclIcon v-else-if="type === 'warning'" name="warning" :size="14" />
      <PclIcon v-else-if="type === 'error'" name="close" :size="14" />
      <PclIcon v-else-if="type === 'success'" name="check" :size="14" />
    </div>
    <div class="flex-1 min-w-0">
      <slot />
    </div>
    <button
      v-if="closable"
      type="button"
      class="opacity-50 hover:opacity-100 transition-opacity p-0.5 -mr-1 -mt-0.5 cursor-pointer select-none"
      title="关闭提示"
      @click="emit('close')"
    >
      <PclIcon name="close" :size="10" />
    </button>
  </div>
</template>
