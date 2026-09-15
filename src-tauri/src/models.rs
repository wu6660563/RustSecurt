use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct HiddenItem {
    pub id: i64,
    pub path: String,
    pub item_type: String,
    pub original_attributes: u32,
    pub current_status: i32,
    pub protection_status: String,
    pub create_time: String,
    pub update_time: String,
    pub file_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BatchFailure {
    pub path: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BatchResult {
    pub succeeded: Vec<HiddenItem>,
    pub failed: Vec<BatchFailure>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PreviewItem {
    pub path: String,
    pub item_type: String,
    pub risks: Vec<String>,
    pub can_lock: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct HealthSummary {
    pub checked: u32,
    pub healthy: u32,
    pub changed: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeleteHistoryResult {
    pub succeeded: Vec<i64>,
    pub failed: Vec<BatchFailure>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StorageInfo {
    pub mode: String,
    pub path: String,
    pub portable_marker: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecoveryCandidate {
    pub path: String,
    pub item_type: String,
    pub original_attributes: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecoveryScanUpdate {
    pub scan_id: String,
    pub kind: String,
    pub scanned: u64,
    pub skipped: u64,
    pub candidates: Vec<RecoveryCandidate>,
}
