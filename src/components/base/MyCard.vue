<script setup lang="ts">
import { ref } from 'vue'
import { useAppStore } from '@/stores/app'

const props = withDefaults(
  defineProps<{
    title?: string
    subtitle?: string
    icon?: string
    canCollapse?: boolean
    defaultCollapsed?: boolean
  }>(),
  {
    canCollapse: true,
    defaultCollapsed: false,
  }
)

const store = useAppStore()
const isCollapsed = ref(props.defaultCollapsed)

const toggleCollapse = () => {
  if (props.canCollapse) {
    isCollapsed.value = !isCollapsed.value
  }
}
</script>

<template>
  <div
    :class="[
      store.uiStyle === 'classic'
        ? 'bg-white/95 dark:bg-[#222730]/95 border border-black/8 dark:border-white/10 rounded-[5px] px-4 py-3 shadow-[0_1.5px_5px_rgba(0,0,0,0.07)] transition-all duration-200 hover:shadow-[0_2px_8px_rgba(19,112,243,0.12)]'
        : 'bg-pcl-card border border-pcl-border rounded-2xl p-4 shadow-pcl-card transition-all duration-300 ease-[var(--pcl-ease-smooth)] hover:shadow-pcl-hover'
    ]"
  >
    <div
      v-if="title || subtitle"
      class="flex items-center justify-between"
      :class="[
        !isCollapsed && 'mb-3',
        canCollapse && 'cursor-pointer select-none'
      ]"
      @click="toggleCollapse"
    >
      <div>
        <h3
          class="font-bold flex items-center gap-2"
          :class="
            store.uiStyle === 'classic'
              ? 'text-[13.5px] text-[var(--pcl-brush-1)] dark:text-gray-100'
              : 'text-[15px] font-semibold text-pcl-text'
          "
        >
          <slot name="icon" />
          {{ title }}
        </h3>
        <p v-if="subtitle && !isCollapsed" class="text-xs text-pcl-subtext mt-0.5">
          {{ subtitle }}
        </p>
      </div>
      <div class="flex items-center gap-2" @click.stop>
        <slot name="extra" />
        <button
          v-if="canCollapse"
          class="w-5 h-5 border-none bg-transparent outline-none flex items-center justify-center text-[11px] text-pcl-subtext hover:text-pcl-primary transition-transform duration-200 cursor-pointer"
          :class="isCollapsed ? '-rotate-90' : 'rotate-0'"
          :title="isCollapsed ? '点击展开卡片' : '点击折叠卡片'"
          @click="toggleCollapse"
        >
          ▼
        </button>
      </div>
    </div>
    <div v-show="!isCollapsed">
      <slot />
    </div>
  </div>
</template>
