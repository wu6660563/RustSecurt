# Quick Lock Folder Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make fast folder locking explicit, observable, and safely recoverable without adding encryption or recursive file operations.

**Architecture:** The Rust backend derives a live protection state from the existing database record and the target folder's current Windows attributes. The Vue UI renders that state as a status chip, blocks invalid restoration attempts, and describes the feature's visibility-only security boundary.

**Tech Stack:** Rust, Tauri 2, windows-rs, SQLite, Vue 3, TypeScript, Vitest.

**Spec:** `FileHide_V1.0_详细设计方案_Codex版.md`, plus the confirmed V1.1 behavior: lock only the selected folder itself with Hidden and System attributes; no encryption, ACL, service, or recursion.

## Global Constraints

- Target platform is Windows 10 and Windows 11.
- Rust performs all file operations through `windows-rs`; frontend code does not access the Node filesystem.
- Folder locking must remain non-recursive and preserve original Windows attributes for restoration.
- The interface must never describe quick locking as encryption or a guarantee against technical users.
- Existing SQLite rows must remain compatible; live state is calculated, not migrated into the database.

---

### Task 1: Derive and expose live protection states

**Files:**
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/commands.rs`
- Test: `src-tauri/src/commands.rs`

**Interfaces:**
- Consumes: `HiddenItem.current_status`, saved `path`, and `GetFileAttributesW`.
- Produces: `HiddenItem.protection_status: String` with one of `LOCKED`, `RESTORED`, `MISSING`, or `UNLOCKED_EXTERNALLY`.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn active_item_with_hidden_and_system_attributes_is_locked() {
    assert_eq!(protection_status(1, Ok(0x2 | 0x4)), "LOCKED");
}

#[test]
fn active_item_with_missing_path_is_marked_missing() {
    assert_eq!(protection_status(1, Err(())), "MISSING");
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml protection_status`

Expected: compilation failure because `protection_status` does not exist.

- [ ] **Step 3: Write the minimal implementation**

```rust
fn protection_status(current_status: i32, attributes: Result<u32, ()>) -> &'static str {
    if current_status == 0 { return "RESTORED"; }
    match attributes {
        Err(()) => "MISSING",
        Ok(value) if value & 0x2 != 0 && value & 0x4 != 0 => "LOCKED",
        Ok(_) => "UNLOCKED_EXTERNALLY",
    }
}
```

Set the public `protection_status` field while returning `list_items`; reject restoration of a missing item before attempting `SetFileAttributesW`.

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --manifest-path src-tauri/Cargo.toml protection_status`

Expected: all protection-state tests pass.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/models.rs src-tauri/src/commands.rs
git commit -m "feat: expose quick lock protection states"
```

### Task 2: Render lock state and security boundary in the UI

**Files:**
- Modify: `src/api/filehide.ts`
- Modify: `src/components/ItemTable.vue`
- Modify: `src/App.vue`
- Modify: `src/style.css`
- Test: `src/App.test.ts`

**Interfaces:**
- Consumes: `HiddenItem.protection_status` from Task 1.
- Produces: a visible status label and a quick-lock notice; restoration buttons only appear for `LOCKED` items.

- [ ] **Step 1: Write the failing test**

```typescript
it('shows the quick-lock warning and marks a locked item as locked', async () => {
  const wrapper = mount(App)
  await flushPromises()

  expect(wrapper.text()).toContain('快速锁定不是加密')
  expect(wrapper.text()).toContain('已锁定')
})
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `npm test -- --run src/App.test.ts`

Expected: assertion failure because the notice and lock label do not exist.

- [ ] **Step 3: Write the minimal implementation**

```vue
<p class="quick-lock-note">快速锁定不是加密：熟悉 Windows 设置的用户仍可显示隐藏项目。</p>
<span class="status status-locked">已锁定</span>
```

Map the four backend status strings to Chinese labels, show a missing-path label for invalid items, and condition the restore button on `protection_status === 'LOCKED'`.

- [ ] **Step 4: Run the test to verify it passes**

Run: `npm test -- --run src/App.test.ts`

Expected: the quick-lock UI test and existing list/history tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/api/filehide.ts src/components/ItemTable.vue src/App.vue src/style.css src/App.test.ts
git commit -m "feat: clarify quick lock status in the interface"
```

### Task 3: Verify release behavior

**Files:**
- Verify: `src-tauri/target/release/filehide.exe`

**Interfaces:**
- Consumes: backend state and Vue UI from Tasks 1 and 2.
- Produces: a release executable with no console window and the updated quick-lock UX.

- [ ] **Step 1: Run all automated tests**

Run: `npm test; cargo test --lib --manifest-path src-tauri/Cargo.toml`

Expected: all Vitest and Rust tests pass.

- [ ] **Step 2: Build the release executable**

Run: `npm run tauri build`

Expected: exit code 0 and `src-tauri/target/release/filehide.exe` is rebuilt.

- [ ] **Step 3: Verify the executable remains a GUI app**

Run: PowerShell code that reads the PE subsystem field at `e_lfanew + 92`.

Expected: subsystem value `2` for Windows GUI.

- [ ] **Step 4: Commit**

```bash
git add docs/superpowers/plans/2026-09-07-quick-lock-folder.md
git commit -m "docs: plan quick lock folder improvements"
```

## Self-Review

- Spec coverage: Tasks 1 and 2 preserve non-recursive attribute locking, surface its real state, and state that it is not encryption. Task 3 verifies the distributable Windows executable.
- Placeholder scan: no `TBD`, `TODO`, or deferred implementation instructions are present.
- Type consistency: backend emits `protection_status`; the TypeScript `HiddenItem` interface and Vue component consume that exact field.
