<script setup lang="ts">
defineProps<{
  percentage: number
  step: string
  speedText: string
}>()
</script>

<template>
  <div class="mt-3 pt-2 border-t border-pcl-border/40 flex flex-col gap-1.5">
    <div class="w-full h-1.5 rounded-full bg-black/5 dark:bg-white/10 overflow-hidden">
      <div
        class="h-full bg-pcl-primary transition-all duration-300 ease-out rounded-full"
        :style="{ width: `${percentage}%` }"
      />
    </div>
    <div class="text-[11px] text-pcl-primary flex items-center justify-between">
      <span>{{ step }}</span>
      <span>{{ speedText }}</span>
    </div>
  </div>
</template>
