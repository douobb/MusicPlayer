use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::DbState;
use crate::WatcherState;
use crate::error::AppError;
use crate::metadata::{reader, writer};
use crate::models::artist::ArtistRole;
use crate::models::browse::ArtistSummary;
use crate::models::library_folder::{FolderSyncResult, LibraryFolder};
use crate::models::track::{FailedFile, ImportResult, Track, TrackDetails};
use crate::scanner::folder_scanner;
use crate::storage::{folder_repo, library_repo};

#[derive(Serialize)]
pub struct BatchTrashResult {
    pub succeeded_ids: Vec<i64>,
    pub failed: Vec<BatchTrashFailure>,
}

#[derive(Serialize)]
pub struct BatchTrashFailure {
    pub id: i64,
    pub error: String,
}

const IMPORT_CHUNK_SIZE: usize = 32;

struct PreparedFile {
    track: Track,
    cover: Option<(Vec<u8>, String)>,
    modified_at_millis: i64,
}

fn file_modified_at_millis(file_path: &str) -> i64 {
    std::fs::metadata(file_path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| modified.duration_since(std::time::UNIX_EPOCH).ok())
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
        .unwrap_or(0)
}

fn read_audio_file(file_path: &str) -> Result<PreparedFile, FailedFile> {
    match reader::read_metadata(file_path) {
        Ok(track) => Ok(PreparedFile {
            cover: reader::extract_cover_art_bytes(file_path),
            modified_at_millis: file_modified_at_millis(file_path),
            track,
        }),
        Err(e) => Err(FailedFile {
            file_path: file_path.to_string(),
            error: e.to_string(),
        }),
    }
}

fn insert_chunk(
    db: &Arc<Mutex<Connection>>,
    app_data_dir: &std::path::Path,
    prepared: &[PreparedFile],
    source_folder_id: Option<i64>,
) -> Result<Vec<Track>, AppError> {
    let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
    let tx = conn.unchecked_transaction()?;
    let mut inserted = Vec::with_capacity(prepared.len());
    for p in prepared {
        let mut track = p.track.clone();
        let id = library_repo::insert_track_with_source(
            &tx,
            &track,
            source_folder_id,
            p.modified_at_millis,
        )?;
        track.id = id;
        if let Some((data, mime)) = &p.cover {
            if let Ok(cover_path) = reader::save_cover_art(app_data_dir, id, data, mime) {
                library_repo::update_cover_art_path(&tx, id, &cover_path)?;
                track.cover_art_path = Some(cover_path);
            }
        }
        track.cover_art = None;
        inserted.push(track);
    }
    tx.commit()?;
    Ok(inserted)
}

pub fn import_audio_files(
    db: &Arc<Mutex<Connection>>,
    app_data_dir: &std::path::Path,
    file_paths: &[String],
) -> ImportResult {
    import_audio_files_for_folder(db, app_data_dir, file_paths, None)
}

fn import_audio_files_for_folder(
    db: &Arc<Mutex<Connection>>,
    app_data_dir: &std::path::Path,
    file_paths: &[String],
    source_folder_id: Option<i64>,
) -> ImportResult {
    let mut tracks = Vec::new();
    let mut failed_files = Vec::new();
    for chunk in file_paths.chunks(IMPORT_CHUNK_SIZE) {
        let mut prepared = Vec::with_capacity(chunk.len());
        for path in chunk {
            match read_audio_file(path) {
                Ok(file) => prepared.push(file),
                Err(failed) => failed_files.push(failed),
            }
        }
        if prepared.is_empty() {
            continue;
        }
        match insert_chunk(db, app_data_dir, &prepared, source_folder_id) {
            Ok(inserted) => tracks.extend(inserted),
            Err(e) => failed_files.extend(prepared.into_iter().map(|file| FailedFile {
                file_path: file.track.file_path,
                error: format!("database error: {e}"),
            })),
        }
    }
    ImportResult {
        tracks,
        failed_files,
    }
}
pub fn sync_library_folder(
    db: &Arc<Mutex<Connection>>,
    app_data_dir: &std::path::Path,
    folder_id: i64,
) -> Result<(FolderSyncResult, Vec<i64>), AppError> {
    let folder = {
        let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
        folder_repo::get_folder(&conn, folder_id)?
    };
    let file_paths = match folder_scanner::scan_folder(&folder.folder_path) {
        Ok(paths) => paths,
        Err(error) => {
            let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
            folder_repo::set_scan_error(&conn, folder_id, &error.to_string())?;
            return Err(error);
        }
    };
    let fingerprints = {
        let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
        folder_repo::track_fingerprints(&conn, folder_id)?
    };
    let current_paths: std::collections::HashSet<&str> =
        file_paths.iter().map(String::as_str).collect();
    let mut changed_paths = Vec::new();
    let mut added_paths = std::collections::HashSet::new();
    let mut updated_paths = std::collections::HashSet::new();
    let mut unchanged = 0;
    for path in &file_paths {
        let modified = file_modified_at_millis(path);
        match fingerprints.get(path) {
            None => {
                added_paths.insert(path.clone());
                changed_paths.push(path.clone());
            }
            Some((_, previous)) if modified > 0 && modified == *previous => unchanged += 1,
            Some(_) => {
                updated_paths.insert(path.clone());
                changed_paths.push(path.clone());
            }
        }
    }
    let imported = import_audio_files_for_folder(db, app_data_dir, &changed_paths, Some(folder_id));
    let failed_paths: std::collections::HashSet<&str> = imported
        .failed_files
        .iter()
        .map(|failed| failed.file_path.as_str())
        .collect();
    let added = added_paths
        .iter()
        .filter(|path| !failed_paths.contains(path.as_str()))
        .count();
    let updated = updated_paths
        .iter()
        .filter(|path| !failed_paths.contains(path.as_str()))
        .count();

    let missing: Vec<(String, i64)> = fingerprints
        .into_iter()
        .filter(|(path, _)| !current_paths.contains(path.as_str()))
        .map(|(path, (id, _))| (path, id))
        .collect();
    let mut removed_ids = Vec::new();
    {
        let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
        for (path, id) in &missing {
            if let Some(cover_path) = library_repo::delete_track_by_path(&conn, path)? {
                reader::remove_cover_art_file(&cover_path);
            }
            removed_ids.push(*id);
        }
    }
    let result = FolderSyncResult {
        folder_id,
        added,
        updated,
        unchanged,
        removed: removed_ids.len(),
        failed_files: imported.failed_files,
    };
    {
        let conn = db.lock().map_err(|_| AppError::LockPoisoned)?;
        let error = (!result.failed_files.is_empty())
            .then(|| format!("{} 個檔案同步失敗", result.failed_files.len()));
        folder_repo::update_sync_result(&conn, &result, error.as_deref())?;
    }
    Ok((result, removed_ids))
}

#[tauri::command]
pub fn import_paths(
    paths: Vec<String>,
    db: State<DbState>,
    app_handle: AppHandle,
) -> Result<ImportResult, AppError> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Generic(format!("failed to get app data dir: {e}")))?;

    let mut audio_files: Vec<String> = Vec::new();
    for p in &paths {
        let path = std::path::Path::new(p);
        if path.is_dir() {
            if let Ok(files) = folder_scanner::scan_folder(p) {
                audio_files.extend(files);
            }
        } else if folder_scanner::is_supported_audio_file(p) {
            audio_files.push(p.clone());
        }
    }

    Ok(import_audio_files(&db.0, &app_data_dir, &audio_files))
}

#[tauri::command]
pub fn get_all_tracks(db: State<DbState>) -> Result<Vec<Track>, AppError> {
    let conn = db.0.lock().map_err(|_| AppError::LockPoisoned)?;
    library_repo::get_all_tracks(&conn)
}

#[tauri::command]
pub fn get_track_cover(id: i64, db: State<DbState>) -> Result<Option<String>, AppError> {
    let conn = db.0.lock().map_err(|_| AppError::LockPoisoned)?;
    let cover_path = library_repo::get_track_cover_path(&conn, id)?;

    match cover_path {
        Some(path) => Ok(reader::read_cover_art_from_file(&path)),
        None => Ok(None),
    }
}

#[tauri::command]
pub fn search_tracks(query: String, db: State<DbState>) -> Result<Vec<Track>, AppError> {
    let conn = db.0.lock().map_err(|_| AppError::LockPoisoned)?;
    library_repo::search_tracks(&conn, &query)
}

#[tauri::command]
pub fn remove_track(id: i64, db: State<DbState>) -> Result<(), AppError> {
    let conn = db.0.lock().map_err(|_| AppError::LockPoisoned)?;
    if let Ok(Some(cover_path)) = library_repo::get_track_cover_path(&conn, id) {
        reader::remove_cover_art_file(&cover_path);
    }
    library_repo::delete_track(&conn, id)
}

#[tauri::command]
pub fn trash_track(id: i64, db: State<DbState>) -> Result<(), AppError> {
    let conn = db.0.lock().map_err(|_| AppError::LockPoisoned)?;
    let track = library_repo::get_track_by_id(&conn, id)?
        .ok_or_else(|| AppError::Generic(format!("Track {id} not found")))?;
    if let Ok(Some(cover_path)) = library_repo::get_track_cover_path(&conn, id) {
        reader::remove_cover_art_file(&cover_path);
    }
    trash::delete(&track.file_path)
        .map_err(|e| AppError::Generic(format!("Failed to trash file: {e}")))?;
    library_repo::delete_track(&conn, id)
}

#[tauri::command]
pub fn trash_tracks(ids: Vec<i64>, db: State<DbState>) -> Result<BatchTrashResult, AppError> {
    let conn = db.0.lock().map_err(|_| AppError::LockPoisoned)?;
    let tracks = library_repo::get_tracks_by_ids(&conn, &ids)?;

    let mut succeeded_ids = Vec::new();
    let mut failed = Vec::new();

    for track in &tracks {
        match trash::delete(&track.file_path) {
            Ok(()) => {
                if let Some(ref cover_path) = track.cover_art_path {
                    reader::remove_cover_art_file(cover_path);
                }
                succeeded_ids.push(track.id);
            }
            Err(e) => {
                failed.push(BatchTrashFailure {
                    id: track.id,
                    error: format!("Failed to trash file: {e}"),
                });
            }
        }
    }

    // Mark any IDs not found in DB as failed
    let found_ids: std::collections::HashSet<i64> = tracks.iter().map(|t| t.id).collect();
    for &id in &ids {
        if !found_ids.contains(&id) {
            failed.push(BatchTrashFailure {
                id,
                error: format!("Track {id} not found"),
            });
        }
    }

    if !succeeded_ids.is_empty() {
        library_repo::delete_tracks(&conn, &succeeded_ids)?;
    }

    Ok(BatchTrashResult {
        succeeded_ids,
        failed,
    })
}

#[tauri::command]
pub fn remove_tracks(ids: Vec<i64>, db: State<DbState>) -> Result<BatchTrashResult, AppError> {
    let conn = db.0.lock().map_err(|_| AppError::LockPoisoned)?;
    let tracks = library_repo::get_tracks_by_ids(&conn, &ids)?;

    let found_ids: std::collections::HashSet<i64> = tracks.iter().map(|t| t.id).collect();

    let mut succeeded_ids = Vec::new();
    let mut failed = Vec::new();

    for &id in &ids {
        if found_ids.contains(&id) {
            succeeded_ids.push(id);
        } else {
            failed.push(BatchTrashFailure {
                id,
                error: format!("Track {id} not found"),
            });
        }
    }

    // Batch clean up cover art files
    for track in &tracks {
        if let Some(ref cover_path) = track.cover_art_path {
            reader::remove_cover_art_file(cover_path);
        }
    }

    if !succeeded_ids.is_empty() {
        library_repo::delete_tracks(&conn, &succeeded_ids)?;
    }

    Ok(BatchTrashResult {
        succeeded_ids,
        failed,
    })
}

#[tauri::command]
pub fn get_track_details(id: i64, db: State<DbState>) -> Result<TrackDetails, AppError> {
    let conn = db.0.lock().map_err(|_| AppError::LockPoisoned)?;
    let track = library_repo::get_track_by_id(&conn, id)?
        .ok_or_else(|| AppError::Generic(format!("Track {id} not found")))?;
    reader::read_track_details(&track.file_path, &track)
}

#[tauri::command]
pub fn update_track_metadata(
    id: i64,
    title: Option<String>,
    performers: Option<Vec<String>>,
    original_performers: Option<Vec<String>>,
    db: State<DbState>,
) -> Result<Track, AppError> {
    let conn = db.0.lock().map_err(|_| AppError::LockPoisoned)?;
    let track = library_repo::get_track_by_id(&conn, id)?
        .ok_or_else(|| AppError::Generic(format!("Track {id} not found")))?;
    let new_title = title.unwrap_or(track.title.clone());
    let new_performers =
        performers.unwrap_or_else(|| track.performers.iter().map(|a| a.name.clone()).collect());
    let new_originals = original_performers.unwrap_or_else(|| {
        track
            .original_performers
            .iter()
            .map(|a| a.name.clone())
            .collect()
    });
    writer::write_metadata(
        &track.file_path,
        Some(&new_title),
        Some(&new_performers),
        Some(&new_originals),
    )?;
    library_repo::update_track_metadata(&conn, id, &new_title, &new_performers, &new_originals).map_err(|e| {
        AppError::Generic(format!("File tags were updated, but syncing the library database failed (rescan the folder to recover): {e}"))
    })?;
    library_repo::get_track_by_id(&conn, id)?
        .ok_or_else(|| AppError::Generic(format!("Track {id} not found after update")))
}

#[tauri::command]
pub fn create_artist(name: String, db: State<DbState>) -> Result<i64, AppError> {
    let conn = db.0.lock().map_err(|_| AppError::LockPoisoned)?;
    library_repo::create_artist(&conn, &name)
}

#[tauri::command]
pub fn rename_artist(id: i64, name: String, db: State<DbState>) -> Result<(), AppError> {
    let conn = db.0.lock().map_err(|_| AppError::LockPoisoned)?;
    library_repo::rename_artist(&conn, id, &name)
}

#[tauri::command]
pub fn merge_artists(source_id: i64, target_id: i64, db: State<DbState>) -> Result<(), AppError> {
    let conn = db.0.lock().map_err(|_| AppError::LockPoisoned)?;
    library_repo::merge_artists(&conn, source_id, target_id)
}

#[tauri::command]
pub fn delete_unused_artists(db: State<DbState>) -> Result<usize, AppError> {
    let conn = db.0.lock().map_err(|_| AppError::LockPoisoned)?;
    library_repo::delete_unused_artists(&conn)
}

#[tauri::command]
pub fn get_all_artists(db: State<DbState>) -> Result<Vec<ArtistSummary>, AppError> {
    let conn = db.0.lock().map_err(|_| AppError::LockPoisoned)?;
    library_repo::get_all_artists(&conn)
}

#[tauri::command]
pub fn get_tracks_by_artist(
    artist_id: i64,
    role: Option<ArtistRole>,
    db: State<DbState>,
) -> Result<Vec<Track>, AppError> {
    let conn = db.0.lock().map_err(|_| AppError::LockPoisoned)?;
    library_repo::get_tracks_by_artist(&conn, artist_id, role)
}

#[tauri::command]
pub fn increment_play_count(track_id: i64, db: State<DbState>) -> Result<(), AppError> {
    let conn = db.0.lock().map_err(|_| AppError::LockPoisoned)?;
    library_repo::increment_play_count(&conn, track_id)
}

#[tauri::command]
pub fn get_most_played_tracks(
    limit: Option<i64>,
    db: State<DbState>,
) -> Result<Vec<Track>, AppError> {
    let conn = db.0.lock().map_err(|_| AppError::LockPoisoned)?;
    library_repo::get_most_played_tracks(&conn, limit.unwrap_or(50))
}

fn emit_library_sync(app_handle: &AppHandle, removed_ids: Vec<i64>) {
    let _ = app_handle.emit("library-changed", ());
    if !removed_ids.is_empty() {
        let _ = app_handle.emit("tracks-removed", removed_ids);
    }
}

#[tauri::command]
pub fn get_library_folders(db: State<DbState>) -> Result<Vec<LibraryFolder>, AppError> {
    let conn = db.0.lock().map_err(|_| AppError::LockPoisoned)?;
    folder_repo::get_folders(&conn)
}

#[tauri::command]
pub fn add_library_folder(
    folder_path: String,
    db: State<DbState>,
    watcher_state: State<WatcherState>,
    app_handle: AppHandle,
) -> Result<FolderSyncResult, AppError> {
    let folder_id = {
        let conn = db.0.lock().map_err(|_| AppError::LockPoisoned)?;
        folder_repo::add_folder(&conn, &folder_path)?
    };
    let folder = {
        let conn = db.0.lock().map_err(|_| AppError::LockPoisoned)?;
        folder_repo::get_folder(&conn, folder_id)?
    };
    if let Some(watcher) = watcher_state
        .0
        .lock()
        .map_err(|_| AppError::LockPoisoned)?
        .as_ref()
    {
        watcher.watch(&folder.folder_path)?;
    }
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Generic(format!("failed to get app data dir: {e}")))?;
    let (result, removed_ids) = sync_library_folder(&db.0, &app_data_dir, folder_id)?;
    emit_library_sync(&app_handle, removed_ids);
    Ok(result)
}

#[tauri::command]
pub fn rescan_library_folder(
    folder_id: i64,
    db: State<DbState>,
    app_handle: AppHandle,
) -> Result<FolderSyncResult, AppError> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Generic(format!("failed to get app data dir: {e}")))?;
    let (result, removed_ids) = sync_library_folder(&db.0, &app_data_dir, folder_id)?;
    emit_library_sync(&app_handle, removed_ids);
    Ok(result)
}

#[tauri::command]
pub fn rescan_all_library_folders(
    db: State<DbState>,
    app_handle: AppHandle,
) -> Result<Vec<FolderSyncResult>, AppError> {
    let folders = {
        let conn = db.0.lock().map_err(|_| AppError::LockPoisoned)?;
        folder_repo::get_folders(&conn)?
    };
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Generic(format!("failed to get app data dir: {e}")))?;
    let mut results = Vec::new();
    let mut removed_ids = Vec::new();
    for folder in folders.into_iter().filter(|folder| folder.enabled) {
        match sync_library_folder(&db.0, &app_data_dir, folder.id) {
            Ok((result, removed)) => {
                results.push(result);
                removed_ids.extend(removed);
            }
            Err(error) => eprintln!(
                "[musicplayer] failed to synchronize {}: {error}",
                folder.folder_path
            ),
        }
    }
    emit_library_sync(&app_handle, removed_ids);
    Ok(results)
}

#[tauri::command]
pub fn set_library_folder_watching(
    folder_id: i64,
    enabled: bool,
    db: State<DbState>,
    watcher_state: State<WatcherState>,
) -> Result<(), AppError> {
    let folder = {
        let conn = db.0.lock().map_err(|_| AppError::LockPoisoned)?;
        folder_repo::get_folder(&conn, folder_id)?
    };
    if enabled && !std::path::Path::new(&folder.folder_path).is_dir() {
        return Err(AppError::Generic(format!(
            "資料夾目前無法存取：{}",
            folder.folder_path
        )));
    }
    if let Some(watcher) = watcher_state
        .0
        .lock()
        .map_err(|_| AppError::LockPoisoned)?
        .as_ref()
    {
        if enabled {
            watcher.watch(&folder.folder_path)?;
        } else {
            watcher.unwatch(&folder.folder_path)?;
        }
    }
    let conn = db.0.lock().map_err(|_| AppError::LockPoisoned)?;
    folder_repo::set_enabled(&conn, folder_id, enabled)?;
    Ok(())
}

#[tauri::command]
pub fn remove_library_folder(
    folder_id: i64,
    remove_tracks: bool,
    db: State<DbState>,
    watcher_state: State<WatcherState>,
    app_handle: AppHandle,
) -> Result<usize, AppError> {
    let folder = {
        let conn = db.0.lock().map_err(|_| AppError::LockPoisoned)?;
        folder_repo::get_folder(&conn, folder_id)?
    };
    if folder.enabled {
        if let Some(watcher) = watcher_state
            .0
            .lock()
            .map_err(|_| AppError::LockPoisoned)?
            .as_ref()
        {
            watcher.unwatch(&folder.folder_path)?;
        }
    }
    let (cover_paths, removed_ids) = {
        let conn = db.0.lock().map_err(|_| AppError::LockPoisoned)?;
        let cover_paths = if remove_tracks {
            let mut stmt = conn.prepare(
                "SELECT cover_art_path FROM tracks WHERE source_folder_id=?1 AND cover_art_path IS NOT NULL",
            )?;
            stmt.query_map(rusqlite::params![folder_id], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()?
        } else {
            Vec::new()
        };
        let ids = folder_repo::remove_folder(&conn, folder_id, remove_tracks)?;
        (cover_paths, ids)
    };
    for path in cover_paths {
        reader::remove_cover_art_file(&path);
    }
    let count = removed_ids.len();
    emit_library_sync(&app_handle, removed_ids);
    Ok(count)
}

#[tauri::command]
pub fn open_library_folder(folder_id: i64, db: State<DbState>) -> Result<(), AppError> {
    let folder = {
        let conn = db.0.lock().map_err(|_| AppError::LockPoisoned)?;
        folder_repo::get_folder(&conn, folder_id)?
    };
    #[cfg(target_os = "windows")]
    let mut command = std::process::Command::new("explorer.exe");
    #[cfg(target_os = "macos")]
    let mut command = std::process::Command::new("open");
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = std::process::Command::new("xdg-open");
    command.arg(&folder.folder_path).spawn()?;
    Ok(())
}
