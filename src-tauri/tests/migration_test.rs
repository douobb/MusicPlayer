use musicplayer_lib::storage::db;
use rusqlite::Connection;

#[test]
fn fresh_db_creates_current_schema() {
    let conn = Connection::open_in_memory().unwrap();
    db::run_migrations(&conn).unwrap();

    let version: i64 = conn
        .query_row("SELECT MAX(version) FROM schema_version", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(version, 12);

    let columns: Vec<String> = conn
        .prepare("PRAGMA table_info(tracks)")
        .unwrap()
        .query_map([], |row| row.get(1))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert!(!columns.iter().any(|column| column == "album"));
    assert!(!columns.iter().any(|column| column == "album_artist"));
    assert!(!columns.iter().any(|column| column == "artist"));
    assert!(columns.iter().any(|column| column == "source_folder_id"));
    assert!(columns.iter().any(|column| column == "modified_at_millis"));
    let artist_tables: i64 = conn.query_row("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('artists','track_artist_credits')", [], |row| row.get(0)).unwrap();
    assert_eq!(artist_tables, 2);

    let folder_columns: Vec<String> = conn
        .prepare("PRAGMA table_info(scan_folders)")
        .unwrap()
        .query_map([], |row| row.get(1))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert!(folder_columns.iter().any(|column| column == "enabled"));
    assert!(folder_columns.iter().any(|column| column == "last_scan_at"));

    let tag_table_exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'tags')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(tag_table_exists);
}

#[test]
fn current_schema_initialization_is_idempotent() {
    let conn = Connection::open_in_memory().unwrap();
    db::run_migrations(&conn).unwrap();
    db::run_migrations(&conn).unwrap();

    let versions: i64 = conn
        .query_row("SELECT COUNT(*) FROM schema_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(versions, 1);
}

#[test]
fn legacy_development_schema_requires_clean_reset() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE schema_version (version INTEGER NOT NULL);
         INSERT INTO schema_version (version) VALUES (9);",
    )
    .unwrap();

    let error = db::run_migrations(&conn).unwrap_err().to_string();
    assert!(error.contains("schema 9"));
    assert!(error.contains("musicplayer.db"));
}
