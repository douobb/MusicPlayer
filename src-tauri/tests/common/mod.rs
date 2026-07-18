#![allow(dead_code)]

use rusqlite::Connection;

use musicplayer_lib::models::track::Track;

/// Create an in-memory SQLite database with the full schema.
pub fn create_test_db() -> Connection {
    let conn = Connection::open_in_memory().expect("failed to open in-memory db");
    conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
    conn.execute_batch(
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

        CREATE INDEX idx_track_tags_tag_id ON track_tags(tag_id);
        ",
    )
    .expect("failed to create schema");
    conn
}

/// 建立具備合理預設值的測試曲目；`idx` 用來確保資料唯一。
pub fn create_test_track(idx: u32) -> Track {
    Track {
        id: 0,
        file_path: format!("/tmp/test_music/track_{}.mp3", idx),
        title: format!("Test Track {}", idx),
        performers: vec![musicplayer_lib::models::artist::ArtistCredit {
            artist_id: 0,
            name: format!("Artist {}", idx),
            position: 0,
        }],
        original_performers: Vec::new(),
        duration_secs: 180.0 + idx as f64,
        cover_art: None,
        cover_art_path: None,
        file_size_bytes: 1024 * (idx as i64 + 1),
        play_count: 0,
        last_played_at: None,
    }
}
/// Create a minimal valid WAV file (PCM 16-bit mono, 44100 Hz) in the given directory.
/// Returns the full path to the created file.
pub fn create_test_wav(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    let sample_rate: u32 = 44100;
    let bits_per_sample: u16 = 16;
    let num_channels: u16 = 1;
    let num_samples: u32 = 4410; // 0.1 seconds
    let byte_rate = sample_rate * (bits_per_sample as u32 / 8) * num_channels as u32;
    let block_align = num_channels * (bits_per_sample / 8);
    let data_size = num_samples * (bits_per_sample as u32 / 8) * num_channels as u32;
    let file_size = 36 + data_size;

    let mut buf: Vec<u8> = Vec::new();

    // RIFF header
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&file_size.to_le_bytes());
    buf.extend_from_slice(b"WAVE");

    // fmt sub-chunk
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes()); // sub-chunk size
    buf.extend_from_slice(&1u16.to_le_bytes()); // audio format (PCM)
    buf.extend_from_slice(&num_channels.to_le_bytes());
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&byte_rate.to_le_bytes());
    buf.extend_from_slice(&block_align.to_le_bytes());
    buf.extend_from_slice(&bits_per_sample.to_le_bytes());

    // data sub-chunk
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_size.to_le_bytes());

    // Generate a simple sine wave (440 Hz)
    for i in 0..num_samples {
        let t = i as f64 / sample_rate as f64;
        let sample = (t * 440.0 * 2.0 * std::f64::consts::PI).sin();
        let value = (sample * 16000.0) as i16;
        buf.extend_from_slice(&value.to_le_bytes());
    }

    std::fs::write(&path, &buf).expect("failed to write test wav");
    path
}

/// Create a test WAV with an embedded ID3v2.4 chunk containing TIT2/TPE1/TDRC text frames.
/// `tdrc` may contain non-digit characters to reproduce the lofty timestamp parsing bug.
pub fn create_test_wav_with_id3(
    dir: &std::path::Path,
    name: &str,
    title: &str,
    artist: &str,
    tdrc: &str,
) -> std::path::PathBuf {
    create_test_wav_with_id3_frames(
        dir,
        name,
        &[(*b"TIT2", title), (*b"TPE1", artist), (*b"TDRC", tdrc)],
    )
}

/// Create a test WAV with an embedded ID3v2.4 chunk containing the given text frames.
pub fn create_test_wav_with_id3_frames(
    dir: &std::path::Path,
    name: &str,
    frames_spec: &[([u8; 4], &str)],
) -> std::path::PathBuf {
    fn syncsafe(n: u32) -> [u8; 4] {
        [
            ((n >> 21) & 0x7F) as u8,
            ((n >> 14) & 0x7F) as u8,
            ((n >> 7) & 0x7F) as u8,
            (n & 0x7F) as u8,
        ]
    }

    fn text_frame(id: &[u8; 4], text: &str) -> Vec<u8> {
        let mut body = vec![3u8]; // text encoding: UTF-8
        body.extend_from_slice(text.as_bytes());
        let mut frame = Vec::with_capacity(10 + body.len());
        frame.extend_from_slice(id);
        frame.extend_from_slice(&syncsafe(body.len() as u32));
        frame.extend_from_slice(&[0, 0]); // frame flags
        frame.extend_from_slice(&body);
        frame
    }

    let path = create_test_wav(dir, name);

    let mut frames = Vec::new();
    for (id, text) in frames_spec {
        frames.extend(text_frame(id, text));
    }

    let mut tag = Vec::with_capacity(10 + frames.len());
    tag.extend_from_slice(b"ID3\x04\x00\x00"); // ID3v2.4 header, no flags
    tag.extend_from_slice(&syncsafe(frames.len() as u32));
    tag.extend_from_slice(&frames);

    let mut buf = std::fs::read(&path).expect("failed to read test wav");
    buf.extend_from_slice(b"ID3 ");
    buf.extend_from_slice(&(tag.len() as u32).to_le_bytes());
    buf.extend_from_slice(&tag);
    if tag.len() % 2 == 1 {
        buf.push(0); // RIFF chunks are word-aligned
    }
    let riff_size = (buf.len() - 8) as u32;
    buf[4..8].copy_from_slice(&riff_size.to_le_bytes());
    std::fs::write(&path, &buf).expect("failed to write test wav with id3");
    path
}
