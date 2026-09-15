<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import './list-tools.css'
import { open } from '@tauri-apps/plugin-dialog'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { cancelRecoveryScan, clearAccessPassword, checkSession, contextMenuEnabled, deleteHistory, getAutoLockMinutes, healthCheck, hideFiles, hideFolders, hidePaths, listItems, passwordConfigured, previewPaths, recoverMarkedItem, restoreItem, restoreItems, setAccessPassword, setAutoLockMinutes, setContextMenuEnabled, startRecoveryScan, storageInfo, verifyPassword, type BatchResult, type DeleteHistoryResult, type HiddenItem, type RecoveryCandidate, type RecoveryScanUpdate, type StorageInfo } from './api/filehide'
import ItemTable from './components/ItemTable.vue'

const tab = ref<'home' | 'active' | 'history' | 'settings' | 'about'>('home')
const items = ref<HiddenItem[]>([])
const busy = ref(false)
const message = ref('')
const recoveryBusy = ref(false)
const recoveryScanId = ref('')
const recoveryScanned = ref(0)
const recoverySkipped = ref(0)
const recoveryCandidates = ref<RecoveryCandidate[]>([])
const appUnlocked = ref(false)
const accessPassword = ref('')
const passwordEnabled = ref(false)
const currentPassword = ref('')
const newPassword = ref('')
const confirmPassword = ref('')
const autoLockMinutes = ref(15)
const contextMenuActive = ref(false)
const storage = ref<StorageInfo | null>(null)
const dragActive = ref(false)
const dragPaths = ref<string[]>([])
const dragRiskCount = ref(0)
let dragPreviewKey = ''
const query = ref('')
const typeFilter = ref<'ALL' | 'FILE' | 'FOLDER'>('ALL')
const statusFilter = ref<'ALL' | HiddenItem['protection_status']>('ALL')
const sortOrder = ref<'newest' | 'oldest'>('newest')
const filtersActive = computed(() => Boolean(query.value.trim()) || typeFilter.value !== 'ALL' || statusFilter.value !== 'ALL' || sortOrder.value !== 'newest')
const clearFilters = () => { query.value = ''; typeFilter.value = 'ALL'; statusFilter.value = 'ALL'; sortOrder.value = 'newest' }
let unlistenRecovery: UnlistenFn | undefined
let unlistenDragDrop: UnlistenFn | undefined
let sessionTimer: number | undefined
const onBatchRestored = async (event: Event) => { const result = (event as CustomEvent<BatchResult>).detail; await refresh(); message.value = result.failed.length ? `批量恢复完成：成功 ${result.succeeded.length} 项，失败 ${result.failed.length} 项。` : `批量恢复完成：${result.succeeded.length} 项。` }
const onHistoryDeleted = async (event: Event) => { const result = (event as CustomEvent<DeleteHistoryResult>).detail; await refresh(); message.value = result.failed.length ? `历史删除完成：删除 ${result.succeeded.length} 项，失败 ${result.failed.length} 项。` : `已安全删除 ${result.succeeded.length} 条历史记录。` }
const active = computed(() => items.value.filter(item => item.current_status === 1))
const restored = computed(() => items.value.filter(item => item.current_status === 0))
const shown = computed(() => {
  const normalized = query.value.trim().toLocaleLowerCase()
  return (tab.value === 'history' ? items.value : active.value)
    .filter(item => typeFilter.value === 'ALL' || item.item_type === typeFilter.value)
    .filter(item => statusFilter.value === 'ALL' || item.protection_status === statusFilter.value)
    .filter(item => !normalized || item.path.toLocaleLowerCase().includes(normalized))
    .sort((a, b) => {
      const diff = new Date(a.update_time).getTime() - new Date(b.update_time).getTime()
      return sortOrder.value === 'newest' ? -diff : diff
    })
})
const refresh = async () => { items.value = await listItems(); if (appUnlocked.value) { try { contextMenuActive.value = await contextMenuEnabled(); storage.value = await storageInfo() } catch {} } }
const select = async (folder: boolean) => {
  try {
    const selected = await open({ directory: folder, multiple: true, title: folder ? '选择要隐藏的文件夹（可多选）' : '选择要隐藏的文件（可多选）' })
    const paths = !selected ? [] : Array.isArray(selected) ? selected : [selected]
    if (!paths.length) return
    busy.value = true
    const previews = await previewPaths(paths)
    const risks = previews.filter(item => item.risks.length)
    if (risks.length && !window.confirm(`选中的 ${paths.length} 项中有 ${risks.length} 项存在风险：\n${risks.slice(0, 5).map(item => `${item.path}: ${item.risks.join('、')}`).join('\n')}\n\n仍要继续吗？`)) { busy.value = false; return }
    const result = folder ? await hideFolders(paths) : await hideFiles(paths)
    await refresh(); tab.value = 'active'; message.value = result.failed.length ? `已锁定 ${result.succeeded.length} 项，${result.failed.length} 项失败。` : `已锁定 ${result.succeeded.length} 项。`
  } catch (error) { message.value = String(error) } finally { busy.value = false }
}
const restore = async (id: number) => { try { busy.value = true; await restoreItem(id); await refresh(); message.value = '已恢复原始文件属性。' } catch (error) { message.value = String(error) } finally { busy.value = false } }
const restoreBatch = async (ids: number[]) => { try { busy.value = true; const result = await restoreItems(ids); await refresh(); message.value = result.failed.length ? `批量恢复完成：成功 ${result.succeeded.length} 项，失败 ${result.failed.length} 项。` : `批量恢复完成：${result.succeeded.length} 项。` } catch (error) { message.value = String(error) } finally { busy.value = false } }
const finishRecoveryScan = () => { recoveryBusy.value = false; unlistenRecovery?.(); unlistenRecovery = undefined }
const beginRecoveryScan = async () => {
  try {
    const path = await open({ directory: true, multiple: false, title: '选择要找回的目录' })
    if (!path || Array.isArray(path)) return
    recoveryScanId.value = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`
    recoveryScanned.value = 0; recoverySkipped.value = 0; recoveryCandidates.value = []; recoveryBusy.value = true
    unlistenRecovery?.()
    unlistenRecovery = await listen<RecoveryScanUpdate>('recovery-scan-update', event => {
      const update = event.payload
      if (update.scan_id !== recoveryScanId.value) return
      recoveryScanned.value = update.scanned; recoverySkipped.value = update.skipped
      if (update.kind === 'completed') { recoveryCandidates.value = update.candidates; finishRecoveryScan(); message.value = `扫描完成：找到 ${update.candidates.length} 个可恢复项目。` }
      if (update.kind === 'cancelled') { finishRecoveryScan(); message.value = '已取消找回扫描。' }
    })
    await startRecoveryScan(path, recoveryScanId.value)
  } catch (error) { finishRecoveryScan(); message.value = String(error) }
}
const cancelScan = async () => { try { await cancelRecoveryScan(recoveryScanId.value) } catch (error) { message.value = String(error) } }
const recoverCandidate = async (path: string) => { try { busy.value = true; await recoverMarkedItem(path); recoveryCandidates.value = recoveryCandidates.value.filter(item => item.path !== path); await refresh(); message.value = '已找回并恢复原始文件属性。' } catch (error) { message.value = String(error) } finally { busy.value = false } }
const unlock = async () => { try { busy.value = true; if (await verifyPassword(accessPassword.value)) { appUnlocked.value = true; accessPassword.value = ''; await refresh() } else { message.value = '访问密码不正确。' } } catch (error) { message.value = String(error) } finally { busy.value = false } }
const saveAccessPassword = async () => {
  if (newPassword.value !== confirmPassword.value) { message.value = '两次输入的新密码不一致。'; return }
  try { busy.value = true; await setAccessPassword(passwordEnabled.value ? currentPassword.value : undefined, newPassword.value); passwordEnabled.value = true; currentPassword.value = ''; newPassword.value = ''; confirmPassword.value = ''; message.value = '访问密码已启用。下次启动 FileHide 时需要先解锁。' } catch (error) { message.value = String(error) } finally { busy.value = false }
}
const disableAccessPassword = async () => { try { busy.value = true; await clearAccessPassword(currentPassword.value); passwordEnabled.value = false; currentPassword.value = ''; message.value = '访问密码已关闭。' } catch (error) { message.value = String(error) } finally { busy.value = false } }
const saveAutoLock = async () => { try { await setAutoLockMinutes(autoLockMinutes.value); message.value = autoLockMinutes.value ? `自动锁定已设置为 ${autoLockMinutes.value} 分钟。` : '自动锁定已关闭。' } catch (error) { message.value = String(error) } }
const toggleContextMenu = async () => { const next = !contextMenuActive.value; try { await setContextMenuEnabled(next); contextMenuActive.value = next; message.value = next ? '右键菜单已添加。' : '右键菜单已移除。' } catch (error) { message.value = String(error) } }
const lockDropped = async (paths: string[]) => { if (!paths.length || !appUnlocked.value) return; try { busy.value = true; const previews = await previewPaths(paths); const risks = previews.filter(item => item.risks.length); if (risks.length && !window.confirm(`拖入项目存在 ${risks.length} 项风险，仍要继续锁定吗？`)) return; const result = await hidePaths(paths); await refresh(); tab.value = 'active'; message.value = result.failed.length ? `拖拽锁定完成：成功 ${result.succeeded.length} 项，失败 ${result.failed.length} 项。` : `拖拽锁定完成：${result.succeeded.length} 项。` } catch (error) { message.value = String(error) } finally { busy.value = false } }
onMounted(async () => { window.addEventListener('filehide-batch-restored', onBatchRestored); try { passwordEnabled.value = await passwordConfigured(); autoLockMinutes.value = await getAutoLockMinutes(); appUnlocked.value = !passwordEnabled.value; if (appUnlocked.value) { await refresh(); const health = await healthCheck(); if (health.changed) message.value = `健康检查发现 ${health.changed} 项状态发生变化，请查看列表。` } unlistenDragDrop = await getCurrentWebview().onDragDropEvent(event => { const type = event.payload.type; if (type === 'enter' || type === 'over') dragActive.value = true; else if (type === 'leave') dragActive.value = false; else if (type === 'drop') { dragActive.value = false; void lockDropped(event.payload.paths) } }); sessionTimer = window.setInterval(async () => { if (passwordEnabled.value && appUnlocked.value && !(await checkSession())) { appUnlocked.value = false; items.value = []; message.value = '会话已自动锁定。' } }, 30000) } catch (error) { message.value = String(error) } })
onBeforeUnmount(() => { window.removeEventListener('filehide-batch-restored', onBatchRestored); unlistenRecovery?.(); unlistenDragDrop?.(); if (sessionTimer) window.clearInterval(sessionTimer) })
const previewDragged = async (paths: string[]) => { if (!appUnlocked.value || !paths.length) return; const key = paths.join('\n'); if (key === dragPreviewKey) return; dragPreviewKey = key; dragPaths.value = paths; try { const previews = await previewPaths(paths); if (key === dragPreviewKey) dragRiskCount.value = previews.filter(item => item.risks.length).length } catch { if (key === dragPreviewKey) dragRiskCount.value = 0 } }
onMounted(async () => { try { await getCurrentWebview().onDragDropEvent(event => { if (event.payload.type === 'enter') void previewDragged(event.payload.paths); if (event.payload.type === 'leave' || event.payload.type === 'drop') { dragPaths.value = []; dragRiskCount.value = 0; dragPreviewKey = '' } }) } catch {} })
onMounted(() => window.addEventListener('filehide-history-deleted', onHistoryDeleted))
onBeforeUnmount(() => window.removeEventListener('filehide-history-deleted', onHistoryDeleted))
</script>

<template>
  <main v-if="!appUnlocked" class="access-gate">
    <section class="access-card">
      <div class="access-mark">F</div>
      <p class="eyebrow">FILEHIDE / ACCESS</p>
      <h1>输入访问密码</h1>
      <p>此密码用于解锁 FileHide 应用，不会加密文件内容。</p>
      <form @submit.prevent="unlock"><input v-model="accessPassword" type="password" autocomplete="current-password" maxlength="128" placeholder="访问密码" autofocus><button class="primary" :disabled="busy">解锁 FileHide</button></form>
    </section>
  </main>
  <main v-else class="shell">
    <div v-if="dragActive" class="drop-overlay"><div><strong>松开以快速锁定</strong><span>{{ dragPaths.length || '若干' }} 个项目</span><small v-if="dragRiskCount">检测到 {{ dragRiskCount }} 项风险，松开后会再次确认</small><small v-else>正在检查路径、重复记录和链接风险</small></div></div>
    <aside class="sidebar"><div class="brand"><span>F</span><div>FileHide<small>PRIVATE SPACE</small></div></div>
      <nav><button :class="{selected: tab === 'home'}" @click="tab = 'home'">◈ <span>首页</span></button><button :class="{selected: tab === 'active'}" @click="tab = 'active'">◉ <span>隐藏项目</span><b>{{ active.length }}</b></button><button :class="{selected: tab === 'history'}" @click="tab = 'history'">◷ <span>历史记录</span></button><button :class="{selected: tab === 'settings'}" @click="tab = 'settings'">⚙ <span>设置</span></button></nav>
      <div class="sidebar-bottom"><button :class="{selected: tab === 'about'}" @click="tab = 'about'">ⓘ <span>关于</span></button><p><i></i> 保护服务运行中</p></div></aside>
    <section class="content"><header><div><p class="eyebrow">FILEHIDE / V1.1</p><h1>{{ tab === 'home' ? '欢迎回来' : tab === 'active' ? '隐藏项目' : tab === 'history' ? '恢复历史' : tab === 'about' ? '关于 FileHide' : '设置' }}</h1></div><button class="ghost" @click="refresh">↻ 刷新</button></header>
      <template v-if="tab === 'home'"><div class="hero"><div><p class="eyebrow">快速锁定，瞬间生效</p><h2>把敏感文件<br><em>藏在视线之外。</em></h2><p>不移动、不加密、不递归。仅修改所选项目本身的 Windows 属性。</p><div class="actions"><button class="primary" :disabled="busy" @click="select(false)">＋ 锁定文件</button><button class="secondary" :disabled="busy" @click="select(true)">＋ 锁定文件夹</button></div></div><div class="orb">⌁</div></div>
      <p class="quick-lock-note">快速锁定不是加密：熟悉 Windows 设置的用户仍可显示隐藏项目。</p>
      <div class="stats"><article><small>正在隐藏</small><strong>{{ active.length }}</strong><span>个项目受保护</span></article><article><small>恢复历史</small><strong>{{ restored.length }}</strong><span>条安全记录</span></article><article><small>处理方式</small><strong>毫秒级</strong><span>不扫描子目录</span></article></div>
      <section class="panel"><div class="panel-title"><h3>最近隐藏</h3><button @click="tab = 'active'">查看全部 →</button></div><ItemTable :items="active.slice(0, 5)" :show-restore="true" @restore="restore" /></section></template>
      <template v-else-if="tab === 'active' || tab === 'history'"><div class="section-actions"><p>{{ tab === 'active' ? '恢复会准确还原隐藏前的属性；路径失效项目不可恢复。' : '全部锁定与恢复操作记录。' }}</p><div v-if="tab === 'active'"><button class="secondary" :disabled="busy" @click="select(false)">锁定文件</button><button class="primary" :disabled="busy" @click="select(true)">锁定文件夹</button></div></div><p class="quick-lock-note compact">快速锁定不是加密：它用于防止日常浏览，不阻止熟悉 Windows 设置的用户。</p><section class="panel"><ItemTable :items="shown" :show-restore="true" @restore="restore" /></section></template>
      <template v-else-if="tab === 'settings'"><section class="panel prose access-settings"><h3>访问密码</h3><p>{{ passwordEnabled ? '已启用。下次启动 FileHide 时需要先输入密码解锁。' : '未启用。启用后，首次进入 FileHide 需要输入访问密码。' }}</p><div class="password-form"><input v-if="passwordEnabled" v-model="currentPassword" type="password" autocomplete="current-password" maxlength="128" placeholder="当前密码"><input v-model="newPassword" type="password" autocomplete="new-password" maxlength="128" placeholder="新密码（至少 8 个字符）"><input v-model="confirmPassword" type="password" autocomplete="new-password" maxlength="128" placeholder="确认新密码"><div class="recovery-actions"><button class="primary" :disabled="busy" @click="saveAccessPassword">{{ passwordEnabled ? '更新密码' : '启用访问密码' }}</button><button v-if="passwordEnabled" class="secondary" :disabled="busy || !currentPassword" @click="disableAccessPassword">关闭访问密码</button></div></div><p class="password-warning">请妥善保存密码。它是应用门禁，不是文件加密；忘记密码后若清除应用数据库，普通历史记录会丢失。</p></section><section class="panel prose recovery"><h3>找回锁定项目</h3><p>仅在你清除了 FileHide 历史记录后使用。选择一个大致目录后，FileHide 只读取文件属性与 NTFS 恢复标记，不读取文件内容；不会自动全盘扫描。</p><div class="recovery-actions"><button class="primary" :disabled="recoveryBusy || busy" @click="beginRecoveryScan">选择目录并扫描</button><button v-if="recoveryBusy" class="secondary" @click="cancelScan">取消扫描</button></div><p v-if="recoveryBusy" class="scan-progress">正在扫描：已检查 {{ recoveryScanned }} 项，跳过 {{ recoverySkipped }} 项。</p><div v-if="recoveryCandidates.length" class="recovery-results"><p>找到 {{ recoveryCandidates.length }} 个带 FileHide 标记且仍处于锁定状态的项目：</p><div v-for="candidate in recoveryCandidates" :key="candidate.path" class="recovery-row"><span><b>{{ candidate.path.split(/[/\\]/).filter(Boolean).pop() }}</b><small>{{ candidate.path }}</small></span><button class="restore" :disabled="busy" @click="recoverCandidate(candidate.path)">找回并恢复</button></div></div></section></template>
      <template v-else><section class="panel prose"><h3>FileHide V1.1</h3><p>面向 Windows 10/11 的快速锁定工具。文件内容从不被读取、移动或加密。</p></section></template>
      <section v-if="tab === 'settings'" class="panel prose auto-lock"><h3>会话自动锁定</h3><p>闲置后自动回到密码门禁，减少无人值守风险。</p><select v-model.number="autoLockMinutes" @change="saveAutoLock"><option :value="0">关闭</option><option :value="5">5 分钟</option><option :value="15">15 分钟</option><option :value="30">30 分钟</option><option :value="60">60 分钟</option></select></section>
      <section v-if="tab === 'settings' && storage" class="panel prose"><h3>数据存储模式</h3><p v-if="storage.mode === 'PORTABLE'">当前为便携模式，数据库位于 EXE 同目录：{{ storage.path }}</p><p v-else>当前为标准模式，数据库位于 Windows 应用数据目录。</p><p class="password-warning">便携模式适合 U 盘使用，请保护好整个目录；数据库不保存明文密码。</p></section>
      <section v-if="tab === 'settings'" class="panel prose"><h3>资源管理器右键菜单</h3><p>在文件或文件夹右键菜单中添加“使用 FileHide 锁定”。仅写入当前用户注册表，不需要管理员权限。</p><button class="secondary" :disabled="busy" @click="toggleContextMenu">{{ contextMenuActive ? '移除右键菜单' : '添加到右键菜单' }}</button></section>
      <section v-if="tab === 'active' || tab === 'history'" class="list-tools"><input v-model="query" type="search" placeholder="搜索名称或路径"><select v-model="typeFilter"><option value="ALL">全部类型</option><option value="FILE">文件</option><option value="FOLDER">文件夹</option></select><select v-model="statusFilter"><option value="ALL">全部状态</option><option value="LOCKED">已锁定</option><option value="PATH_MOVED">已移动</option><option value="PATH_REPLACED">已替换</option><option value="PATH_DELETED">已删除</option><option value="UNLOCKED_EXTERNALLY">已外部解除</option><option value="RESTORED">已恢复</option></select><select v-model="sortOrder"><option value="newest">最新操作</option><option value="oldest">最早操作</option></select></section>
      <Transition><div v-if="message" class="toast" @click="message = ''">{{ message }}</div></Transition>
      <button v-if="filtersActive && (tab === 'active' || tab === 'history')" class="ghost clear-filters" @click="clearFilters">清除筛选</button>
    </section>
  </main>
</template>
