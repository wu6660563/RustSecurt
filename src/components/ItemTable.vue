<script setup lang="ts">
import { ref } from 'vue'
import { deleteHistory, restoreItems, type BatchResult, type DeleteHistoryResult, type HiddenItem } from '../api/filehide'

const props = defineProps<{
  items: HiddenItem[]
  showRestore: boolean
}>()

const selected = ref<number[]>([])
const hasRestorableSelection = () => selected.value.some(id => props.items.some(item => item.id === id && item.current_status === 1))
const hasDeletableSelection = () => selected.value.some(id => props.items.some(item => item.id === id && item.current_status === 0))
const emit = defineEmits<{ restore: [id: number]; batchRestore: [ids: number[]] }>()
const runBatchRestore = async () => {
  const ids = [...selected.value]
  if (!ids.length) return
  try {
    const result = await restoreItems(ids)
    window.dispatchEvent(new CustomEvent<BatchResult>('filehide-batch-restored', { detail: result }))
  } finally {
    selected.value = []
  }
}
const runDeleteHistory = async () => {
  const ids = [...selected.value]
  if (!ids.length || !window.confirm('删除历史记录后将无法从列表恢复，是否继续？')) return
  try {
    const result = await deleteHistory(ids, true)
    window.dispatchEvent(new CustomEvent<DeleteHistoryResult>('filehide-history-deleted', { detail: result }))
  } finally { selected.value = [] }
}
const itemName = (path: string) => path.split(/[/\\]/).filter(Boolean).pop() || path
const formatTime = (time: string) => new Date(time).toLocaleString()
const statusLabel: Record<HiddenItem['protection_status'], string> = {
  LOCKED: '已锁定',
  RESTORED: '已恢复',
  MISSING: '路径失效',
  UNLOCKED_EXTERNALLY: '已外部解除'
  ,PATH_CHANGED: '路径已变化', PATH_MOVED: '已移动', PATH_REPLACED: '已替换', PATH_DELETED: '已删除', TRANSACTION_PENDING: '处理中'
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
      <span><input v-if="showRestore && item.current_status === 1 && ['LOCKED', 'PATH_MOVED'].includes(item.protection_status) || item.current_status === 0 || !showRestore && ['PATH_DELETED', 'PATH_REPLACED'].includes(item.protection_status)" v-model="selected" type="checkbox" :value="item.id"><button v-if="showRestore && item.current_status === 1 && ['LOCKED', 'PATH_MOVED'].includes(item.protection_status)" class="restore" @click="$emit('restore', item.id)">恢复</button></span>
    </div>
    <button v-if="selected.length && hasRestorableSelection()" class="secondary" @click="runBatchRestore">批量恢复（{{ selected.length }}）</button>
    <button v-if="selected.length && hasDeletableSelection()" class="secondary" @click="runDeleteHistory">安全删除历史</button>
  </div>
</template>
