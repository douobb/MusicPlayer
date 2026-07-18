mod common;

use musicplayer_lib::metadata::{reader, writer};
use musicplayer_lib::models::track::Track;

#[test]
fn test_read_metadata_wav_file() {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let wav_path = common::create_test_wav(dir.path(), "test.wav");

    let result = reader::read_metadata(wav_path.to_str().unwrap());
    assert!(result.is_ok(), "read_metadata failed: {:?}", result.err());

    let track = result.unwrap();
    assert_eq!(track.file_path, wav_path.to_str().unwrap());
    assert!(track.duration_secs > 0.0);
    // WAV files typically don't have tags, so should fallback to filename
    assert_eq!(track.title, "test");
    assert_eq!(track.performers[0].name, "Unknown Artist");
    assert!(
        track.file_size_bytes > 0,
        "file_size_bytes should be > 0 for a real file"
    );
}

#[test]
fn test_read_metadata_invalid_timestamp_preserves_tags() {
    // Regression for the lofty timestamp bug: an ASCII TDRC frame with
    // non-digit characters (e.g. the Japanese era date "H17.10.26") errors in
    // BestAttempt mode and would fail the whole file read. It must not break
    // the import, and the remaining tags (title/artist) should still be read
    // instead of falling back to the filename. Fully non-ASCII timestamps are
    // already skipped gracefully by lofty itself.
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let path = common::create_test_wav_with_id3(
        dir.path(),
        "japanese_date.wav",
        "日本語タイトル",
        "テストアーティスト",
        "H17.10.26",
    );

    let track = reader::read_metadata(path.to_str().unwrap()).unwrap();
    assert_eq!(track.title, "日本語タイトル");
    assert_eq!(track.performers[0].name, "テストアーティスト");
}

#[test]
fn test_read_metadata_nonexistent_file() {
    let result = reader::read_metadata("/nonexistent/file.mp3");
    assert!(result.is_err());
}

#[test]
fn test_read_metadata_fallback_no_tags() {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let wav_path = common::create_test_wav(dir.path(), "my_song.wav");

    let track = reader::read_metadata(wav_path.to_str().unwrap()).unwrap();
    // Should fallback to file stem as title
    assert_eq!(track.title, "my_song");
    assert_eq!(track.performers[0].name, "Unknown Artist");
}

#[test]
fn test_read_metadata_cover_art_none_for_wav() {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let wav_path = common::create_test_wav(dir.path(), "nocover.wav");

    let track = reader::read_metadata(wav_path.to_str().unwrap()).unwrap();
    assert!(track.cover_art.is_none());
}

#[test]
fn test_read_metadata_track_id_is_zero() {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let wav_path = common::create_test_wav(dir.path(), "zero_id.wav");

    let track = reader::read_metadata(wav_path.to_str().unwrap()).unwrap();
    // read_metadata always returns id=0 (DB assigns the real id)
    assert_eq!(track.id, 0);
}

#[test]
fn test_read_cover_art_none_for_wav() {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let wav_path = common::create_test_wav(dir.path(), "cover_test.wav");

    let cover = reader::extract_cover_art_bytes(wav_path.to_str().unwrap());
    assert!(cover.is_none());
}

#[test]
fn test_read_cover_art_nonexistent_file() {
    let cover = reader::extract_cover_art_bytes("/nonexistent/file.mp3");
    assert!(cover.is_none());
}

#[test]
fn test_read_track_details_wav_file() {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let wav_path = common::create_test_wav(dir.path(), "details_test.wav");

    let track = reader::read_metadata(wav_path.to_str().unwrap()).unwrap();
    let details = reader::read_track_details(wav_path.to_str().unwrap(), &track).unwrap();

    assert_eq!(details.sample_rate_hz, Some(44100));
    assert_eq!(details.channels, Some(1)); // mono
    assert_eq!(details.bits_per_sample, Some(16));
    assert_eq!(details.format, "WAV");
    assert!(details.file_size_bytes > 0);
    assert!(details.duration_secs > 0.0);
}

#[test]
fn test_read_track_details_nonexistent_file() {
    let track = Track {
        id: 1,
        file_path: "/nonexistent/file.mp3".to_string(),
        title: "Fake".to_string(),
        performers: vec![musicplayer_lib::models::artist::ArtistCredit {
            artist_id: 0,
            name: "Fake".to_string(),
            position: 0,
        }],
        original_performers: Vec::new(),
        duration_secs: 0.0,
        cover_art: None,
        cover_art_path: None,
        file_size_bytes: 0,
        play_count: 0,
        last_played_at: None,
    };

    let result = reader::read_track_details("/nonexistent/file.mp3", &track);
    assert!(result.is_err());
}

#[test]
fn test_multiple_performers_and_originals_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let path = common::create_test_wav(dir.path(), "credits.wav");
    let performers = vec!["Singer A".to_string(), "Singer B".to_string()];
    let originals = vec!["Original A".to_string(), "Original B".to_string()];
    writer::write_metadata(
        path.to_str().unwrap(),
        Some("Cover"),
        Some(&performers),
        Some(&originals),
    )
    .unwrap();
    let track = reader::read_metadata(path.to_str().unwrap()).unwrap();
    assert_eq!(
        track
            .performers
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Singer A", "Singer B"]
    );
    assert_eq!(
        track
            .original_performers
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Original A", "Original B"]
    );
}
