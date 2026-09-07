use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
    os::windows::ffi::OsStrExt,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Algorithm, Argon2, Params, Version,
};
use rand_core::OsRng;
use tauri::{AppHandle, Emitter, State};
use windows::{
    core::PCWSTR,
    Win32::Storage::FileSystem::{
        GetFileAttributesW, SetFileAttributesW, FILE_FLAGS_AND_ATTRIBUTES, INVALID_FILE_ATTRIBUTES,
    },
};

use crate::{
    database::Database,
    file_attributes,
    models::{BatchFailure, BatchResult, HiddenItem, RecoveryCandidate, RecoveryScanUpdate},
};

const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
const RECOVERY_STREAM: &str = "FileHide";
const RECOVERY_MARKER_VERSION: &str = "FILEHIDE/1";
const MAX_MARKER_BYTES: u64 = 128;

pub struct RecoveryScans(pub Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>);

pub struct AccessSession(pub Mutex<SessionState>);

pub struct SessionState {
    pub unlocked: bool,
    pub last_activity: Option<Instant>,
}

impl Default for AccessSession {
    fn default() -> Self {
        Self(Mutex::new(SessionState {
            unlocked: false,
            last_activity: None,
        }))
    }
}

impl Default for RecoveryScans {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(HashMap::new())))
    }
}

#[derive(Debug, Clone)]
struct RecoveryMarker {
    item_type: String,
    original_attributes: u32,
}

fn validate_new_password(password: &str) -> Result<(), String> {
    if password.chars().count() < 8 {
        return Err("密码至少需要 8 个字符".into());
    }
    if password.chars().count() > 128 || password.chars().any(char::is_control) {
        return Err("密码包含不支持的字符或长度超过限制".into());
    }
    Ok(())
}

fn password_hash(password: &str) -> Result<String, String> {
    validate_new_password(password)?;
    let salt = SaltString::generate(&mut OsRng);
    let params =
        Params::new(19 * 1024, 2, 1, Some(32)).map_err(|_| "无法初始化密码保护参数".to_string())?;
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password(password.as_bytes(), &salt)
        .map_err(|_| "无法安全处理密码".to_string())
        .map(|hash| hash.to_string())
}

fn password_matches(password: &str, stored_hash: &str) -> bool {
    let parsed = match PasswordHash::new(stored_hash) {
        Ok(parsed) => parsed,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

fn require_access(database: &Database, session: &AccessSession) -> Result<(), String> {
    if database.password_hash()?.is_some() {
        let minutes = database.auto_lock_minutes()?;
        let mut state = session
            .0
            .lock()
            .map_err(|_| "访问会话锁定失败".to_string())?;
        if !state.unlocked {
            return Err("请先输入访问密码解锁 FileHide".into());
        }
        if minutes > 0
            && state
                .last_activity
                .map(|t| t.elapsed() >= Duration::from_secs(minutes as u64 * 60))
                .unwrap_or(false)
        {
            state.unlocked = false;
            state.last_activity = None;
            return Err("会话已自动锁定，请重新输入密码".into());
        }
        state.last_activity = Some(Instant::now());
    }
    Ok(())
}

fn validate_auto_lock_minutes(minutes: u32) -> Result<(), String> {
    if [0, 5, 15, 30, 60].contains(&minutes) {
        Ok(())
    } else {
        Err("自动锁定时长无效".into())
    }
}

fn wide(path: &str) -> Vec<u16> {
    Path::new(path)
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect()
}

fn read_attributes(path: &str) -> Result<u32, String> {
    let wide_path = wide(path);
    let attributes = unsafe { GetFileAttributesW(PCWSTR(wide_path.as_ptr())) };
    if attributes == INVALID_FILE_ATTRIBUTES {
        Err(format!("无法读取文件属性：{path}"))
    } else {
        Ok(attributes)
    }
}

fn file_identity(path: &str) -> Option<String> {
    let metadata = std::fs::metadata(path).ok()?;
    let modified = metadata
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_nanos();
    Some(format!(
        "{}:{}:{}",
        metadata.len(),
        modified,
        metadata.is_dir()
    ))
}

fn find_renamed_sibling(path: &str, expected_type: &str, stored_id: &str) -> Option<String> {
    let parent = Path::new(path).parent()?;
    for entry in std::fs::read_dir(parent).ok()? {
        let candidate = match entry {
            Ok(entry) => entry.path(),
            Err(_) => continue,
        };
        if candidate.to_string_lossy().eq_ignore_ascii_case(path) {
            continue;
        }
        let attributes = match read_attributes(&candidate.to_string_lossy()) {
            Ok(value) => value,
            Err(_) => continue,
        };
        if is_reparse_point(attributes) {
            continue;
        }
        let metadata = match std::fs::metadata(&candidate) {
            Ok(value) => value,
            Err(_) => continue,
        };
        if metadata.is_dir() != (expected_type == "FOLDER") {
            continue;
        }
        if file_identity(&candidate.to_string_lossy()).as_deref() == Some(stored_id) {
            return Some(remove_extended_path_prefix(&candidate.to_string_lossy()).to_owned());
        }
    }
    None
}

fn set_attributes(path: &str, attributes: u32) -> Result<(), String> {
    let wide_path = wide(path);
    unsafe {
        SetFileAttributesW(
            PCWSTR(wide_path.as_ptr()),
            FILE_FLAGS_AND_ATTRIBUTES(attributes),
        )
    }
    .map_err(|error| error.to_string())
}

fn remove_extended_path_prefix(path: &str) -> &str {
    path.strip_prefix(r"\\?\").unwrap_or(path)
}

fn is_reparse_point(attributes: u32) -> bool {
    attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

fn recovery_stream_path(path: &str) -> String {
    format!("{path}:{RECOVERY_STREAM}")
}

fn marker_content(item_type: &str, original_attributes: u32) -> String {
    format!("{RECOVERY_MARKER_VERSION}\n{item_type}\n{original_attributes}\n")
}

fn parse_recovery_marker(content: &str) -> Option<RecoveryMarker> {
    let mut lines = content.lines();
    if lines.next()? != RECOVERY_MARKER_VERSION {
        return None;
    }
    let item_type = lines.next()?;
    if !matches!(item_type, "FILE" | "FOLDER") {
        return None;
    }
    let original_attributes = lines.next()?.parse().ok()?;
    if lines.next().is_some() {
        return None;
    }
    Some(RecoveryMarker {
        item_type: item_type.to_owned(),
        original_attributes,
    })
}

fn write_recovery_marker(
    path: &str,
    item_type: &str,
    original_attributes: u32,
) -> Result<(), String> {
    let mut stream = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(recovery_stream_path(path))
        .map_err(|_| {
            "无法写入 FileHide 恢复标记。请确认目标位于 NTFS 卷、可写且未被其他 FileHide 记录占用。"
                .to_string()
        })?;
    stream
        .write_all(marker_content(item_type, original_attributes).as_bytes())
        .map_err(|_| "无法写入 FileHide 恢复标记。".to_string())
}

fn read_recovery_marker(path: &str) -> Option<RecoveryMarker> {
    let stream = File::open(recovery_stream_path(path)).ok()?;
    let mut content = String::new();
    stream
        .take(MAX_MARKER_BYTES)
        .read_to_string(&mut content)
        .ok()?;
    parse_recovery_marker(&content)
}

fn remove_recovery_marker(path: &str) -> Result<(), String> {
    std::fs::remove_file(recovery_stream_path(path))
        .map_err(|_| "无法移除 FileHide 恢复标记。".to_string())
}

fn resolve_target_path(path: &str, expected_directory: bool) -> Result<String, String> {
    if path.trim().is_empty() {
        return Err("请选择要锁定的路径".into());
    }

    let selected_attributes = read_attributes(path)?;
    if is_reparse_point(selected_attributes) {
        return Err("不支持锁定符号链接或联接点，请选择实际文件或文件夹".into());
    }

    let canonical =
        std::fs::canonicalize(path).map_err(|_| format!("路径不存在或无法访问：{path}"))?;
    let metadata =
        std::fs::metadata(&canonical).map_err(|_| format!("路径不存在或无法访问：{path}"))?;
    if metadata.is_dir() != expected_directory {
        return Err(if expected_directory {
            "请选择文件夹".into()
        } else {
            "请选择文件".into()
        });
    }

    Ok(remove_extended_path_prefix(&canonical.to_string_lossy()).to_owned())
}

fn protection_status(current_status: i32, attributes: Result<u32, ()>) -> &'static str {
    if current_status == 0 {
        return "RESTORED";
    }

    match attributes {
        Err(()) => "MISSING",
        Ok(attributes) if attributes & 0x2 != 0 && attributes & 0x4 != 0 => "LOCKED",
        Ok(_) => "UNLOCKED_EXTERNALLY",
    }
}

fn with_protection_status(mut item: HiddenItem) -> HiddenItem {
    item.protection_status = protection_status(
        item.current_status,
        read_attributes(&item.path).map_err(|_| ()),
    )
    .into();
    item
}

fn recovery_candidate(path: &Path) -> Option<RecoveryCandidate> {
    let path = path.to_string_lossy();
    let attributes = read_attributes(&path).ok()?;
    if is_reparse_point(attributes) || attributes & 0x2 == 0 || attributes & 0x4 == 0 {
        return None;
    }
    let marker = read_recovery_marker(&path)?;
    let metadata = std::fs::metadata(&*path).ok()?;
    if metadata.is_dir() != (marker.item_type == "FOLDER") {
        return None;
    }
    Some(RecoveryCandidate {
        path: path.into_owned(),
        item_type: marker.item_type,
        original_attributes: marker.original_attributes,
    })
}

fn emit_scan_update(
    app: &AppHandle,
    scan_id: &str,
    kind: &str,
    scanned: u64,
    skipped: u64,
    candidates: Vec<RecoveryCandidate>,
) {
    let _ = app.emit(
        "recovery-scan-update",
        RecoveryScanUpdate {
            scan_id: scan_id.to_owned(),
            kind: kind.to_owned(),
            scanned,
            skipped,
            candidates,
        },
    );
}

fn hide(path: String, expected_directory: bool, database: &Database) -> Result<HiddenItem, String> {
    let path = resolve_target_path(&path, expected_directory)?;
    if read_recovery_marker(&path).is_some() {
        return Err("该项目已带有 FileHide 恢复标记，请使用“找回锁定项目”处理。".into());
    }
    if database.active_item_for_path(&path)?.is_some() {
        return Err("该项目已在隐藏列表中".into());
    }
    let original = read_attributes(&path)?;
    let id = database.insert_with_file_id(
        &path,
        if expected_directory { "FOLDER" } else { "FILE" },
        original,
        file_identity(&path).as_deref(),
    )?;
    if let Err(marker_error) = write_recovery_marker(
        &path,
        if expected_directory { "FOLDER" } else { "FILE" },
        original,
    ) {
        let _ = database.remove_active_item(id);
        return Err(marker_error);
    }
    if let Err(attribute_error) =
        set_attributes(&path, file_attributes::attributes_to_hide(original))
    {
        let _ = remove_recovery_marker(&path);
        return match database.remove_active_item(id) {
            Ok(()) => Err(format!("锁定失败，未创建历史记录：{attribute_error}")),
            Err(cleanup_error) => Err(format!(
                "锁定失败，且无法清理未完成的历史记录（{cleanup_error}）：{attribute_error}"
            )),
        };
    }
    database.get(id).map(with_protection_status)
}

#[tauri::command]
pub fn hide_file(
    path: String,
    database: State<'_, Database>,
    session: State<'_, AccessSession>,
) -> Result<HiddenItem, String> {
    require_access(&database, &session)?;
    hide(path, false, &database)
}

#[tauri::command]
pub fn hide_folder(
    path: String,
    database: State<'_, Database>,
    session: State<'_, AccessSession>,
) -> Result<HiddenItem, String> {
    require_access(&database, &session)?;
    hide(path, true, &database)
}

fn deduplicate_paths(paths: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    paths
        .into_iter()
        .filter(|p| !p.trim().is_empty() && seen.insert(p.to_lowercase()))
        .take(256)
        .collect()
}

fn hide_batch(
    paths: Vec<String>,
    expected_directory: Option<bool>,
    database: &Database,
) -> BatchResult {
    let mut result = BatchResult {
        succeeded: Vec::new(),
        failed: Vec::new(),
    };
    if paths.len() > 256 {
        result.failed.push(BatchFailure {
            path: "<batch>".into(),
            error: "单次最多处理 256 个项目".into(),
        });
        return result;
    }
    for path in deduplicate_paths(paths) {
        let expected = match expected_directory {
            Some(value) => value,
            None => match std::fs::metadata(&path) {
                Ok(metadata) => metadata.is_dir(),
                Err(error) => {
                    result.failed.push(BatchFailure {
                        path,
                        error: error.to_string(),
                    });
                    continue;
                }
            },
        };
        match hide(path.clone(), expected, database) {
            Ok(item) => result.succeeded.push(item),
            Err(error) => result.failed.push(BatchFailure { path, error }),
        }
    }
    result
}

#[tauri::command]
pub fn hide_files(
    paths: Vec<String>,
    database: State<'_, Database>,
    session: State<'_, AccessSession>,
) -> Result<BatchResult, String> {
    require_access(&database, &session)?;
    Ok(hide_batch(paths, Some(false), &database))
}

#[tauri::command]
pub fn hide_folders(
    paths: Vec<String>,
    database: State<'_, Database>,
    session: State<'_, AccessSession>,
) -> Result<BatchResult, String> {
    require_access(&database, &session)?;
    Ok(hide_batch(paths, Some(true), &database))
}

#[tauri::command]
pub fn hide_paths(
    paths: Vec<String>,
    database: State<'_, Database>,
    session: State<'_, AccessSession>,
) -> Result<BatchResult, String> {
    require_access(&database, &session)?;
    Ok(hide_batch(paths, None, &database))
}

#[tauri::command]
pub fn restore_item(
    id: i64,
    database: State<'_, Database>,
    session: State<'_, AccessSession>,
) -> Result<HiddenItem, String> {
    require_access(&database, &session)?;
    let item = database.get(id)?;
    if item.current_status == 0 {
        return Err("该项目已恢复".into());
    }
    let locked_attributes = read_attributes(&item.path)
        .map_err(|_| "路径已不存在，无法恢复；请在历史记录中保留该条目。".to_string())?;
    if protection_status(item.current_status, Ok(locked_attributes)) != "LOCKED" {
        return Err("项目未处于已锁定状态，无法恢复；请先在 FileHide 中重新锁定。".into());
    }
    set_attributes(
        &item.path,
        file_attributes::attributes_to_restore(item.original_attributes),
    )?;
    if read_recovery_marker(&item.path).is_some() {
        if let Err(marker_error) = remove_recovery_marker(&item.path) {
            let _ = set_attributes(&item.path, locked_attributes);
            return Err(format!(
                "恢复标记无法移除，已自动重新锁定项目：{marker_error}"
            ));
        }
    }
    if let Err(database_error) = database.mark_restored(id) {
        let _ = write_recovery_marker(&item.path, &item.item_type, item.original_attributes);
        return match set_attributes(&item.path, locked_attributes) {
            Ok(()) => Err(format!(
                "恢复后无法更新历史记录，已自动重新锁定项目，请重试：{database_error}"
            )),
            Err(rollback_error) => Err(format!(
                "恢复后无法更新历史记录，且自动重新锁定失败（{rollback_error}）：{database_error}"
            )),
        };
    }
    database.get(id).map(with_protection_status)
}

#[tauri::command]
pub fn start_recovery_scan(
    path: String,
    scan_id: String,
    app: AppHandle,
    scans: State<'_, RecoveryScans>,
    database: State<'_, Database>,
    session: State<'_, AccessSession>,
) -> Result<(), String> {
    require_access(&database, &session)?;
    if scan_id.trim().is_empty() {
        return Err("恢复扫描标识无效".into());
    }
    let root = resolve_target_path(&path, true)?;
    let cancelled = Arc::new(AtomicBool::new(false));
    scans
        .0
        .lock()
        .map_err(|_| "恢复扫描状态锁定失败".to_string())?
        .insert(scan_id.clone(), cancelled.clone());
    let registry = scans.0.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let mut directories = vec![PathBuf::from(&root)];
        let mut scanned = 0_u64;
        let mut skipped = 0_u64;
        let mut candidates = Vec::new();
        if let Some(candidate) = recovery_candidate(Path::new(&root)) {
            candidates.push(candidate);
        }

        while let Some(directory) = directories.pop() {
            if cancelled.load(Ordering::Relaxed) {
                emit_scan_update(&app, &scan_id, "cancelled", scanned, skipped, candidates);
                let _ = registry.lock().map(|mut entries| entries.remove(&scan_id));
                return;
            }
            let entries = match std::fs::read_dir(&directory) {
                Ok(entries) => entries,
                Err(_) => {
                    skipped += 1;
                    continue;
                }
            };
            for entry in entries.flatten() {
                if cancelled.load(Ordering::Relaxed) {
                    emit_scan_update(&app, &scan_id, "cancelled", scanned, skipped, candidates);
                    let _ = registry.lock().map(|mut entries| entries.remove(&scan_id));
                    return;
                }
                scanned += 1;
                let entry_path = entry.path();
                let entry_path_text = entry_path.to_string_lossy();
                let attributes = match read_attributes(&entry_path_text) {
                    Ok(attributes) => attributes,
                    Err(_) => {
                        skipped += 1;
                        continue;
                    }
                };
                if is_reparse_point(attributes) {
                    skipped += 1;
                    continue;
                }
                match entry.metadata() {
                    Ok(metadata) if metadata.is_dir() => directories.push(entry_path.clone()),
                    Ok(_) => {}
                    Err(_) => {
                        skipped += 1;
                        continue;
                    }
                }
                if let Some(candidate) = recovery_candidate(&entry_path) {
                    candidates.push(candidate);
                }
                if scanned % 128 == 0 {
                    emit_scan_update(&app, &scan_id, "progress", scanned, skipped, Vec::new());
                }
            }
        }
        emit_scan_update(&app, &scan_id, "completed", scanned, skipped, candidates);
        let _ = registry.lock().map(|mut entries| entries.remove(&scan_id));
    });
    Ok(())
}

#[tauri::command]
pub fn cancel_recovery_scan(
    scan_id: String,
    scans: State<'_, RecoveryScans>,
    database: State<'_, Database>,
    session: State<'_, AccessSession>,
) -> Result<(), String> {
    require_access(&database, &session)?;
    let entry = scans
        .0
        .lock()
        .map_err(|_| "恢复扫描状态锁定失败".to_string())?
        .get(&scan_id)
        .cloned()
        .ok_or_else(|| "恢复扫描不存在或已结束".to_string())?;
    entry.store(true, Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
pub fn recover_marked_item(
    path: String,
    database: State<'_, Database>,
    session: State<'_, AccessSession>,
) -> Result<HiddenItem, String> {
    require_access(&database, &session)?;
    let marker =
        read_recovery_marker(&path).ok_or_else(|| "未找到有效的 FileHide 恢复标记".to_string())?;
    let path = resolve_target_path(&path, marker.item_type == "FOLDER")?;
    let marker = read_recovery_marker(&path).ok_or_else(|| "恢复标记已失效".to_string())?;
    if database.active_item_for_path(&path)?.is_some() {
        return Err("该项目已存在于隐藏列表，请直接从隐藏项目中恢复".into());
    }
    let locked_attributes = read_attributes(&path)?;
    if protection_status(1, Ok(locked_attributes)) != "LOCKED" {
        return Err("项目未处于已锁定状态，无法找回恢复".into());
    }
    let id = database.insert(&path, &marker.item_type, marker.original_attributes)?;
    if let Err(attribute_error) = set_attributes(&path, marker.original_attributes) {
        let _ = database.remove_active_item(id);
        return Err(format!("找回恢复失败，未创建历史记录：{attribute_error}"));
    }
    if let Err(marker_error) = remove_recovery_marker(&path) {
        let _ = set_attributes(&path, locked_attributes);
        let _ = database.remove_active_item(id);
        return Err(format!("找回恢复失败，已自动重新锁定项目：{marker_error}"));
    }
    if let Err(database_error) = database.mark_restored(id) {
        let _ = write_recovery_marker(&path, &marker.item_type, marker.original_attributes);
        let _ = set_attributes(&path, locked_attributes);
        let _ = database.remove_active_item(id);
        return Err(format!(
            "找回恢复后无法更新历史记录，已自动重新锁定项目：{database_error}"
        ));
    }
    database.get(id).map(with_protection_status)
}

#[tauri::command]
pub fn password_configured(database: State<'_, Database>) -> Result<bool, String> {
    database.password_hash().map(|hash| hash.is_some())
}

#[tauri::command]
pub fn verify_password(
    password: String,
    database: State<'_, Database>,
    session: State<'_, AccessSession>,
) -> Result<bool, String> {
    let Some(stored_hash) = database.password_hash()? else {
        let mut state = session
            .0
            .lock()
            .map_err(|_| "访问会话锁定失败".to_string())?;
        state.unlocked = true;
        state.last_activity = Some(Instant::now());
        return Ok(true);
    };
    let matches = password_matches(&password, &stored_hash);
    if matches {
        let mut state = session
            .0
            .lock()
            .map_err(|_| "访问会话锁定失败".to_string())?;
        state.unlocked = true;
        state.last_activity = Some(Instant::now());
    }
    Ok(matches)
}

#[tauri::command]
pub fn set_access_password(
    current_password: Option<String>,
    new_password: String,
    database: State<'_, Database>,
    session: State<'_, AccessSession>,
) -> Result<(), String> {
    if let Some(stored_hash) = database.password_hash()? {
        let current_password = current_password.ok_or_else(|| "请输入当前密码".to_string())?;
        if !password_matches(&current_password, &stored_hash) {
            return Err("当前密码不正确".into());
        }
    }
    database.set_password_hash(&password_hash(&new_password)?)?;
    let mut state = session
        .0
        .lock()
        .map_err(|_| "访问会话锁定失败".to_string())?;
    state.unlocked = true;
    state.last_activity = Some(Instant::now());
    Ok(())
}

#[tauri::command]
pub fn clear_access_password(
    current_password: String,
    database: State<'_, Database>,
    session: State<'_, AccessSession>,
) -> Result<(), String> {
    let stored_hash = database
        .password_hash()?
        .ok_or_else(|| "尚未设置访问密码".to_string())?;
    if !password_matches(&current_password, &stored_hash) {
        return Err("当前密码不正确".into());
    }
    database.clear_password_hash()?;
    let mut state = session
        .0
        .lock()
        .map_err(|_| "访问会话锁定失败".to_string())?;
    state.unlocked = true;
    state.last_activity = Some(Instant::now());
    Ok(())
}

#[tauri::command]
pub fn get_auto_lock_minutes(
    database: State<'_, Database>,
    _session: State<'_, AccessSession>,
) -> Result<u32, String> {
    database.auto_lock_minutes()
}

#[tauri::command]
pub fn set_auto_lock_minutes(
    minutes: u32,
    database: State<'_, Database>,
    session: State<'_, AccessSession>,
) -> Result<(), String> {
    require_access(&database, &session)?;
    validate_auto_lock_minutes(minutes)?;
    database.set_auto_lock_minutes(minutes)
}

#[tauri::command]
pub fn check_session(
    database: State<'_, Database>,
    session: State<'_, AccessSession>,
) -> Result<bool, String> {
    if database.password_hash()?.is_none() {
        return Ok(true);
    }
    let minutes = database.auto_lock_minutes()?;
    let mut state = session
        .0
        .lock()
        .map_err(|_| "访问会话锁定失败".to_string())?;
    if !state.unlocked {
        return Ok(false);
    }
    if minutes > 0
        && state
            .last_activity
            .map(|t| t.elapsed() >= Duration::from_secs(minutes as u64 * 60))
            .unwrap_or(false)
    {
        state.unlocked = false;
        state.last_activity = None;
        return Ok(false);
    }
    Ok(true)
}

#[tauri::command]
pub fn lock_session(session: State<'_, AccessSession>) -> Result<(), String> {
    let mut state = session
        .0
        .lock()
        .map_err(|_| "访问会话锁定失败".to_string())?;
    state.unlocked = false;
    state.last_activity = None;
    Ok(())
}

#[tauri::command]
pub fn list_items(
    database: State<'_, Database>,
    session: State<'_, AccessSession>,
) -> Result<Vec<HiddenItem>, String> {
    require_access(&database, &session)?;
    let mut items = database.list()?;
    for item in &mut items {
        if item.current_status == 1 {
            if let Some(stored) = item.file_id.clone() {
                if file_identity(&item.path).as_deref() != Some(stored.as_str()) {
                    if let Some(new_path) =
                        find_renamed_sibling(&item.path, &item.item_type, &stored)
                    {
                        let _ = database.update_path_and_file_id(item.id, &new_path, Some(&stored));
                        item.path = new_path;
                        item.protection_status = protection_status(
                            item.current_status,
                            read_attributes(&item.path).map_err(|_| ()),
                        )
                        .into();
                        continue;
                    }
                    item.protection_status = "PATH_CHANGED".into();
                    continue;
                }
            }
        }
        item.protection_status = protection_status(
            item.current_status,
            read_attributes(&item.path).map_err(|_| ()),
        )
        .into();
    }
    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::{
        is_reparse_point, parse_recovery_marker, password_hash, password_matches,
        protection_status, read_recovery_marker, remove_extended_path_prefix,
        remove_recovery_marker, validate_auto_lock_minutes, validate_new_password,
        write_recovery_marker,
    };
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn active_item_with_hidden_and_system_attributes_is_locked() {
        assert_eq!(protection_status(1, Ok(0x2 | 0x4)), "LOCKED");
    }

    #[test]
    fn auto_lock_accepts_only_supported_values() {
        for value in [0, 5, 15, 30, 60] {
            assert!(validate_auto_lock_minutes(value).is_ok());
        }
        assert!(validate_auto_lock_minutes(1).is_err());
    }

    #[test]
    fn active_item_with_missing_path_is_marked_missing() {
        assert_eq!(protection_status(1, Err(())), "MISSING");
    }

    #[test]
    fn restored_item_stays_restored_even_when_its_path_is_missing() {
        assert_eq!(protection_status(0, Err(())), "RESTORED");
    }

    #[test]
    fn path_cleanup_removes_only_the_windows_extended_path_prefix() {
        assert_eq!(
            remove_extended_path_prefix(r"\\?\C:\\Private\\report.docx"),
            r"C:\\Private\\report.docx"
        );
        assert_eq!(
            remove_extended_path_prefix(r"C:\\Private\\report.docx"),
            r"C:\\Private\\report.docx"
        );
    }

    #[test]
    fn reparse_points_are_rejected_before_locking() {
        assert!(is_reparse_point(0x400));
        assert!(!is_reparse_point(0x20));
    }

    #[test]
    fn parses_a_versioned_recovery_marker() {
        let marker = parse_recovery_marker("FILEHIDE/1\nFOLDER\n36\n").unwrap();

        assert_eq!(marker.item_type, "FOLDER");
        assert_eq!(marker.original_attributes, 36);
    }

    #[test]
    fn rejects_a_malformed_recovery_marker() {
        assert!(parse_recovery_marker("FILEHIDE/1\nFOLDER\nnot-a-number\n").is_none());
        assert!(parse_recovery_marker("unrelated marker").is_none());
    }

    #[test]
    fn writes_and_removes_recovery_markers_on_files_and_folders() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("filehide-ads-test-{suffix}"));
        let file = root.join("private.txt");
        let folder = root.join("private-folder");
        fs::create_dir_all(&folder).unwrap();
        fs::write(&file, "test").unwrap();

        for (path, item_type) in [(&file, "FILE"), (&folder, "FOLDER")] {
            let path = path.to_string_lossy();
            write_recovery_marker(&path, item_type, 0x20).unwrap();
            let marker = read_recovery_marker(&path).unwrap();
            assert_eq!(marker.item_type, item_type);
            assert_eq!(marker.original_attributes, 0x20);
            remove_recovery_marker(&path).unwrap();
            assert!(read_recovery_marker(&path).is_none());
        }

        fs::remove_file(file).unwrap();
        fs::remove_dir(folder).unwrap();
        fs::remove_dir(root).unwrap();
    }

    #[test]
    fn password_validation_requires_a_reasonable_non_control_password() {
        assert!(validate_new_password("short").is_err());
        assert!(validate_new_password("correct horse battery staple").is_ok());
        assert!(validate_new_password("valid\u{0000}password").is_err());
    }

    #[test]
    fn argon2id_password_hash_verifies_only_the_original_password() {
        let hash = password_hash("correct horse battery staple").unwrap();

        assert!(hash.starts_with("$argon2id$"));
        assert!(password_matches("correct horse battery staple", &hash));
        assert!(!password_matches("incorrect password", &hash));
    }
}
