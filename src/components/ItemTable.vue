<script setup lang="ts">
import type { HiddenItem } from '../api/filehide'

defineProps<{
  items: HiddenItem[]
  showRestore: boolean
}>()

defineEmits<{
  restore: [id: number]
}>()

const itemName = (path: string) => path.split(/[/\\]/).filter(Boolean).pop() || path
const formatTime = (time: string) => new Date(time).toLocaleString()
const statusLabel: Record<HiddenItem['protection_status'], string> = {
  LOCKED: '已锁定',
  RESTORED: '已恢复',
  MISSING: '路径失效',
  UNLOCKED_EXTERNALLY: '已外部解除'
}

const statusClass = (status: HiddenItem['protection_status']) => `status-${status.toLowerCase().replace('_', '-')}`
</script>

<template>
  <div v-if="!items.length" class="empty">没有可显示的项目</div>
  <div v-else class="table">
    <div class="row heading">
      <span>项目</span><span>类型</span><span>状态</span><span>时间</span><span></span>
    </div>
    <div v-for="item in items" :key="item.id" class="row">
      <span class="item">
        <i>{{ item.item_type === 'FOLDER' ? '□' : '◇' }}</i>
        <b>{{ itemName(item.path) }}</b>
        <small>{{ item.path }}</small>
      </span>
      <span><label>{{ item.item_type === 'FOLDER' ? '文件夹' : '文件' }}</label></span>
      <span><label class="status" :class="statusClass(item.protection_status)">{{ statusLabel[item.protection_status] }}</label></span>
      <span class="date">{{ formatTime(item.update_time) }}</span>
      <span><button v-if="showRestore && item.current_status === 1 && item.protection_status === 'LOCKED'" class="restore" @click="$emit('restore', item.id)">恢复</button></span>
    </div>
  </div>
</template>
