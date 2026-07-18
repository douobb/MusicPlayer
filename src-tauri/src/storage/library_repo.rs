use std::collections::HashMap;

use rusqlite::{Connection, OptionalExtension, params};

use crate::error::AppError;
use crate::models::artist::{ArtistCredit, ArtistRole};
use crate::models::browse::ArtistSummary;
use crate::models::track::Track;

pub(crate) const TRACK_COLUMNS: &str = "id, file_path, title, duration_secs, cover_art_path, file_size_bytes, play_count, last_played_at";

pub fn row_to_track(row: &rusqlite::Row) -> rusqlite::Result<Track> {
    Ok(Track {
        id: row.get(0)?,
        file_path: row.get(1)?,
        title: row.get(2)?,
        performers: Vec::new(),
        original_performers: Vec::new(),
        duration_secs: row.get(3)?,
        cover_art: None,
        cover_art_path: row.get(4)?,
        file_size_bytes: row.get(5)?,
        play_count: row.get(6)?,
        last_played_at: row.get(7)?,
    })
}

fn normalized_artist_name(name: &str) -> Result<(String, String), AppError> {
    let display = name.trim();
    if display.is_empty() {
        return Err(AppError::Generic("Artist 名稱不可為空白".to_string()));
    }
    Ok((display.to_string(), display.to_lowercase()))
}

pub fn create_artist(conn: &Connection, name: &str) -> Result<i64, AppError> {
    let (display, normalized) = normalized_artist_name(name)?;
    conn.execute(
        "INSERT INTO artists (name, normalized_name) VALUES (?1, ?2)",
        params![display, normalized],
    )?;
    Ok(conn.last_insert_rowid())
}

fn ensure_artist(conn: &Connection, name: &str) -> Result<i64, AppError> {
    let (display, normalized) = normalized_artist_name(name)?;
    if let Some(id) = conn
        .query_row(
            "SELECT id FROM artists WHERE normalized_name = ?1",
            params![normalized],
            |row| row.get(0),
        )
        .optional()?
    {
        return Ok(id);
    }
    conn.execute(
        "INSERT INTO artists (name, normalized_name) VALUES (?1, ?2)",
        params![display, normalized],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn rename_artist(conn: &Connection, id: i64, name: &str) -> Result<(), AppError> {
    let (display, normalized) = normalized_artist_name(name)?;
    let changed = conn.execute(
        "UPDATE artists SET name = ?1, normalized_name = ?2 WHERE id = ?3",
        params![display, normalized, id],
    )?;
    if changed == 0 {
        return Err(AppError::Generic(format!("Artist {id} 不存在")));
    }
    Ok(())
}

pub fn merge_artists(conn: &Connection, source_id: i64, target_id: i64) -> Result<(), AppError> {
    if source_id == target_id {
        return Err(AppError::Generic("來源與目標 Artist 不可相同".to_string()));
    }
    let tx = conn.unchecked_transaction()?;
    let source_exists: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM artists WHERE id=?1)",
        params![source_id],
        |r| r.get(0),
    )?;
    let target_exists: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM artists WHERE id=?1)",
        params![target_id],
        |r| r.get(0),
    )?;
    if !source_exists || !target_exists {
        return Err(AppError::Generic("來源或目標 Artist 不存在".to_string()));
    }
    tx.execute("DELETE FROM track_artist_credits WHERE artist_id=?1 AND EXISTS (SELECT 1 FROM track_artist_credits x WHERE x.track_id=track_artist_credits.track_id AND x.role=track_artist_credits.role AND x.artist_id=?2)", params![source_id, target_id])?;
    tx.execute(
        "UPDATE track_artist_credits SET artist_id=?1 WHERE artist_id=?2",
        params![target_id, source_id],
    )?;
    tx.execute("DELETE FROM artists WHERE id=?1", params![source_id])?;
    tx.commit()?;
    Ok(())
}

pub fn delete_unused_artists(conn: &Connection) -> Result<usize, AppError> {
    conn.execute("DELETE FROM artists WHERE NOT EXISTS (SELECT 1 FROM track_artist_credits WHERE artist_id=artists.id)", []).map_err(Into::into)
}

fn replace_role_credits(
    conn: &Connection,
    track_id: i64,
    role: ArtistRole,
    names: &[String],
) -> Result<(), AppError> {
    conn.execute(
        "DELETE FROM track_artist_credits WHERE track_id=?1 AND role=?2",
        params![track_id, role.as_str()],
    )?;
    let mut seen = std::collections::HashSet::new();
    for name in names {
        let (_, normalized) = normalized_artist_name(name)?;
        if !seen.insert(normalized) {
            continue;
        }
        let artist_id = ensure_artist(conn, name)?;
        #[allow(clippy::cast_possible_wrap)]
        let position = seen.len() as i64 - 1;
        conn.execute("INSERT INTO track_artist_credits (track_id, artist_id, role, position) VALUES (?1, ?2, ?3, ?4)", params![track_id, artist_id, role.as_str(), position])?;
    }
    Ok(())
}

pub fn replace_track_credits(
    conn: &Connection,
    track_id: i64,
    performers: &[String],
    original_performers: &[String],
) -> Result<(), AppError> {
    let performer_names = if performers.is_empty() {
        vec!["Unknown Artist".to_string()]
    } else {
        performers.to_vec()
    };
    replace_role_credits(conn, track_id, ArtistRole::Performer, &performer_names)?;
    replace_role_credits(
        conn,
        track_id,
        ArtistRole::OriginalPerformer,
        original_performers,
    )?;
    Ok(())
}

pub fn hydrate_tracks(conn: &Connection, tracks: &mut [Track]) -> Result<(), AppError> {
    const CHUNK_SIZE: usize = 500;
    if tracks.is_empty() {
        return Ok(());
    }
    let mut index = HashMap::new();
    for (i, track) in tracks.iter().enumerate() {
        index.insert(track.id, i);
    }
    let ids: Vec<i64> = tracks.iter().map(|t| t.id).collect();
    for chunk in ids.chunks(CHUNK_SIZE) {
        let placeholders = vec!["?"; chunk.len()].join(",");
        let sql = format!(
            "SELECT tac.track_id, a.id, a.name, tac.role, tac.position FROM track_artist_credits tac INNER JOIN artists a ON a.id=tac.artist_id WHERE tac.track_id IN ({placeholders}) ORDER BY tac.track_id, tac.role, tac.position"
        );
        let values: Vec<&dyn rusqlite::ToSql> =
            chunk.iter().map(|id| id as &dyn rusqlite::ToSql).collect();
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(values.as_slice(), |row| {
            Ok((
                row.get::<_, i64>(0)?,
                ArtistCredit {
                    artist_id: row.get(1)?,
                    name: row.get(2)?,
                    position: row.get(4)?,
                },
                row.get::<_, String>(3)?,
            ))
        })?;
        for row in rows {
            let (track_id, credit, role) = row?;
            if let Some(&i) = index.get(&track_id) {
                if role == ArtistRole::Performer.as_str() {
                    tracks[i].performers.push(credit);
                } else {
                    tracks[i].original_performers.push(credit);
                }
            }
        }
    }
    Ok(())
}

pub fn insert_track(conn: &Connection, track: &Track) -> Result<i64, AppError> {
    insert_track_with_source(conn, track, None, 0)
}

pub fn insert_track_with_source(
    conn: &Connection,
    track: &Track,
    source_folder_id: Option<i64>,
    modified_at_millis: i64,
) -> Result<i64, AppError> {
    conn.execute(
        "INSERT INTO tracks (file_path,title,duration_secs,cover_art_path,file_size_bytes,source_folder_id,modified_at_millis) VALUES (?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(file_path) DO UPDATE SET title=excluded.title,duration_secs=excluded.duration_secs,file_size_bytes=excluded.file_size_bytes,source_folder_id=COALESCE(excluded.source_folder_id,tracks.source_folder_id),modified_at_millis=CASE WHEN excluded.modified_at_millis>0 THEN excluded.modified_at_millis ELSE tracks.modified_at_millis END",
        params![track.file_path,track.title,track.duration_secs,track.cover_art_path,track.file_size_bytes,source_folder_id,modified_at_millis],
    )?;
    let id = conn.query_row(
        "SELECT id FROM tracks WHERE file_path=?1",
        params![track.file_path],
        |r| r.get(0),
    )?;
    let performers: Vec<String> = track.performers.iter().map(|a| a.name.clone()).collect();
    let originals: Vec<String> = track
        .original_performers
        .iter()
        .map(|a| a.name.clone())
        .collect();
    replace_track_credits(conn, id, &performers, &originals)?;
    Ok(id)
}
pub fn update_cover_art_path(
    conn: &Connection,
    track_id: i64,
    cover_art_path: &str,
) -> Result<(), AppError> {
    conn.execute(
        "UPDATE tracks SET cover_art_path=?1 WHERE id=?2",
        params![cover_art_path, track_id],
    )?;
    Ok(())
}

fn query_tracks(
    conn: &Connection,
    sql: &str,
    values: &[&dyn rusqlite::ToSql],
) -> Result<Vec<Track>, AppError> {
    let mut stmt = conn.prepare(sql)?;
    let mut tracks = stmt
        .query_map(values, row_to_track)?
        .collect::<Result<Vec<_>, _>>()?;
    hydrate_tracks(conn, &mut tracks)?;
    Ok(tracks)
}

pub fn get_all_tracks(conn: &Connection) -> Result<Vec<Track>, AppError> {
    query_tracks(
        conn,
        &format!("SELECT {TRACK_COLUMNS} FROM tracks ORDER BY title"),
        &[],
    )
}
pub fn get_track_by_id(conn: &Connection, id: i64) -> Result<Option<Track>, AppError> {
    let mut tracks = query_tracks(
        conn,
        &format!("SELECT {TRACK_COLUMNS} FROM tracks WHERE id=?1"),
        &[&id],
    )?;
    Ok(tracks.pop())
}
pub fn get_track_cover_path(conn: &Connection, id: i64) -> Result<Option<String>, AppError> {
    conn.query_row(
        "SELECT cover_art_path FROM tracks WHERE id=?1",
        params![id],
        |r| r.get(0),
    )
    .optional()
    .map_err(Into::into)
    .map(Option::flatten)
}
pub fn delete_track(conn: &Connection, id: i64) -> Result<(), AppError> {
    conn.execute("DELETE FROM tracks WHERE id=?1", params![id])?;
    Ok(())
}

pub fn delete_tracks(conn: &Connection, ids: &[i64]) -> Result<(), AppError> {
    if ids.is_empty() {
        return Ok(());
    }
    for chunk in ids.chunks(500) {
        let p = vec!["?"; chunk.len()].join(",");
        let sql = format!("DELETE FROM tracks WHERE id IN ({p})");
        let v: Vec<&dyn rusqlite::ToSql> =
            chunk.iter().map(|id| id as &dyn rusqlite::ToSql).collect();
        conn.execute(&sql, v.as_slice())?;
    }
    Ok(())
}

pub fn get_tracks_by_ids(conn: &Connection, ids: &[i64]) -> Result<Vec<Track>, AppError> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut all = Vec::new();
    for chunk in ids.chunks(500) {
        let p = vec!["?"; chunk.len()].join(",");
        let sql = format!("SELECT {TRACK_COLUMNS} FROM tracks WHERE id IN ({p})");
        let v: Vec<&dyn rusqlite::ToSql> =
            chunk.iter().map(|id| id as &dyn rusqlite::ToSql).collect();
        all.extend(query_tracks(conn, &sql, v.as_slice())?);
    }
    Ok(all)
}

pub fn search_tracks(conn: &Connection, query: &str) -> Result<Vec<Track>, AppError> {
    let escaped = query
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    let pattern = format!("%{escaped}%");
    query_tracks(
        conn,
        &format!(
            "SELECT {TRACK_COLUMNS} FROM tracks t WHERE t.title LIKE ?1 ESCAPE '\\' OR EXISTS (SELECT 1 FROM track_artist_credits tac INNER JOIN artists a ON a.id=tac.artist_id WHERE tac.track_id=t.id AND a.name LIKE ?1 ESCAPE '\\') ORDER BY t.title"
        ),
        &[&pattern],
    )
}

pub fn update_track_metadata(
    conn: &Connection,
    id: i64,
    title: &str,
    performers: &[String],
    originals: &[String],
) -> Result<(), AppError> {
    conn.execute("UPDATE tracks SET title=?1 WHERE id=?2", params![title, id])?;
    replace_track_credits(conn, id, performers, originals)
}

pub fn get_all_artists(conn: &Connection) -> Result<Vec<ArtistSummary>, AppError> {
    let mut stmt=conn.prepare("SELECT a.id,a.name,COUNT(DISTINCT tac.track_id),COUNT(DISTINCT CASE WHEN tac.role='performer' THEN tac.track_id END),COUNT(DISTINCT CASE WHEN tac.role='original_performer' THEN tac.track_id END) FROM artists a LEFT JOIN track_artist_credits tac ON tac.artist_id=a.id GROUP BY a.id,a.name ORDER BY a.normalized_name")?;
    stmt.query_map([], |r| {
        Ok(ArtistSummary {
            id: r.get(0)?,
            name: r.get(1)?,
            track_count: r.get(2)?,
            performer_track_count: r.get(3)?,
            original_track_count: r.get(4)?,
        })
    })?
    .collect::<Result<Vec<_>, _>>()
    .map_err(Into::into)
}

pub fn get_tracks_by_artist(
    conn: &Connection,
    artist_id: i64,
    role: Option<ArtistRole>,
) -> Result<Vec<Track>, AppError> {
    let role_value = role.map(ArtistRole::as_str);
    query_tracks(
        conn,
        &format!(
            "SELECT {TRACK_COLUMNS} FROM tracks t WHERE EXISTS (SELECT 1 FROM track_artist_credits tac WHERE tac.track_id=t.id AND tac.artist_id=?1 AND (?2 IS NULL OR tac.role=?2)) ORDER BY t.title"
        ),
        &[&artist_id, &role_value],
    )
}

pub fn increment_play_count(conn: &Connection, track_id: i64) -> Result<(), AppError> {
    conn.execute(
        "UPDATE tracks SET play_count=play_count+1,last_played_at=datetime('now') WHERE id=?1",
        params![track_id],
    )?;
    Ok(())
}
pub fn get_most_played_tracks(conn: &Connection, limit: i64) -> Result<Vec<Track>, AppError> {
    query_tracks(
        conn,
        &format!(
            "SELECT {TRACK_COLUMNS} FROM tracks WHERE play_count>0 ORDER BY play_count DESC,last_played_at DESC LIMIT ?1"
        ),
        &[&limit],
    )
}

pub fn get_track_id_by_path(conn: &Connection, file_path: &str) -> Result<Option<i64>, AppError> {
    conn.query_row(
        "SELECT id FROM tracks WHERE file_path=?1",
        params![file_path],
        |r| r.get(0),
    )
    .optional()
    .map_err(Into::into)
}
pub fn delete_track_by_path(
    conn: &Connection,
    file_path: &str,
) -> Result<Option<String>, AppError> {
    let cover: Option<String> = conn
        .query_row(
            "SELECT cover_art_path FROM tracks WHERE file_path=?1",
            params![file_path],
            |r| r.get(0),
        )
        .optional()?
        .flatten();
    conn.execute("DELETE FROM tracks WHERE file_path=?1", params![file_path])?;
    Ok(cover)
}
