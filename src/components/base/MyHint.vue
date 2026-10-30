<script setup lang="ts">
withDefaults(
  defineProps<{
    type?: 'info' | 'warning' | 'error' | 'success'
  }>(),
  {
    type: 'info',
  }
)
</script>

<template>
  <div
    class="flex items-start gap-3 p-3.5 rounded-xl text-xs leading-relaxed"
    :class="[
      type === 'info' && 'bg-blue-500/10 text-blue-600 dark:text-blue-400 border border-blue-500/20',
      type === 'warning' && 'bg-amber-500/10 text-amber-600 dark:text-amber-400 border border-amber-500/20',
      type === 'error' && 'bg-red-500/10 text-red-600 dark:text-red-400 border border-red-500/20',
      type === 'success' && 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 border border-emerald-500/20',
    ]"
  >
    <div class="font-bold text-sm select-none">
      <span v-if="type === 'info'">ℹ️</span>
      <span v-else-if="type === 'warning'">⚠️</span>
      <span v-else-if="type === 'error'">❌</span>
      <span v-else-if="type === 'success'">✅</span>
    </div>
    <div class="flex-1">
      <slot />
    </div>
  </div>
</template>
