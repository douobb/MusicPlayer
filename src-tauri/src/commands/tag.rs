use tauri::State;

use crate::DbState;
use crate::error::AppError;
use crate::models::tag::{TagAssignment, TagStatistics, TagSummary};
use crate::models::track::Track;
use crate::storage::tag_repo;

#[tauri::command]
pub fn create_tag(name: String, db: State<DbState>) -> Result<TagSummary, AppError> {
    let conn = db.0.lock().map_err(|_| AppError::LockPoisoned)?;
    tag_repo::create_tag(&conn, &name)
}

#[tauri::command]
pub fn rename_tag(id: i64, name: String, db: State<DbState>) -> Result<TagSummary, AppError> {
    let conn = db.0.lock().map_err(|_| AppError::LockPoisoned)?;
    tag_repo::rename_tag(&conn, id, &name)
}

#[tauri::command]
pub fn delete_tag(id: i64, db: State<DbState>) -> Result<(), AppError> {
    let conn = db.0.lock().map_err(|_| AppError::LockPoisoned)?;
    tag_repo::delete_tag(&conn, id)
}

#[tauri::command]
pub fn delete_empty_tags(db: State<DbState>) -> Result<usize, AppError> {
    let conn = db.0.lock().map_err(|_| AppError::LockPoisoned)?;
    tag_repo::delete_empty_tags(&conn)
}
#[tauri::command]
pub fn merge_tags(
    source_tag_id: i64,
    target_tag_id: i64,
    db: State<DbState>,
) -> Result<TagSummary, AppError> {
    let mut conn = db.0.lock().map_err(|_| AppError::LockPoisoned)?;
    tag_repo::merge_tags(&mut conn, source_tag_id, target_tag_id)
}

#[tauri::command]
pub fn get_all_tags(db: State<DbState>) -> Result<Vec<TagSummary>, AppError> {
    let conn = db.0.lock().map_err(|_| AppError::LockPoisoned)?;
    tag_repo::get_all_tags(&conn)
}

#[tauri::command]
pub fn get_tag_statistics(db: State<DbState>) -> Result<TagStatistics, AppError> {
    let conn = db.0.lock().map_err(|_| AppError::LockPoisoned)?;
    tag_repo::get_tag_statistics(&conn)
}

#[tauri::command]
pub fn get_tags_for_track(track_id: i64, db: State<DbState>) -> Result<Vec<TagSummary>, AppError> {
    let conn = db.0.lock().map_err(|_| AppError::LockPoisoned)?;
    tag_repo::get_tags_for_track(&conn, track_id)
}

#[tauri::command]
pub fn get_tag_assignments_for_tracks(
    track_ids: Vec<i64>,
    db: State<DbState>,
) -> Result<Vec<TagAssignment>, AppError> {
    let conn = db.0.lock().map_err(|_| AppError::LockPoisoned)?;
    tag_repo::get_tag_assignments_for_tracks(&conn, &track_ids)
}
#[tauri::command]
pub fn add_tags_to_tracks(
    track_ids: Vec<i64>,
    tag_ids: Vec<i64>,
    db: State<DbState>,
) -> Result<(), AppError> {
    let mut conn = db.0.lock().map_err(|_| AppError::LockPoisoned)?;
    tag_repo::add_tags_to_tracks(&mut conn, &track_ids, &tag_ids)
}

#[tauri::command]
pub fn remove_tags_from_tracks(
    track_ids: Vec<i64>,
    tag_ids: Vec<i64>,
    db: State<DbState>,
) -> Result<(), AppError> {
    let mut conn = db.0.lock().map_err(|_| AppError::LockPoisoned)?;
    tag_repo::remove_tags_from_tracks(&mut conn, &track_ids, &tag_ids)
}

#[tauri::command]
pub fn get_tracks_by_tag(tag_id: i64, db: State<DbState>) -> Result<Vec<Track>, AppError> {
    let conn = db.0.lock().map_err(|_| AppError::LockPoisoned)?;
    tag_repo::get_tracks_by_tag(&conn, tag_id)
}
