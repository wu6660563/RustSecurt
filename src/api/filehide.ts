import { invoke } from '@tauri-apps/api/core'

export type ProtectionStatus = 'LOCKED' | 'RESTORED' | 'MISSING' | 'UNLOCKED_EXTERNALLY' | 'PATH_CHANGED' | 'PATH_MOVED' | 'PATH_REPLACED' | 'PATH_DELETED' | 'TRANSACTION_PENDING'
export interface HiddenItem { id: number; path: string; item_type: 'FILE' | 'FOLDER'; original_attributes: number; current_status: number; protection_status: ProtectionStatus; create_time: string; update_time: string; file_id?: string | null }
export interface BatchFailure { path: string; error: string }
export interface BatchResult { succeeded: HiddenItem[]; failed: BatchFailure[] }
export interface PreviewItem { path: string; item_type: 'FILE' | 'FOLDER' | 'UNKNOWN'; risks: string[]; can_lock: boolean }
export interface HealthSummary { checked: number; healthy: number; changed: number }
export interface DeleteHistoryResult { succeeded: number[]; failed: BatchFailure[] }
export interface StorageInfo { mode: 'PORTABLE' | 'APP_DATA'; path: string; portable_marker: string }
export interface RecoveryCandidate { path: string; item_type: 'FILE' | 'FOLDER'; original_attributes: number }
export interface RecoveryScanUpdate { scan_id: string; kind: 'progress' | 'completed' | 'cancelled'; scanned: number; skipped: number; candidates: RecoveryCandidate[] }
export const listItems = () => invoke<HiddenItem[]>('list_items')
export const hideFile = (path: string) => invoke<HiddenItem>('hide_file', { path })
export const hideFolder = (path: string) => invoke<HiddenItem>('hide_folder', { path })
export const hideFiles = (paths: string[]) => invoke<BatchResult>('hide_files', { paths })
export const hideFolders = (paths: string[]) => invoke<BatchResult>('hide_folders', { paths })
export const hidePaths = (paths: string[]) => invoke<BatchResult>('hide_paths', { paths })
export const previewPaths = (paths: string[]) => invoke<PreviewItem[]>('preview_paths', { paths })
export const healthCheck = () => invoke<HealthSummary>('health_check')
export const deleteHistory = (ids: number[], removeMarkers = true) => invoke<DeleteHistoryResult>('delete_history', { ids, removeMarkers })
export const storageInfo = () => invoke<StorageInfo>('storage_info')
export const contextMenuEnabled = () => invoke<boolean>('context_menu_enabled')
export const setContextMenuEnabled = (enabled: boolean) => invoke<void>('set_context_menu_enabled', { enabled })
export const restoreItem = (id: number) => invoke<HiddenItem>('restore_item', { id })
export const restoreItems = (ids: number[]) => invoke<BatchResult>('restore_items', { ids })
export const startRecoveryScan = (path: string, scanId: string) => invoke<void>('start_recovery_scan', { path, scanId })
export const cancelRecoveryScan = (scanId: string) => invoke<void>('cancel_recovery_scan', { scanId })
export const recoverMarkedItem = (path: string) => invoke<HiddenItem>('recover_marked_item', { path })
export const passwordConfigured = () => invoke<boolean>('password_configured')
export const verifyPassword = (password: string) => invoke<boolean>('verify_password', { password })
export const setAccessPassword = (currentPassword: string | undefined, newPassword: string) => invoke<void>('set_access_password', { currentPassword, newPassword })
export const clearAccessPassword = (currentPassword: string) => invoke<void>('clear_access_password', { currentPassword })
export const getAutoLockMinutes = () => invoke<number>('get_auto_lock_minutes')
export const setAutoLockMinutes = (minutes: number) => invoke<void>('set_auto_lock_minutes', { minutes })
export const checkSession = () => invoke<boolean>('check_session')
export const lockSession = () => invoke<void>('lock_session')
