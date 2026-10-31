<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useAppStore } from '@/stores/app'
import MyCard from '@/components/base/MyCard.vue'
import MyButton from '@/components/base/MyButton.vue'
import MyHint from '@/components/base/MyHint.vue'
import MyListItem from '@/components/base/MyListItem.vue'
import { backend } from '@/services/backend'
import type { LanRoomInfo } from '@/services/backend'

const store = useAppStore()
const rooms = ref<LanRoomInfo[]>([])
const isScanning = ref(false)

// 创建/广播房间表单
const hostMotd = ref('PBL 好友联机世界')
const hostIp = ref('127.0.0.1')
const hostPort = ref(25565)
const isBroadcasting = ref(false)
const createdRoom = ref<LanRoomInfo | null>(null)

// 邀请码解析表单
const inputInviteCode = ref('')
const decodedResult = ref<{ ip: string; port: number; address: string } | null>(null)
const decodeError = ref('')
const copyFeedback = ref('')

const scanRooms = async () => {
  isScanning.value = true
  try {
    rooms.value = await backend.scanLanRooms()
  } finally {
    isScanning.value = false
  }
}

const handleBroadcast = async () => {
  if (!hostPort.value || hostPort.value < 1 || hostPort.value > 65535) {
    alert('请输入有效的端口号 (1 ~ 65535)')
    return
  }
  isBroadcasting.value = true
  try {
    const info = await backend.broadcastLanRoom({
      motd: hostMotd.value,
      hostIp: hostIp.value,
      port: hostPort.value,
    })
    createdRoom.value = info
    // 将创建的房间同步加入上方列表展示
    const exists = rooms.value.some((r) => r.server_address === info.server_address)
    if (!exists) {
      rooms.value.unshift(info)
    }
  } catch (err: any) {
    alert(`广播房间失败: ${err}`)
  } finally {
    isBroadcasting.value = false
  }
}

const handleDecodeInvite = async () => {
  decodeError.value = ''
  decodedResult.value = null
  if (!inputInviteCode.value.trim()) {
    decodeError.value = '请输入好友分享的 PBL 联机邀请码'
    return
  }
  try {
    const [ip, port] = await backend.decodeLanInviteCode(inputInviteCode.value.trim())
    decodedResult.value = {
      ip,
      port,
      address: `${ip}:${port}`,
    }
  } catch (err: any) {
    decodeError.value = String(err)
  }
}

const copyText = async (text: string, label: string) => {
  try {
    await navigator.clipboard.writeText(text)
    copyFeedback.value = `已复制 ${label}: ${text}`
    setTimeout(() => {
      copyFeedback.value = ''
    }, 2500)
  } catch {
    alert(`${label}: ${text}`)
  }
}

onMounted(() => {
  scanRooms()
})
</script>

<template>
  <div class="h-full flex overflow-hidden" :class="store.uiStyle === 'classic' ? 'flex-row gap-0' : 'flex-col gap-4'">
    <!-- 原版 PCL2 经典左侧 152px 菜单栏 (PageLinkLeft.xaml) -->
    <div
      v-if="store.uiStyle === 'classic'"
      class="w-[152px] h-full bg-white dark:bg-[#1e232b] border-r border-black/8 dark:border-white/10 pt-3 flex flex-col shrink-0 select-none"
    >
      <MyListItem
        title="联机大厅"
        icon="link"
        :checked="true"
        action-title="重新扫描房间"
        @action="scanRooms"
      />
    </div>

    <div
      class="flex-1 h-full flex flex-col gap-4 overflow-y-auto"
      :class="store.uiStyle === 'classic' ? 'p-5' : 'pr-1'"
    >
    <!-- 顶部标题栏 -->
    <div class="flex items-center justify-between gap-4 shrink-0">
      <div class="flex items-center gap-3">
        <div class="w-10 h-10 rounded-xl bg-pcl-primary/20 flex items-center justify-center text-lg">
          🌐
        </div>
        <div>
          <h2 class="text-base font-bold text-pcl-text flex items-center gap-2">
            <span>局域网联机大厅</span>
            <span
              v-if="copyFeedback"
              class="text-xs bg-emerald-500/15 text-emerald-600 px-2 py-0.5 rounded font-normal"
            >
              ✔ {{ copyFeedback }}
            </span>
          </h2>
          <p class="text-xs text-pcl-subtext mt-0.5">
            探测局域网/虚拟局域网内的 Minecraft 共享世界（UDP 224.0.2.60:4445），支持邀请码一键互连
          </p>
        </div>
      </div>

      <MyButton type="primary" size="small" :loading="isScanning" @click="scanRooms">
        📡 扫描局域网房间
      </MyButton>
    </div>

    <MyHint type="info">
      💡 联机提示：房主在单人世界中按 ESC 点击「对局域网开放」后，本大厅可自动捕获广播或生成
      <b>PBL-LINK</b> 邀请码；配合虚拟局域网或内网穿透工具即可实现跨网联机。
    </MyHint>

    <div class="grid grid-cols-2 gap-4">
      <!-- 左侧：发布房间与生成邀请码 -->
      <MyCard title="创建联机广播 & 生成邀请码" subtitle="将你在游戏内开放的端口生成邀请码分享给好友">
        <div class="space-y-3 text-xs">
          <div>
            <label class="text-[11px] font-semibold text-pcl-subtext block mb-1">房间名称 (MOTD)</label>
            <input
              v-model="hostMotd"
              placeholder="例如：Steve 的养老生存服"
              class="w-full px-3 py-2 rounded-xl bg-black/5 dark:bg-white/5 border border-pcl-border text-xs text-pcl-text outline-none focus:border-pcl-primary"
            />
          </div>

          <div class="grid grid-cols-2 gap-2">
            <div>
              <label class="text-[11px] font-semibold text-pcl-subtext block mb-1">主机 IP / 穿透地址</label>
              <input
                v-model="hostIp"
                placeholder="如 192.168.1.10"
                class="w-full px-3 py-2 rounded-xl bg-black/5 dark:bg-white/5 border border-pcl-border text-xs text-pcl-text font-mono outline-none focus:border-pcl-primary"
              />
            </div>
            <div>
              <label class="text-[11px] font-semibold text-pcl-subtext block mb-1">局域网端口 (Port)</label>
              <input
                v-model.number="hostPort"
                type="number"
                placeholder="25565"
                class="w-full px-3 py-2 rounded-xl bg-black/5 dark:bg-white/5 border border-pcl-border text-xs text-pcl-text font-mono outline-none focus:border-pcl-primary"
              />
            </div>
          </div>

          <MyButton
            type="primary"
            size="small"
            class="w-full"
            :loading="isBroadcasting"
            @click="handleBroadcast"
          >
            📣 发送局域网广播并生成邀请码
          </MyButton>

          <div
            v-if="createdRoom"
            class="p-3 rounded-xl bg-pcl-primary/10 border border-pcl-primary/30 space-y-1.5"
          >
            <div class="flex items-center justify-between">
              <span class="font-bold text-pcl-primary">专属联机邀请码</span>
              <button
                class="text-[11px] text-pcl-primary underline cursor-pointer"
                @click="copyText(createdRoom.invite_code, '邀请码')"
              >
                复制邀请码
              </button>
            </div>
            <div class="font-mono font-bold text-sm text-pcl-text select-all">
              {{ createdRoom.invite_code }}
            </div>
            <div class="text-[11px] text-pcl-subtext">
              直连地址：<span class="font-mono">{{ createdRoom.server_address }}</span>
            </div>
          </div>
        </div>
      </MyCard>

      <!-- 右侧：输入邀请码解析直连地址 -->
      <MyCard title="凭邀请码加入好友房间" subtitle="解析好友发来的 PBL-LINK 邀请码并获取直连地址">
        <div class="space-y-3 text-xs">
          <div>
            <label class="text-[11px] font-semibold text-pcl-subtext block mb-1">输入邀请码</label>
            <input
              v-model="inputInviteCode"
              placeholder="例如：PBL-LINK-C0A8010A-63DD"
              class="w-full px-3 py-2 rounded-xl bg-black/5 dark:bg-white/5 border border-pcl-border text-xs text-pcl-text font-mono outline-none focus:border-pcl-primary"
              @keyup.enter="handleDecodeInvite"
            />
          </div>

          <MyButton type="secondary" size="small" class="w-full" @click="handleDecodeInvite">
            🔍 解析邀请码
          </MyButton>

          <div v-if="decodeError" class="text-[11px] text-red-500 font-medium">
            ❌ {{ decodeError }}
          </div>

          <div
            v-if="decodedResult"
            class="p-3 rounded-xl bg-emerald-500/10 border border-emerald-500/30 space-y-2"
          >
            <div class="flex items-center justify-between">
              <span class="font-bold text-emerald-600 dark:text-emerald-400">✔ 解析成功！直连服务器地址：</span>
              <MyButton
                type="primary"
                size="small"
                @click="copyText(decodedResult.address, '服务器直连地址')"
              >
                📋 复制地址
              </MyButton>
            </div>
            <div class="font-mono font-bold text-sm text-pcl-text select-all">
              {{ decodedResult.address }}
            </div>
            <div class="text-[11px] text-pcl-subtext">
              打开游戏 → 多人游戏 → 直接连接 → 粘贴上方地址即可加入世界。
            </div>
          </div>
        </div>
      </MyCard>
    </div>

    <!-- 已发现的局域网房间列表 -->
    <MyCard
      :title="`已发现的局域网世界 (${rooms.length})`"
      subtitle="自动监听 UDP 224.0.2.60:4445 广播报文"
    >
      <div v-if="rooms.length === 0" class="py-8 text-center text-xs text-pcl-subtext">
        {{
          isScanning
            ? '正在监听局域网广播信号...'
            : '暂未探测到局域网内开放的 Minecraft 房间。你可以在左侧手动创建广播或点击右上角重新扫描。'
        }}
      </div>
      <div v-else class="space-y-2">
        <div
          v-for="room in rooms"
          :key="room.server_address"
          class="flex items-center justify-between p-3 rounded-xl border border-pcl-border bg-black/5 dark:bg-white/5 hover:border-pcl-primary/40 transition-colors"
        >
          <div>
            <div class="font-bold text-sm text-pcl-text flex items-center gap-2">
              <span>🌍 {{ room.motd }}</span>
              <span class="text-[10px] font-mono bg-pcl-primary/15 text-pcl-primary px-2 py-0.5 rounded">
                {{ room.server_address }}
              </span>
            </div>
            <div class="text-[11px] text-pcl-subtext mt-1 font-mono">
              邀请码：{{ room.invite_code }}
            </div>
          </div>

          <div class="flex items-center gap-2">
            <MyButton
              type="secondary"
              size="small"
              @click="copyText(room.invite_code, '邀请码')"
            >
              复制邀请码
            </MyButton>
            <MyButton
              type="primary"
              size="small"
              @click="copyText(room.server_address, '直连地址')"
            >
              复制直连地址
            </MyButton>
          </div>
        </div>
      </div>
    </MyCard>
    </div>
  </div>
</template>
