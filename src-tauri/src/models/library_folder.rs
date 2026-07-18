use serde::Serialize;

use super::track::FailedFile;

#[derive(Debug, Clone, Serialize)]
pub struct LibraryFolder {
    pub id: i64,
    pub folder_path: String,
    pub enabled: bool,
    pub track_count: i64,
    pub last_scan_at: Option<String>,
    pub last_error: Option<String>,
    pub last_added: i64,
    pub last_updated: i64,
    pub last_unchanged: i64,
    pub last_removed: i64,
    pub last_failed: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct FolderSyncResult {
    pub folder_id: i64,
    pub added: usize,
    pub updated: usize,
    pub unchanged: usize,
    pub removed: usize,
    pub failed_files: Vec<FailedFile>,
}
