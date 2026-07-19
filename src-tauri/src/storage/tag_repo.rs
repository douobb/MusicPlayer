use std::collections::{HashMap, HashSet};

use rusqlite::{Connection, OptionalExtension, params, params_from_iter};

use crate::error::AppError;
use crate::models::tag::{TagAssignment, TagStatistics, TagSummary};
use crate::models::track::Track;
use crate::storage::library_repo::{TRACK_COLUMNS, hydrate_tracks, row_to_track};

fn normalized_name(name: &str) -> Result<(String, String), AppError> {
    let display_name = name.trim();
    if display_name.is_empty() {
        return Err(AppError::Generic("Tag 名稱不可為空白".to_string()));
    }
    Ok((display_name.to_string(), display_name.to_lowercase()))
}

fn get_tag(conn: &Connection, tag_id: i64) -> Result<Option<TagSummary>, AppError> {
    conn.query_row(
        "SELECT t.id, t.name, COUNT(tt.track_id)
         FROM tags t
         LEFT JOIN track_tags tt ON tt.tag_id = t.id
         WHERE t.id = ?1
         GROUP BY t.id, t.name",
        params![tag_id],
        |row| {
            Ok(TagSummary {
                id: row.get(0)?,
                name: row.get(1)?,
                track_count: row.get(2)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

pub fn create_tag(conn: &Connection, name: &str) -> Result<TagSummary, AppError> {
    let (display_name, normalized_name) = normalized_name(name)?;
    conn.execute(
        "INSERT INTO tags (name, normalized_name) VALUES (?1, ?2)",
        params![display_name, normalized_name],
    )?;
    let id = conn.last_insert_rowid();
    get_tag(conn, id)?.ok_or_else(|| AppError::Generic("建立 Tag 後無法讀回資料".to_string()))
}

pub fn rename_tag(conn: &Connection, tag_id: i64, name: &str) -> Result<TagSummary, AppError> {
    let (display_name, normalized_name) = normalized_name(name)?;
    let changed = conn.execute(
        "UPDATE tags SET name = ?1, normalized_name = ?2 WHERE id = ?3",
        params![display_name, normalized_name, tag_id],
    )?;
    if changed == 0 {
        return Err(AppError::Generic(format!("Tag {tag_id} 不存在")));
    }
    get_tag(conn, tag_id)?.ok_or_else(|| AppError::Generic(format!("Tag {tag_id} 不存在")))
}

pub fn delete_tag(conn: &Connection, tag_id: i64) -> Result<(), AppError> {
    conn.execute("DELETE FROM tags WHERE id = ?1", params![tag_id])?;
    Ok(())
}

pub fn delete_empty_tags(conn: &Connection) -> Result<usize, AppError> {
    conn.execute(
        "DELETE FROM tags WHERE NOT EXISTS (SELECT 1 FROM track_tags WHERE track_tags.tag_id = tags.id)",
        [],
    )
    .map_err(Into::into)
}
pub fn merge_tags(
    conn: &mut Connection,
    source_tag_id: i64,
    target_tag_id: i64,
) -> Result<TagSummary, AppError> {
    if source_tag_id == target_tag_id {
        return Err(AppError::Generic("來源與目標 Tag 不可相同".to_string()));
    }

    let tx = conn.transaction()?;
    let source_exists: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM tags WHERE id = ?1)",
        params![source_tag_id],
        |row| row.get(0),
    )?;
    let target_exists: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM tags WHERE id = ?1)",
        params![target_tag_id],
        |row| row.get(0),
    )?;
    if !source_exists || !target_exists {
        return Err(AppError::Generic("來源或目標 Tag 不存在".to_string()));
    }

    tx.execute(
        "INSERT OR IGNORE INTO track_tags (track_id, tag_id)
         SELECT track_id, ?1 FROM track_tags WHERE tag_id = ?2",
        params![target_tag_id, source_tag_id],
    )?;
    tx.execute("DELETE FROM tags WHERE id = ?1", params![source_tag_id])?;
    tx.commit()?;

    get_tag(conn, target_tag_id)?
        .ok_or_else(|| AppError::Generic(format!("Tag {target_tag_id} 不存在")))
}

pub fn get_all_tags(conn: &Connection) -> Result<Vec<TagSummary>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT t.id, t.name, COUNT(tt.track_id) AS track_count
         FROM tags t
         LEFT JOIN track_tags tt ON tt.tag_id = t.id
         GROUP BY t.id, t.name
         ORDER BY t.normalized_name",
    )?;
    stmt.query_map([], |row| {
        Ok(TagSummary {
            id: row.get(0)?,
            name: row.get(1)?,
            track_count: row.get(2)?,
        })
    })?
    .collect::<Result<Vec<_>, _>>()
    .map_err(Into::into)
}

pub fn get_tag_statistics(conn: &Connection) -> Result<TagStatistics, AppError> {
    let (tag_count, tagged_track_count, untagged_track_count, assignment_count) = conn.query_row(
        "SELECT
            (SELECT COUNT(*) FROM tags),
            (SELECT COUNT(DISTINCT track_id) FROM track_tags),
            (SELECT COUNT(*) FROM tracks t
             WHERE NOT EXISTS (
                 SELECT 1 FROM track_tags tt WHERE tt.track_id = t.id
             )),
            (SELECT COUNT(*) FROM track_tags)",
        [],
        |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
            ))
        },
    )?;
    let most_used_tag = conn
        .query_row(
            "SELECT t.id, t.name, COUNT(tt.track_id) AS track_count
             FROM tags t
             INNER JOIN track_tags tt ON tt.tag_id = t.id
             GROUP BY t.id, t.name
             ORDER BY track_count DESC, t.normalized_name
             LIMIT 1",
            [],
            |row| {
                Ok(TagSummary {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    track_count: row.get(2)?,
                })
            },
        )
        .optional()?;
    let average_tags_per_tagged_track = if tagged_track_count == 0 {
        0.0
    } else {
        assignment_count as f64 / tagged_track_count as f64
    };

    Ok(TagStatistics {
        tag_count,
        tagged_track_count,
        untagged_track_count,
        assignment_count,
        average_tags_per_tagged_track,
        most_used_tag,
    })
}

pub fn get_tags_for_track(conn: &Connection, track_id: i64) -> Result<Vec<TagSummary>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT t.id, t.name, 1
         FROM tags t
         INNER JOIN track_tags tt ON tt.tag_id = t.id
         WHERE tt.track_id = ?1
         ORDER BY t.normalized_name",
    )?;
    stmt.query_map(params![track_id], |row| {
        Ok(TagSummary {
            id: row.get(0)?,
            name: row.get(1)?,
            track_count: row.get(2)?,
        })
    })?
    .collect::<Result<Vec<_>, _>>()
    .map_err(Into::into)
}

pub fn get_tag_assignments_for_tracks(
    conn: &Connection,
    track_ids: &[i64],
) -> Result<Vec<TagAssignment>, AppError> {
    let mut assignments: Vec<TagAssignment> = get_all_tags(conn)?
        .into_iter()
        .map(|tag| TagAssignment {
            id: tag.id,
            name: tag.name,
            assigned_count: 0,
        })
        .collect();
    if track_ids.is_empty() || assignments.is_empty() {
        return Ok(assignments);
    }

    let positions: HashMap<i64, usize> = assignments
        .iter()
        .enumerate()
        .map(|(index, tag)| (tag.id, index))
        .collect();
    let unique_track_ids: Vec<i64> = track_ids
        .iter()
        .copied()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    for chunk in unique_track_ids.chunks(500) {
        let placeholders = vec!["?"; chunk.len()].join(",");
        let sql = format!(
            "SELECT tag_id, COUNT(DISTINCT track_id) FROM track_tags WHERE track_id IN ({placeholders}) GROUP BY tag_id"
        );
        let mut stmt = conn.prepare(&sql)?;
        let counts = stmt
            .query_map(params_from_iter(chunk.iter()), |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        for (tag_id, count) in counts {
            if let Some(index) = positions.get(&tag_id) {
                assignments[*index].assigned_count += count;
            }
        }
    }

    Ok(assignments)
}
pub fn add_tags_to_tracks(
    conn: &mut Connection,
    track_ids: &[i64],
    tag_ids: &[i64],
) -> Result<(), AppError> {
    if track_ids.is_empty() || tag_ids.is_empty() {
        return Ok(());
    }
    let tx = conn.transaction()?;
    for track_id in track_ids {
        for tag_id in tag_ids {
            tx.execute(
                "INSERT OR IGNORE INTO track_tags (track_id, tag_id) VALUES (?1, ?2)",
                params![track_id, tag_id],
            )?;
        }
    }
    tx.commit()?;
    Ok(())
}

pub fn remove_tags_from_tracks(
    conn: &mut Connection,
    track_ids: &[i64],
    tag_ids: &[i64],
) -> Result<(), AppError> {
    if track_ids.is_empty() || tag_ids.is_empty() {
        return Ok(());
    }
    let tx = conn.transaction()?;
    for track_id in track_ids {
        for tag_id in tag_ids {
            tx.execute(
                "DELETE FROM track_tags WHERE track_id = ?1 AND tag_id = ?2",
                params![track_id, tag_id],
            )?;
        }
    }
    tx.commit()?;
    Ok(())
}

pub fn get_tracks_by_tag(conn: &Connection, tag_id: i64) -> Result<Vec<Track>, AppError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {TRACK_COLUMNS} FROM tracks t
         INNER JOIN track_tags tt ON tt.track_id = t.id
         WHERE tt.tag_id = ?1
         ORDER BY t.title"
    ))?;
    let mut tracks = stmt
        .query_map(params![tag_id], row_to_track)?
        .collect::<Result<Vec<_>, _>>()?;
    hydrate_tracks(conn, &mut tracks)?;
    Ok(tracks)
}
