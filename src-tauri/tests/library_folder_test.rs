mod common;

use std::sync::{Arc, Mutex};

use musicplayer_lib::commands::library::sync_library_folder;
use musicplayer_lib::metadata::writer;
use musicplayer_lib::storage::{folder_repo, library_repo, playlist_repo, tag_repo};

#[test]
fn folder_paths_are_unique_and_cannot_overlap() {
    let conn = common::create_test_db();
    let root = tempfile::tempdir().unwrap();
    let child = root.path().join("child");
    std::fs::create_dir(&child).unwrap();
    let id = folder_repo::add_folder(&conn, root.path().to_str().unwrap()).unwrap();
    assert_eq!(
        folder_repo::add_folder(&conn, root.path().to_str().unwrap()).unwrap(),
        id
    );
    assert!(folder_repo::add_folder(&conn, child.to_str().unwrap()).is_err());
}

#[test]
fn incremental_sync_adds_skips_and_removes_without_recreating_tracks() {
    let conn = common::create_test_db();
    let music = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let song = common::create_test_wav(music.path(), "song.wav");
    let folder_id = folder_repo::add_folder(&conn, music.path().to_str().unwrap()).unwrap();
    let db = Arc::new(Mutex::new(conn));

    let (first, _) = sync_library_folder(&db, cache.path(), folder_id).unwrap();
    assert_eq!(
        (first.added, first.updated, first.unchanged, first.removed),
        (1, 0, 0, 0)
    );
    let track_id = {
        let mut conn = db.lock().unwrap();
        let track = library_repo::get_all_tracks(&conn).unwrap().remove(0);
        library_repo::increment_play_count(&conn, track.id).unwrap();
        let tag = tag_repo::create_tag(&conn, "Favorite").unwrap();
        tag_repo::add_tags_to_tracks(&mut conn, &[track.id], &[tag.id]).unwrap();
        let playlist = playlist_repo::create_playlist(&conn, "List").unwrap();
        playlist_repo::add_to_playlist(&conn, playlist, track.id).unwrap();
        track.id
    };

    let (second, _) = sync_library_folder(&db, cache.path(), folder_id).unwrap();
    assert_eq!(
        (
            second.added,
            second.updated,
            second.unchanged,
            second.removed
        ),
        (0, 0, 1, 0)
    );
    {
        let conn = db.lock().unwrap();
        let track = library_repo::get_track_by_id(&conn, track_id)
            .unwrap()
            .unwrap();
        assert_eq!(track.play_count, 1);
    }

    std::thread::sleep(std::time::Duration::from_millis(20));
    writer::write_metadata(song.to_str().unwrap(), Some("Updated"), None, None).unwrap();
    let (third, _) = sync_library_folder(&db, cache.path(), folder_id).unwrap();
    assert_eq!((third.added, third.updated, third.unchanged), (0, 1, 0));
    {
        let conn = db.lock().unwrap();
        let track = library_repo::get_track_by_id(&conn, track_id)
            .unwrap()
            .unwrap();
        assert_eq!(track.title, "Updated");
        assert_eq!(track.play_count, 1);
        assert_eq!(
            tag_repo::get_tags_for_track(&conn, track_id).unwrap().len(),
            1
        );
        assert_eq!(
            playlist_repo::get_all_playlists(&conn).unwrap()[0].track_ids,
            vec![track_id]
        );
    }

    std::fs::remove_file(song).unwrap();
    let (fourth, removed_ids) = sync_library_folder(&db, cache.path(), folder_id).unwrap();
    assert_eq!(fourth.removed, 1);
    assert_eq!(removed_ids, vec![track_id]);
}

#[test]
fn unavailable_folder_keeps_indexed_tracks_and_records_error() {
    let conn = common::create_test_db();
    let music = tempfile::tempdir().unwrap();
    let path = music.path().to_path_buf();
    let cache = tempfile::tempdir().unwrap();
    common::create_test_wav(&path, "song.wav");
    let folder_id = folder_repo::add_folder(&conn, path.to_str().unwrap()).unwrap();
    let db = Arc::new(Mutex::new(conn));
    sync_library_folder(&db, cache.path(), folder_id).unwrap();
    music.close().unwrap();

    assert!(sync_library_folder(&db, cache.path(), folder_id).is_err());
    let conn = db.lock().unwrap();
    assert_eq!(library_repo::get_all_tracks(&conn).unwrap().len(), 1);
    assert!(
        folder_repo::get_folder(&conn, folder_id)
            .unwrap()
            .last_error
            .is_some()
    );
}

#[test]
fn pausing_and_removing_folder_have_explicit_data_retention() {
    let conn = common::create_test_db();
    let music = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    common::create_test_wav(music.path(), "song.wav");
    let folder_id = folder_repo::add_folder(&conn, music.path().to_str().unwrap()).unwrap();
    let db = Arc::new(Mutex::new(conn));
    sync_library_folder(&db, cache.path(), folder_id).unwrap();

    let conn = db.lock().unwrap();
    folder_repo::set_enabled(&conn, folder_id, false).unwrap();
    assert!(!folder_repo::get_folder(&conn, folder_id).unwrap().enabled);
    let removed = folder_repo::remove_folder(&conn, folder_id, false).unwrap();
    assert!(removed.is_empty());
    assert_eq!(library_repo::get_all_tracks(&conn).unwrap().len(), 1);
}

#[test]
fn removing_folder_can_delete_only_the_indexed_tracks() {
    let conn = common::create_test_db();
    let music = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    common::create_test_wav(music.path(), "song.wav");
    let folder_id = folder_repo::add_folder(&conn, music.path().to_str().unwrap()).unwrap();
    let db = Arc::new(Mutex::new(conn));
    sync_library_folder(&db, cache.path(), folder_id).unwrap();

    let conn = db.lock().unwrap();
    let removed = folder_repo::remove_folder(&conn, folder_id, true).unwrap();
    assert_eq!(removed.len(), 1);
    assert!(library_repo::get_all_tracks(&conn).unwrap().is_empty());
    assert!(music.path().join("song.wav").exists());
}
