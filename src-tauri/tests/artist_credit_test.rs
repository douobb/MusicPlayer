mod common;

use musicplayer_lib::models::artist::{ArtistCredit, ArtistRole};
use musicplayer_lib::storage::library_repo;

#[test]
fn track_supports_ordered_multiple_performers_and_originals() {
    let conn = common::create_test_db();
    let mut track = common::create_test_track(1);
    track.performers = vec![
        ArtistCredit {
            artist_id: 0,
            name: "Singer A".into(),
            position: 0,
        },
        ArtistCredit {
            artist_id: 0,
            name: "Singer B".into(),
            position: 1,
        },
    ];
    track.original_performers = vec![ArtistCredit {
        artist_id: 0,
        name: "Original".into(),
        position: 0,
    }];
    let id = library_repo::insert_track(&conn, &track).unwrap();
    let saved = library_repo::get_track_by_id(&conn, id).unwrap().unwrap();
    assert_eq!(
        saved
            .performers
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Singer A", "Singer B"]
    );
    assert_eq!(saved.original_performers[0].name, "Original");
}

#[test]
fn artist_names_are_trimmed_unique_and_case_insensitive() {
    let conn = common::create_test_db();
    let id = library_repo::create_artist(&conn, "  Artist A  ").unwrap();
    assert!(library_repo::create_artist(&conn, "artist a").is_err());
    library_repo::rename_artist(&conn, id, " Artist B ").unwrap();
    assert_eq!(
        library_repo::get_all_artists(&conn).unwrap()[0].name,
        "Artist B"
    );
}

#[test]
fn artist_role_queries_and_counts_are_distinct() {
    let conn = common::create_test_db();
    let mut track = common::create_test_track(1);
    track.performers[0].name = "Current".into();
    track.original_performers = vec![ArtistCredit {
        artist_id: 0,
        name: "Original".into(),
        position: 0,
    }];
    library_repo::insert_track(&conn, &track).unwrap();
    let artists = library_repo::get_all_artists(&conn).unwrap();
    let current = artists.iter().find(|a| a.name == "Current").unwrap();
    let original = artists.iter().find(|a| a.name == "Original").unwrap();
    assert_eq!(
        (current.performer_track_count, current.original_track_count),
        (1, 0)
    );
    assert_eq!(
        (
            original.performer_track_count,
            original.original_track_count
        ),
        (0, 1)
    );
    assert_eq!(
        library_repo::get_tracks_by_artist(&conn, original.id, Some(ArtistRole::OriginalPerformer))
            .unwrap()
            .len(),
        1
    );
    assert!(
        library_repo::get_tracks_by_artist(&conn, original.id, Some(ArtistRole::Performer))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn merge_and_cleanup_artists_preserve_credits() {
    let conn = common::create_test_db();
    let mut track = common::create_test_track(1);
    track.performers[0].name = "Source".into();
    let track_id = library_repo::insert_track(&conn, &track).unwrap();
    let target = library_repo::create_artist(&conn, "Target").unwrap();
    let source = library_repo::get_all_artists(&conn)
        .unwrap()
        .into_iter()
        .find(|a| a.name == "Source")
        .unwrap();
    library_repo::merge_artists(&conn, source.id, target).unwrap();
    assert_eq!(
        library_repo::get_track_by_id(&conn, track_id)
            .unwrap()
            .unwrap()
            .performers[0]
            .name,
        "Target"
    );
    assert_eq!(library_repo::delete_unused_artists(&conn).unwrap(), 0);
}
