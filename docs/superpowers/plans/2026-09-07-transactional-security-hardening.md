# Transactional Security Hardening Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Ensure fast lock and restore never silently leave the filesystem and history database in contradictory states, while reducing local-app attack surface.

**Architecture:** Validate a selected target before writing a history record, reject Windows reparse points, then use a compensating database delete if setting the lock attributes fails. Restore reverses that order and attempts to re-lock the target if the database status update fails. SQLite is configured for durable WAL writes; the webview is constrained to local resources by CSP.

**Tech Stack:** Rust, Tauri 2, windows-rs, rusqlite, Vue 3, Vitest.

---

### Task 1: Test and harden selected-path validation

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Test: `src-tauri/src/commands.rs`

1. Add failing unit tests for extended-path prefix cleanup and reparse-point detection.
2. Run `cargo test --manifest-path src-tauri/Cargo.toml path_` and confirm the missing helpers cause failure.
3. Add a target resolver that rejects reparse points, canonicalizes existing paths, removes only the Windows extended-path display prefix, and checks the expected file/folder type.
4. Run the focused Rust tests and then all Rust tests.

### Task 2: Make filesystem/database updates compensating operations

**Files:**
- Modify: `src-tauri/src/database.rs`
- Modify: `src-tauri/src/commands.rs`
- Test: `src-tauri/src/database.rs`

1. Add a failing database test showing an active history row can be removed by its compensating operation.
2. Run `cargo test --manifest-path src-tauri/Cargo.toml remove_active_item` and confirm it fails because the method does not exist.
3. Add `remove_active_item`, durable SQLite pragmas, and a small test-only in-memory database constructor.
4. After a failed lock attribute write, remove the just-created active row. After a failed restore database update, attempt to restore the locked attributes and return an error that says whether compensation succeeded.
5. Run all Rust tests.

### Task 3: Lock down webview resource loading

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src/style.css`
- Test: `src/App.test.ts`

1. Add a failing frontend test that asserts no Google Fonts URL is left in the application stylesheet.
2. Run `npm test -- --run src/App.test.ts` and observe failure.
3. Remove the remote font import and replace the permissive null CSP with a local-only policy that permits no remote scripts or connections.
4. Run the focused frontend tests and production frontend build.

### Task 4: Release verification

1. Run `npm test`.
2. Run `cargo test --manifest-path src-tauri/Cargo.toml`.
3. Run `npm run tauri build`.
4. Verify `src-tauri/target/release/filehide.exe` still has PE subsystem `2` (Windows GUI).
5. Review generated `dist` for any external `http://` or `https://` resource references.

## Non-goals

- This does not claim to make hidden files cryptographically protected.
- This does not add recursive folder encryption, ACL deny rules, a driver, or a background service.
- Existing history paths remain plaintext because they are necessary for FileHide to locate and restore items.
