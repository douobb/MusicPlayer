use rusqlite::Connection;
use std::fs;
use tauri::{AppHandle, Manager};

use crate::error::AppError;

const SCHEMA_VERSION: i64 = 12;

pub fn init_db(app_handle: &AppHandle) -> Result<Connection, AppError> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Generic(format!("failed to get app data dir: {e}")))?;

    fs::create_dir_all(&app_data_dir)?;

    let db_path = app_data_dir.join("musicplayer.db");
    let conn = Connection::open(db_path)?;

    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    conn.execute_batch("PRAGMA foreign_keys=ON;")?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;

    run_migrations(&conn)?;

    Ok(conn)
}

pub fn run_migrations(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER NOT NULL
        );",
    )?;

    let current_version: i64 = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_version",
        [],
        |row| row.get(0),
    )?;

    if current_version == SCHEMA_VERSION {
        return Ok(());
    }

    if current_version != 0 {
        return Err(AppError::Generic(format!(
            "不支援開發階段 schema {current_version}；請刪除本機 musicplayer.db 後重新啟動"
        )));
    }

    let tx = conn.unchecked_transaction()?;
    tx.execute_batch(&format!(
        "
        CREATE TABLE tracks (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            file_path       TEXT NOT NULL UNIQUE,
            title           TEXT NOT NULL,
            duration_secs   REAL NOT NULL DEFAULT 0.0,
            cover_art       TEXT,
            cover_art_path  TEXT,
            file_size_bytes INTEGER NOT NULL DEFAULT 0,
            play_count      INTEGER NOT NULL DEFAULT 0,
            last_played_at  TEXT,
            source_folder_id INTEGER,
            modified_at_millis INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY (source_folder_id) REFERENCES scan_folders(id) ON DELETE SET NULL
        );

        CREATE TABLE artists (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            name            TEXT NOT NULL,
            normalized_name TEXT NOT NULL UNIQUE
        );

        CREATE TABLE track_artist_credits (
            track_id  INTEGER NOT NULL,
            artist_id INTEGER NOT NULL,
            role      TEXT NOT NULL CHECK (role IN ('performer', 'original_performer')),
            position  INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (track_id, artist_id, role),
            UNIQUE (track_id, role, position),
            FOREIGN KEY (track_id) REFERENCES tracks(id) ON DELETE CASCADE,
            FOREIGN KEY (artist_id) REFERENCES artists(id) ON DELETE CASCADE
        );
        CREATE TABLE playlists (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            name                TEXT NOT NULL,
            last_track_id       INTEGER,
            last_position_secs  REAL DEFAULT 0.0,
            sort_order          INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE playlist_tracks (
            playlist_id INTEGER NOT NULL,
            track_id    INTEGER NOT NULL,
            sort_order  INTEGER NOT NULL,
            PRIMARY KEY (playlist_id, track_id),
            FOREIGN KEY (playlist_id) REFERENCES playlists(id) ON DELETE CASCADE,
            FOREIGN KEY (track_id) REFERENCES tracks(id) ON DELETE CASCADE
        );

        CREATE TABLE scan_folders (
            id             INTEGER PRIMARY KEY AUTOINCREMENT,
            folder_path    TEXT NOT NULL UNIQUE,
            normalized_path TEXT NOT NULL UNIQUE,
            enabled        INTEGER NOT NULL DEFAULT 1,
            last_scan_at   TEXT,
            last_error     TEXT,
            last_added     INTEGER NOT NULL DEFAULT 0,
            last_updated   INTEGER NOT NULL DEFAULT 0,
            last_unchanged INTEGER NOT NULL DEFAULT 0,
            last_removed   INTEGER NOT NULL DEFAULT 0,
            last_failed    INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE tags (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            name            TEXT NOT NULL,
            normalized_name TEXT NOT NULL UNIQUE
        );

        CREATE TABLE track_tags (
            track_id INTEGER NOT NULL,
            tag_id   INTEGER NOT NULL,
            PRIMARY KEY (track_id, tag_id),
            FOREIGN KEY (track_id) REFERENCES tracks(id) ON DELETE CASCADE,
            FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
        );

        CREATE INDEX idx_tracks_title ON tracks(title);
        CREATE INDEX idx_tracks_source_folder ON tracks(source_folder_id);
        CREATE INDEX idx_tracks_play_count ON tracks(play_count DESC);
        CREATE INDEX idx_track_artist_credits_artist ON track_artist_credits(artist_id, role);
        CREATE INDEX idx_track_tags_tag_id ON track_tags(tag_id);

        INSERT INTO schema_version (version) VALUES ({SCHEMA_VERSION});
        "
    ))?;
    tx.commit()?;

    Ok(())
}
