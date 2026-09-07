# ADS Recovery Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Let a user explicitly scan a selected NTFS folder for FileHide-marked items and safely restore a selected result after the local history database is lost.

**Architecture:** Every newly locked file or folder gets a small `:FileHide` NTFS alternate data stream containing a versioned marker, target type, and original attributes. A background Tauri command recursively enumerates only a user-selected root, skips reparse points, reads at most 128 marker bytes per entry, and emits progress/cancellation events. Recovery revalidates the marker and lock attributes, then restores attributes, removes the marker, and writes a recovered history entry using compensating cleanup on failure.

**Tech Stack:** Rust, Tauri 2 events, windows-rs, NTFS ADS, SQLite, Vue 3, TypeScript, Vitest.

---

### Task 1: Marker primitives and locking transaction

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Test: `src-tauri/src/commands.rs`

1. Add failing parser tests for a valid versioned marker and malformed marker input.
2. Run the focused Rust test and confirm failure because marker helpers do not exist.
3. Add bounded ADS read/write/remove helpers and insert the marker before committing a new lock; compensate by removing the marker and/or history row on any failed later step.
4. Run all Rust tests.

### Task 2: Background recovery scan and safe recovery command

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/database.rs`
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/commands.rs`

1. Add failing tests for rejecting reparse-point traversal and parsing a recovery candidate.
2. Add progress event payloads, cancellation registry, bounded recursive enumeration, and `recover_marked_item`.
3. Ensure a scan never reads file content, only directory metadata and the first 128 bytes of the named stream.
4. Run Rust tests.

### Task 3: Recovery UI

**Files:**
- Modify: `src/api/filehide.ts`
- Modify: `src/App.vue`
- Modify: `src/style.css`
- Modify: `src/App.test.ts`

1. Add a failing UI test for the recovery action in Settings.
2. Add a user-initiated folder chooser, event listener, progress display, cancel control, result list, and per-result recovery button.
3. Run frontend tests and build.

### Task 4: Release verification

1. Run all Vitest and Rust tests.
2. Build `filehide.exe`.
3. Verify GUI PE subsystem and final executable timestamp.

### Task 5: Optional local access password

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/database.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/api/filehide.ts`
- Modify: `src/App.vue`
- Modify: `src/App.test.ts`

1. Add failing backend tests for password validation and a failing UI test for the unlock gate.
2. Store only an Argon2id PHC password hash in SQLite, using a random salt; never store or log a plaintext password.
3. Add an in-memory session state. Once a password exists, every operation that exposes or changes hidden records must require a successful unlock in the current app process.
4. Let Settings enable, replace, or remove the password. Replacement/removal requires the current password; first-time enablement requires a confirmed new password.
5. Run all frontend/Rust tests and release build.

## Constraints

- No automatic, startup, or whole-disk scanning.
- ADS recovery requires NTFS; a clear error is returned for unsupported volumes.
- Old items without the new marker remain recoverable from their existing SQLite history but cannot be found by a post-uninstall scan.
- Scan results are not automatically restored; each recovery requires an explicit user click.
- The password is an app access gate, not file encryption or an OS-level access-control boundary.
