import { invoke } from '@tauri-apps/api/core'

export type ProtectionStatus = 'LOCKED' | 'RESTORED' | 'MISSING' | 'UNLOCKED_EXTERNALLY'
export interface HiddenItem { id: number; path: string; item_type: 'FILE' | 'FOLDER'; original_attributes: number; current_status: number; protection_status: ProtectionStatus; create_time: string; update_time: string }
export interface RecoveryCandidate { path: string; item_type: 'FILE' | 'FOLDER'; original_attributes: number }
export interface RecoveryScanUpdate { scan_id: string; kind: 'progress' | 'completed' | 'cancelled'; scanned: number; skipped: number; candidates: RecoveryCandidate[] }
export const listItems = () => invoke<HiddenItem[]>('list_items')
export const hideFile = (path: string) => invoke<HiddenItem>('hide_file', { path })
export const hideFolder = (path: string) => invoke<HiddenItem>('hide_folder', { path })
export const restoreItem = (id: number) => invoke<HiddenItem>('restore_item', { id })
export const startRecoveryScan = (path: string, scanId: string) => invoke<void>('start_recovery_scan', { path, scanId })
export const cancelRecoveryScan = (scanId: string) => invoke<void>('cancel_recovery_scan', { scanId })
export const recoverMarkedItem = (path: string) => invoke<HiddenItem>('recover_marked_item', { path })
export const passwordConfigured = () => invoke<boolean>('password_configured')
export const verifyPassword = (password: string) => invoke<boolean>('verify_password', { password })
export const setAccessPassword = (currentPassword: string | undefined, newPassword: string) => invoke<void>('set_access_password', { currentPassword, newPassword })
export const clearAccessPassword = (currentPassword: string) => invoke<void>('clear_access_password', { currentPassword })
