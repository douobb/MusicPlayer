use std::collections::HashMap;
use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};

use crate::error::AppError;
use crate::models::library_folder::{FolderSyncResult, LibraryFolder};

fn normalize_existing_path(folder_path: &str) -> Result<(String, String), AppError> {
    let canonical = Path::new(folder_path).canonicalize()?;
    if !canonical.is_dir() {
        return Err(AppError::Generic(format!("不是資料夾：{folder_path}")));
    }
    let raw = canonical.to_string_lossy();
    let display = if let Some(path) = raw.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{path}")
    } else {
        raw.strip_prefix(r"\\?\").unwrap_or(&raw).to_string()
    };
    let mut normalized = display.replace('\\', "/");
    while normalized.len() > 3 && normalized.ends_with('/') {
        normalized.pop();
    }
    if cfg!(windows) {
        normalized = normalized.to_lowercase();
    }
    Ok((display, normalized))
}

fn paths_overlap(left: &str, right: &str) -> bool {
    left == right
        || left.starts_with(&format!("{right}/"))
        || right.starts_with(&format!("{left}/"))
}

pub fn add_folder(conn: &Connection, folder_path: &str) -> Result<i64, AppError> {
    let (display, normalized) = normalize_existing_path(folder_path)?;
    if let Some(id) = conn
        .query_row(
            "SELECT id FROM scan_folders WHERE normalized_path=?1",
            params![normalized],
            |row| row.get(0),
        )
        .optional()?
    {
        return Ok(id);
    }

    let mut stmt = conn.prepare("SELECT folder_path, normalized_path FROM scan_folders")?;
    let existing = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    if let Some((path, _)) = existing
        .iter()
        .find(|(_, candidate)| paths_overlap(&normalized, candidate))
    {
        return Err(AppError::Generic(format!(
            "資料夾與既有媒體庫路徑重疊：{path}"
        )));
    }

    conn.execute(
        "INSERT INTO scan_folders(folder_path,normalized_path,enabled) VALUES(?1,?2,1)",
        params![display, normalized],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_folder(conn: &Connection, id: i64) -> Result<LibraryFolder, AppError> {
    folder_query(conn, "WHERE sf.id=?1", &[&id])?
        .into_iter()
        .next()
        .ok_or_else(|| AppError::Generic(format!("媒體庫資料夾 {id} 不存在")))
}

pub fn get_folders(conn: &Connection) -> Result<Vec<LibraryFolder>, AppError> {
    folder_query(conn, "", &[])
}

fn folder_query(
    conn: &Connection,
    condition: &str,
    values: &[&dyn rusqlite::ToSql],
) -> Result<Vec<LibraryFolder>, AppError> {
    let sql = format!(
        "SELECT sf.id,sf.folder_path,sf.enabled,COUNT(t.id),sf.last_scan_at,sf.last_error,sf.last_added,sf.last_updated,sf.last_unchanged,sf.last_removed,sf.last_failed FROM scan_folders sf LEFT JOIN tracks t ON t.source_folder_id=sf.id {condition} GROUP BY sf.id ORDER BY sf.normalized_path"
    );
    let mut stmt = conn.prepare(&sql)?;
    stmt.query_map(values, |row| {
        Ok(LibraryFolder {
            id: row.get(0)?,
            folder_path: row.get(1)?,
            enabled: row.get(2)?,
            track_count: row.get(3)?,
            last_scan_at: row.get(4)?,
            last_error: row.get(5)?,
            last_added: row.get(6)?,
            last_updated: row.get(7)?,
            last_unchanged: row.get(8)?,
            last_removed: row.get(9)?,
            last_failed: row.get(10)?,
        })
    })?
    .collect::<Result<Vec<_>, _>>()
    .map_err(Into::into)
}

pub fn get_enabled_paths(conn: &Connection) -> Result<Vec<String>, AppError> {
    let mut stmt = conn.prepare("SELECT folder_path FROM scan_folders WHERE enabled=1")?;
    stmt.query_map([], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn set_enabled(conn: &Connection, id: i64, enabled: bool) -> Result<String, AppError> {
    let path: String = conn
        .query_row(
            "SELECT folder_path FROM scan_folders WHERE id=?1",
            params![id],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| AppError::Generic(format!("媒體庫資料夾 {id} 不存在")))?;
    conn.execute(
        "UPDATE scan_folders SET enabled=?1 WHERE id=?2",
        params![enabled, id],
    )?;
    Ok(path)
}

pub fn track_fingerprints(
    conn: &Connection,
    folder_id: i64,
) -> Result<HashMap<String, (i64, i64)>, AppError> {
    let mut stmt = conn
        .prepare("SELECT file_path,id,modified_at_millis FROM tracks WHERE source_folder_id=?1")?;
    let rows = stmt.query_map(params![folder_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            (row.get::<_, i64>(1)?, row.get::<_, i64>(2)?),
        ))
    })?;
    Ok(rows.collect::<Result<HashMap<_, _>, _>>()?)
}

pub fn update_sync_result(
    conn: &Connection,
    result: &FolderSyncResult,
    error: Option<&str>,
) -> Result<(), AppError> {
    #[allow(clippy::cast_possible_wrap)]
    conn.execute(
        "UPDATE scan_folders SET last_scan_at=CURRENT_TIMESTAMP,last_error=?1,last_added=?2,last_updated=?3,last_unchanged=?4,last_removed=?5,last_failed=?6 WHERE id=?7",
        params![error, result.added as i64, result.updated as i64, result.unchanged as i64, result.removed as i64, result.failed_files.len() as i64, result.folder_id],
    )?;
    Ok(())
}

pub fn set_scan_error(conn: &Connection, folder_id: i64, error: &str) -> Result<(), AppError> {
    conn.execute(
        "UPDATE scan_folders SET last_scan_at=CURRENT_TIMESTAMP,last_error=?1 WHERE id=?2",
        params![error, folder_id],
    )?;
    Ok(())
}

pub fn remove_folder(
    conn: &Connection,
    folder_id: i64,
    remove_tracks: bool,
) -> Result<Vec<i64>, AppError> {
    let tx = conn.unchecked_transaction()?;
    let ids = {
        let mut stmt = tx.prepare("SELECT id FROM tracks WHERE source_folder_id=?1")?;
        stmt.query_map(params![folder_id], |row| row.get(0))?
            .collect::<Result<Vec<i64>, _>>()?
    };
    if remove_tracks {
        tx.execute(
            "DELETE FROM tracks WHERE source_folder_id=?1",
            params![folder_id],
        )?;
    } else {
        tx.execute(
            "UPDATE tracks SET source_folder_id=NULL WHERE source_folder_id=?1",
            params![folder_id],
        )?;
    }
    let changed = tx.execute("DELETE FROM scan_folders WHERE id=?1", params![folder_id])?;
    if changed == 0 {
        return Err(AppError::Generic(format!(
            "媒體庫資料夾 {folder_id} 不存在"
        )));
    }
    tx.commit()?;
    Ok(if remove_tracks { ids } else { Vec::new() })
}

pub fn delete_tracks_by_ids(conn: &Connection, ids: &[i64]) -> Result<(), AppError> {
    if ids.is_empty() {
        return Ok(());
    }
    for chunk in ids.chunks(500) {
        let placeholders = vec!["?"; chunk.len()].join(",");
        let sql = format!("DELETE FROM tracks WHERE id IN ({placeholders})");
        conn.execute(&sql, rusqlite::params_from_iter(chunk))?;
    }
    Ok(())
}

pub fn find_folder_for_file(conn: &Connection, file_path: &str) -> Result<Option<i64>, AppError> {
    let mut normalized_file = file_path.replace('\\', "/");
    if cfg!(windows) {
        normalized_file = normalized_file.to_lowercase();
    }
    let mut best: Option<(usize, i64)> = None;
    let mut stmt = conn.prepare("SELECT id,normalized_path FROM scan_folders WHERE enabled=1")?;
    for row in stmt.query_map([], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    })? {
        let (id, folder) = row?;
        if normalized_file == folder || normalized_file.starts_with(&format!("{folder}/")) {
            let length = folder.len();
            if best.is_none_or(|(best_length, _)| length > best_length) {
                best = Some((length, id));
            }
        }
    }
    Ok(best.map(|(_, id)| id))
}
