<script setup lang="ts">
import { useVersionsStore } from '@/stores/versions'
import PclIcon from '@/components/base/PclIcon.vue'
import type { HorizonInstance, VersionInfo } from '@/services/backend'

const props = withDefaults(
  defineProps<{
    layout?: 'classic' | 'modern'
  }>(),
  {
    layout: 'classic',
  }
)

const emit = defineEmits<{
  (e: 'select'): void
}>()

const versionsStore = useVersionsStore()

const handleSelectJava = (v: VersionInfo) => {
  versionsStore.setCurrentVersion(v)
  emit('select')
}

const handleSelectHorizon = (inst: HorizonInstance) => {
  versionsStore.setCurrentHorizonInstance(inst)
  emit('select')
}
</script>

<template>
  <div
    v-if="layout === 'classic'"
    class="max-h-44 overflow-y-auto rounded-[4px] border border-[var(--pcl-brush-5)] bg-white dark:bg-[#222730] shadow-md p-1 space-y-0.5"
  >
    <div
      v-for="inst in versionsStore.horizonInstances"
      :key="`horizon:${inst.name}`"
      class="px-2.5 py-1.5 rounded-[3px] text-xs flex items-center justify-between cursor-pointer hover:bg-[var(--pcl-brush-7)]"
      :class="versionsStore.currentHorizonInstance?.name === inst.name && 'bg-[var(--pcl-brush-6)] font-bold text-[var(--pcl-brush-2)]'"
      @click="handleSelectHorizon(inst)"
    >
      <div class="flex items-center gap-1.5 truncate">
        <span class="px-1 py-0.2 rounded text-[9px] font-semibold bg-pcl-primary/15 text-pcl-primary shrink-0">
          Horizon
        </span>
        <span class="truncate">{{ inst.name }}</span>
      </div>
      <span class="text-[10px] text-[#737373] ml-2 shrink-0 font-mono">{{ inst.build_tag }}</span>
    </div>

    <div
      v-for="v in versionsStore.installedVersions"
      :key="`java:${v.id}`"
      class="px-2.5 py-1.5 rounded-[3px] text-xs flex items-center justify-between cursor-pointer hover:bg-[var(--pcl-brush-7)]"
      :class="versionsStore.currentVersion?.id === v.id && 'bg-[var(--pcl-brush-6)] font-bold text-[var(--pcl-brush-2)]'"
      @click="handleSelectJava(v)"
    >
      <div class="flex items-center gap-1.5 truncate">
        <span class="px-1 py-0.2 rounded text-[9px] font-medium bg-black/8 dark:bg-white/10 text-[#737373] shrink-0">
          Java
        </span>
        <span class="truncate">{{ v.name }}</span>
      </div>
      <span class="text-[10px] text-[#737373] ml-2 shrink-0">{{ v.loader || '原版' }}</span>
    </div>
  </div>

  <div
    v-else
    class="mt-2.5 space-y-1 border-t border-pcl-border pt-2 max-h-44 overflow-y-auto"
  >
    <div
      v-for="inst in versionsStore.horizonInstances"
      :key="`horizon:${inst.name}`"
      class="flex items-center justify-between px-2.5 py-1.5 rounded-md cursor-pointer transition-colors text-xs"
      :class="
        versionsStore.currentHorizonInstance?.name === inst.name
          ? 'bg-pcl-primary/10 border border-pcl-primary/30 text-pcl-primary font-medium'
          : 'hover:bg-black/5 dark:hover:bg-white/5 text-pcl-text'
      "
      @click="handleSelectHorizon(inst)"
    >
      <div class="flex items-center gap-2 truncate">
        <PclIcon name="launch" :size="13" class="text-pcl-primary shrink-0" />
        <span class="px-1.5 py-0.2 rounded text-[10px] font-semibold bg-pcl-primary/15 text-pcl-primary shrink-0">
          Horizon
        </span>
        <span class="truncate">{{ inst.name }}</span>
      </div>
      <span class="text-pcl-subtext shrink-0 ml-2 font-mono text-[11px]">
        {{ versionsStore.currentHorizonInstance?.name === inst.name ? '当前选中' : inst.build_tag }}
      </span>
    </div>

    <div
      v-for="v in versionsStore.installedVersions"
      :key="`java:${v.id}`"
      class="flex items-center justify-between px-2.5 py-1.5 rounded-md cursor-pointer transition-colors text-xs"
      :class="
        versionsStore.currentVersion?.id === v.id
          ? 'bg-pcl-primary/10 border border-pcl-primary/30 text-pcl-primary font-medium'
          : 'hover:bg-black/5 dark:hover:bg-white/5 text-pcl-text'
      "
      @click="handleSelectJava(v)"
    >
      <div class="flex items-center gap-2 truncate">
        <PclIcon :name="v.loader ? 'mod' : 'instance'" :size="13" class="text-pcl-subtext shrink-0" />
        <span class="px-1.5 py-0.2 rounded text-[10px] font-medium bg-black/5 dark:bg-white/10 text-pcl-subtext shrink-0">
          Java
        </span>
        <span class="truncate">{{ v.name }}</span>
      </div>
      <span class="text-pcl-subtext shrink-0 ml-2 font-mono text-[11px]">
        {{ versionsStore.currentVersion?.id === v.id ? '当前选中' : v.loader || 'Vanilla' }}
      </span>
    </div>
  </div>
</template>
