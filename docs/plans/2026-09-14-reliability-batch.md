# FileHide Reliability Batch Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add reliable recovery workflows for batch restore, pre-lock risk preview, interrupted transactions, startup health checks, and explicit external path-change states.

**Architecture:** Keep each file operation independently transactional in Rust, persist a small operation journal in SQLite, and expose a single health report to the Vue UI. Batch restore will reuse the existing single-item restore validation so one failure cannot corrupt other records.

**Tech Stack:** Rust/Tauri 2, SQLite/rusqlite, Windows file attributes/NTFS ADS, Vue 3/TypeScript, Vitest.

---

### Task 1: Expand status and health models

**Files:** `src-tauri/src/models.rs`, `src/api/filehide.ts`, `src/components/ItemTable.vue`

Add explicit `PATH_MOVED`, `PATH_REPLACED`, `PATH_DELETED`, and `TRANSACTION_PENDING` states plus serializable health summaries. Update labels and tests.

### Task 2: Add operation journal and interrupted-transaction recovery

**Files:** `src-tauri/src/database.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`

Create an `operation_journal` table with operation id, item id/path, phase, and timestamps. Lock operations record `prepared`, `marked`, `attributes_applied`, and `committed`; startup repairs non-committed rows by removing compensating records or restoring original attributes where safe.

### Task 3: Implement batch restore

**Files:** `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`, `src/api/filehide.ts`, `src/components/ItemTable.vue`, `src/App.vue`

Add checkbox selection and `restore_items(ids)` returning per-item success/failure. Keep single-item restore behavior unchanged and disable batch actions while a transaction is active.

### Task 4: Add lock preview and risk confirmation

**Files:** `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`, `src/api/filehide.ts`, `src/App.vue`

Add a read-only `preview_paths` command that classifies file/folder type, filesystem, reparse status, existing marker, duplicate history, and permission risk. Require confirmation when any risk is reported; preserve a fast path for clean selections.

### Task 5: Startup health check and explicit external-change handling

**Files:** `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`, `src/api/filehide.ts`, `src/App.vue`

Run reconciliation after opening the database and expose `health_check`. Distinguish deleted, moved, replaced, externally-unlocked, and healthy items. Add a UI banner with refresh and safe-action guidance.

### Task 6: Verification and documentation

Run `cargo fmt --check`, `cargo test`, Vitest, `npm run build`, and `npm run tauri build`. Update README with transaction recovery, preview behavior, and status meanings.
