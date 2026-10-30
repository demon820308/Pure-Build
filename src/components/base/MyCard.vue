<script setup lang="ts">
withDefaults(
  defineProps<{
    title?: string
    subtitle?: string
    icon?: string
    canCollapse?: boolean
  }>(),
  {
    canCollapse: false,
  }
)
</script>

<template>
  <div
    class="bg-pcl-card border border-pcl-border rounded-2xl p-5 shadow-pcl-card transition-all duration-300 ease-[var(--pcl-ease-smooth)] hover:shadow-pcl-hover hover:-translate-y-0.5"
  >
    <div v-if="title || subtitle" class="flex items-center justify-between mb-3">
      <div>
        <h3 class="text-base font-semibold text-pcl-text flex items-center gap-2">
          <slot name="icon" />
          {{ title }}
        </h3>
        <p v-if="subtitle" class="text-xs text-pcl-subtext mt-0.5">
          {{ subtitle }}
        </p>
      </div>
      <slot name="extra" />
    </div>
    <div>
      <slot />
    </div>
  </div>
</template>
