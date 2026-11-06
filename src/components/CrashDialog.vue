<script setup lang="ts">
import { ref } from 'vue'
import PclIcon from '@/components/base/PclIcon.vue'
import type { CrashAnalysisResult } from '@/services/backend'
import { backend } from '@/services/backend'

const props = defineProps<{
  crash: CrashAnalysisResult
}>()

const emit = defineEmits<{
  (e: 'close'): void
}>()

const showLogSnippet = ref(false)
const copied = ref(false)

const copyReport = async () => {
  const text = `【Minecraft 崩溃诊断报告】
诊断结论: ${props.crash.title}
退出代码: ${props.crash.exit_code ?? '未知'}
原因分析: ${props.crash.reason}
解决建议: ${props.crash.suggestion}

--- 日志截取片段 ---
${props.crash.log_snippet}
`
  try {
    await navigator.clipboard.writeText(text)
    copied.value = true
    setTimeout(() => {
      copied.value = false
    }, 2000)
  } catch (err) {
    console.error('复制失败:', err)
  }
}

const openReportFile = async () => {
  if (props.crash.report_file) {
    await backend.openFolder(props.crash.report_file)
  }
}
</script>

<template>
  <div class="fixed inset-0 z-50 flex items-center justify-center p-6 bg-black/60 backdrop-blur-md animate-fade-in select-text">
    <div
      class="w-full max-w-xl bg-pcl-card border border-red-500/40 rounded-3xl shadow-2xl p-6 flex flex-col gap-4 max-h-[85vh] overflow-hidden"
    >
      <div class="flex items-center justify-between shrink-0 pb-3 border-b border-pcl-border/60">
        <div class="flex items-center gap-3">
          <div class="w-9 h-9 rounded-lg bg-red-500/10 text-red-500 flex items-center justify-center shrink-0">
            <PclIcon name="warning" :size="18" />
          </div>
          <div>
            <h3 class="text-base font-bold text-pcl-text flex items-center gap-2">
              <span>游戏崩溃诊断报告</span>
              <span class="text-xs font-mono font-normal px-2 py-0.5 rounded bg-red-500/10 text-red-500">
                代码: {{ crash.exit_code ?? '异常' }}
              </span>
            </h3>
            <p class="text-xs text-pcl-subtext mt-0.5">
              启动器已自动捕获日志并完成智能诊断分析
            </p>
          </div>
        </div>

        <button
          class="w-8 h-8 rounded-xl flex items-center justify-center text-pcl-subtext hover:text-pcl-text hover:bg-black/5 dark:hover:bg-white/10 transition-colors cursor-pointer"
          @click="emit('close')"
        >
          ✕
        </button>
      </div>

      <div class="flex-1 overflow-y-auto pr-1 space-y-3">
        <div class="p-3.5 rounded-lg bg-red-500/10 border border-red-500/20 flex flex-col gap-1.5">
          <div class="text-xs font-semibold text-red-500 flex items-center gap-1.5">
            <PclIcon name="warning" :size="13" />
            <span>诊断结论：{{ crash.title }}</span>
          </div>
          <div class="text-xs text-pcl-text leading-relaxed">
            {{ crash.reason }}
          </div>
        </div>

        <div class="p-3.5 rounded-lg bg-emerald-500/10 border border-emerald-500/20 flex flex-col gap-1.5">
          <div class="text-xs font-semibold text-emerald-600 dark:text-emerald-400 flex items-center gap-1.5">
            <PclIcon name="bulb" :size="13" />
            <span>排错与解决方案</span>
          </div>
          <div class="text-xs text-pcl-text leading-relaxed">
            {{ crash.suggestion }}
          </div>
        </div>

        <div class="flex flex-col gap-2">
          <div class="flex items-center justify-between">
            <button
              class="text-xs text-pcl-primary hover:underline cursor-pointer flex items-center gap-1 font-medium select-none"
              @click="showLogSnippet = !showLogSnippet"
            >
              <span>{{ showLogSnippet ? '收起' : '展开' }} 崩溃日志截取 (末尾 35 行)</span>
            </button>
            <span v-if="crash.report_file" class="text-[11px] text-pcl-subtext truncate max-w-xs font-mono">
              报告: {{ crash.report_file }}
            </span>
          </div>

          <div
            v-if="showLogSnippet"
            class="p-3 rounded-lg bg-black/85 text-emerald-400 font-mono text-[11px] leading-relaxed max-h-56 overflow-y-auto whitespace-pre-wrap select-all border border-pcl-border/40"
          >
            {{ crash.log_snippet }}
          </div>
        </div>
      </div>

      <div class="flex items-center justify-between gap-3 pt-3 border-t border-pcl-border shrink-0">
        <div class="flex items-center gap-2">
          <button
            v-if="crash.report_file"
            class="px-3 py-1.5 rounded-md text-xs font-medium bg-black/5 dark:bg-white/10 hover:bg-black/10 dark:hover:bg-white/15 text-pcl-text transition-colors cursor-pointer flex items-center gap-1.5"
            @click="openReportFile"
          >
            <PclIcon name="folder" :size="13" />
            <span>打开崩溃日志</span>
          </button>

          <button
            class="px-3 py-1.5 rounded-md text-xs font-medium bg-black/5 dark:bg-white/10 hover:bg-black/10 dark:hover:bg-white/15 text-pcl-text transition-colors cursor-pointer flex items-center gap-1.5"
            @click="copyReport"
          >
            <PclIcon :name="copied ? 'check' : 'copy'" :size="13" />
            <span>{{ copied ? '已复制到剪贴板' : '复制诊断报告' }}</span>
          </button>
        </div>

        <button
          class="px-4 py-1.5 rounded-md text-xs font-medium bg-pcl-primary hover:bg-pcl-hover text-white shadow-xs transition-colors cursor-pointer active:scale-98"
          @click="emit('close')"
        >
          确定
        </button>
      </div>
    </div>
  </div>
</template>