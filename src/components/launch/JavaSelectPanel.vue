<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import MyCard from '@/components/base/MyCard.vue'
import MyButton from '@/components/base/MyButton.vue'
import { backend } from '@/services/backend'
import type { JavaInfo } from '@/services/backend'
import { useVersionsStore } from '@/stores/versions'

const props = withDefaults(
  defineProps<{
    modelValue: JavaInfo | null
    layout?: 'classic' | 'modern'
  }>(),
  {
    layout: 'classic',
  }
)

const emit = defineEmits<{
  (e: 'update:modelValue', val: JavaInfo | null): void
}>()

const versionsStore = useVersionsStore()
const javaList = ref<JavaInfo[]>([])
const isScanning = ref(false)

const scanJavaRuntime = async () => {
  isScanning.value = true
  try {
    const javas = await backend.getInstalledJava()
    javaList.value = javas
    const reqMajor = versionsStore.currentVersion?.java_version || 8
    const matched = javas.find((j) => j.major_version === reqMajor)
    if (matched) {
      emit('update:modelValue', matched)
    } else if (javas.length > 0 && !props.modelValue) {
      emit('update:modelValue', javas[0])
    }
  } finally {
    isScanning.value = false
  }
}

onMounted(() => {
  void scanJavaRuntime()
})

watch(
  () => versionsStore.currentVersion,
  (newVer) => {
    if (newVer && javaList.value.length > 0) {
      const reqMajor = newVer.java_version || 8
      const matched = javaList.value.find((j) => j.major_version === reqMajor)
      if (matched) {
        emit('update:modelValue', matched)
      }
    }
  }
)

const handleSelect = (j: JavaInfo) => {
  emit('update:modelValue', j)
}

defineExpose({
  scanJavaRuntime,
  javaList,
  isScanning,
})
</script>

<template>
  <MyCard
    v-if="layout === 'classic'"
    title="Java 运行环境选择"
    subtitle="点击切换用于启动当前版本的 Java 虚拟机"
  >
    <template #extra>
      <MyButton type="text" size="small" :loading="isScanning" @click="scanJavaRuntime">
        刷新
      </MyButton>
    </template>
    <div class="space-y-1.5">
      <div
        v-for="(j, idx) in javaList"
        :key="j.path"
        class="flex items-center justify-between px-3 py-2 rounded-[4px] border text-xs cursor-pointer transition-colors"
        :class="
          modelValue?.path === j.path
            ? 'border-[var(--pcl-brush-3)] bg-[var(--pcl-brush-7)] text-[var(--pcl-brush-2)]'
            : 'border-black/8 bg-white/60 hover:bg-[var(--pcl-brush-8)]'
        "
        @click="handleSelect(j)"
      >
        <div class="truncate pr-2">
          <span class="font-semibold">Java {{ j.version }}</span>
          <span class="ml-2 text-[11px] text-[#737373]">({{ j.arch || '64-Bit' }})</span>
          <span v-if="idx === 0" class="ml-1.5 text-[10px] text-[var(--pcl-brush-3)] font-bold">[推荐]</span>
          <div class="text-[11px] text-[#8c8c8c] truncate font-mono">{{ j.path }}</div>
        </div>
        <span v-if="modelValue?.path === j.path" class="font-bold text-[var(--pcl-brush-3)] shrink-0">
          已选中
        </span>
      </div>
    </div>
  </MyCard>

  <MyCard
    v-else
    title="Java 运行时环境"
    subtitle="点击展开切换用于启动该版本的 Java 虚拟机"
    :default-collapsed="true"
  >
    <template #extra>
      <span
        v-if="modelValue"
        class="px-2 py-0.5 rounded-md bg-pcl-primary/15 border border-pcl-primary/30 text-pcl-primary text-[11px] font-semibold"
      >
        ☕ Java {{ modelValue.version }} ({{ modelValue.arch || '64-Bit' }})
      </span>
    </template>

    <div v-if="javaList.length === 0" class="text-xs text-pcl-subtext p-2">
      正在搜寻 Java 运行时...
    </div>
    <div v-else class="space-y-2">
      <div
        v-for="(j, idx) in javaList"
        :key="j.path"
        class="flex items-center justify-between px-3 py-2 rounded-xl border border-pcl-border bg-black/5 dark:bg-black/25 text-xs cursor-pointer transition-all"
        :class="modelValue?.path === j.path && 'border-pcl-primary bg-pcl-primary/10 ring-1 ring-pcl-primary/30'"
        @click="handleSelect(j)"
      >
        <div class="flex items-center gap-2.5 overflow-hidden">
          <span class="text-base">{{ j.is_jdk ? '☕' : '📦' }}</span>
          <div class="truncate">
            <div class="font-semibold text-pcl-text flex items-center gap-1.5">
              <span>Java {{ j.version }}</span>
              <span v-if="idx === 0" class="text-[10px] bg-pcl-primary/20 text-pcl-primary px-1.5 py-0.2 rounded font-bold">推荐</span>
              <span class="text-[10px] bg-black/5 dark:bg-white/10 px-1.5 py-0.2 rounded text-pcl-subtext">
                {{ j.arch || (j.is_64_bit ? '64-Bit' : '32-Bit') }}
              </span>
            </div>
            <div class="text-[11px] text-pcl-subtext truncate mt-0.5 font-mono" :title="j.path">{{ j.path }}</div>
          </div>
        </div>
        <span v-if="modelValue?.path === j.path" class="text-pcl-primary font-bold ml-2 shrink-0">✔ 已选</span>
        <span v-else class="text-emerald-500 font-bold ml-2 shrink-0">可用</span>
      </div>
    </div>
  </MyCard>
</template>
