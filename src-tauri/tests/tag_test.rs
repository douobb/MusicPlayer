mod common;

use common::{create_test_db, create_test_track};
use musicplayer_lib::storage::{library_repo, tag_repo};

#[test]
fn create_tag_trims_name_and_lists_empty_tag() {
    let conn = create_test_db();
    let tag = tag_repo::create_tag(&conn, "  Chill  ").unwrap();
    assert_eq!(tag.name, "Chill");
    assert_eq!(tag.track_count, 0);
    assert_eq!(tag_repo::get_all_tags(&conn).unwrap(), vec![tag]);
}

#[test]
fn tag_statistics_are_zero_for_an_empty_library() {
    let conn = create_test_db();
    let statistics = tag_repo::get_tag_statistics(&conn).unwrap();

    assert_eq!(statistics.tag_count, 0);
    assert_eq!(statistics.tagged_track_count, 0);
    assert_eq!(statistics.untagged_track_count, 0);
    assert_eq!(statistics.assignment_count, 0);
    assert_eq!(statistics.average_tags_per_tagged_track, 0.0);
    assert_eq!(statistics.most_used_tag, None);
}

#[test]
fn tag_statistics_distinguish_tracks_from_assignments() {
    let mut conn = create_test_db();
    let first_track = library_repo::insert_track(&conn, &create_test_track(1)).unwrap();
    let second_track = library_repo::insert_track(&conn, &create_test_track(2)).unwrap();
    library_repo::insert_track(&conn, &create_test_track(3)).unwrap();
    let rock = tag_repo::create_tag(&conn, "Rock").unwrap();
    let chill = tag_repo::create_tag(&conn, "Chill").unwrap();

    tag_repo::add_tags_to_tracks(&mut conn, &[first_track, second_track], &[rock.id]).unwrap();
    tag_repo::add_tags_to_tracks(&mut conn, &[first_track], &[chill.id]).unwrap();

    let statistics = tag_repo::get_tag_statistics(&conn).unwrap();
    assert_eq!(statistics.tag_count, 2);
    assert_eq!(statistics.tagged_track_count, 2);
    assert_eq!(statistics.untagged_track_count, 1);
    assert_eq!(statistics.assignment_count, 3);
    assert_eq!(statistics.average_tags_per_tagged_track, 1.5);
    let most_used_tag = statistics.most_used_tag.unwrap();
    assert_eq!(most_used_tag.id, rock.id);
    assert_eq!(most_used_tag.name, "Rock");
    assert_eq!(most_used_tag.track_count, 2);
}

#[test]
fn create_tag_rejects_blank_name() {
    let conn = create_test_db();
    assert!(tag_repo::create_tag(&conn, "   ").is_err());
}

#[test]
fn tag_names_are_unique_ignoring_case_and_whitespace() {
    let conn = create_test_db();
    tag_repo::create_tag(&conn, "Rock").unwrap();
    assert!(tag_repo::create_tag(&conn, " rock ").is_err());
}

#[test]
fn rename_tag_updates_display_name_and_rejects_collision() {
    let conn = create_test_db();
    let first = tag_repo::create_tag(&conn, "Focus").unwrap();
    tag_repo::create_tag(&conn, "Workout").unwrap();

    let renamed = tag_repo::rename_tag(&conn, first.id, " Deep Focus ").unwrap();
    assert_eq!(renamed.name, "Deep Focus");
    assert!(tag_repo::rename_tag(&conn, first.id, "WORKOUT").is_err());
}

#[test]
fn add_and_remove_tags_from_multiple_tracks() {
    let mut conn = create_test_db();
    let first_track = library_repo::insert_track(&conn, &create_test_track(1)).unwrap();
    let second_track = library_repo::insert_track(&conn, &create_test_track(2)).unwrap();
    let chill = tag_repo::create_tag(&conn, "Chill").unwrap();
    let favorite = tag_repo::create_tag(&conn, "Favorite").unwrap();

    tag_repo::add_tags_to_tracks(
        &mut conn,
        &[first_track, second_track],
        &[chill.id, favorite.id],
    )
    .unwrap();

    assert_eq!(
        tag_repo::get_tags_for_track(&conn, first_track)
            .unwrap()
            .len(),
        2
    );
    assert_eq!(tag_repo::get_all_tags(&conn).unwrap()[0].track_count, 2);

    tag_repo::remove_tags_from_tracks(&mut conn, &[first_track], &[chill.id]).unwrap();
    let first_tags = tag_repo::get_tags_for_track(&conn, first_track).unwrap();
    assert_eq!(first_tags.len(), 1);
    assert_eq!(first_tags[0].name, "Favorite");
}

#[test]
fn adding_same_tag_twice_is_idempotent() {
    let mut conn = create_test_db();
    let track_id = library_repo::insert_track(&conn, &create_test_track(1)).unwrap();
    let tag = tag_repo::create_tag(&conn, "Chill").unwrap();

    tag_repo::add_tags_to_tracks(&mut conn, &[track_id, track_id], &[tag.id, tag.id]).unwrap();
    assert_eq!(
        tag_repo::get_tags_for_track(&conn, track_id).unwrap().len(),
        1
    );
}

#[test]
fn deleting_tag_cascades_track_relationships_only() {
    let mut conn = create_test_db();
    let track_id = library_repo::insert_track(&conn, &create_test_track(1)).unwrap();
    let tag = tag_repo::create_tag(&conn, "Temporary").unwrap();
    tag_repo::add_tags_to_tracks(&mut conn, &[track_id], &[tag.id]).unwrap();

    tag_repo::delete_tag(&conn, tag.id).unwrap();
    assert!(
        tag_repo::get_tags_for_track(&conn, track_id)
            .unwrap()
            .is_empty()
    );
    assert!(
        library_repo::get_track_by_id(&conn, track_id)
            .unwrap()
            .is_some()
    );
}

#[test]
fn deleting_track_cascades_tag_relationship_but_keeps_tag() {
    let mut conn = create_test_db();
    let track_id = library_repo::insert_track(&conn, &create_test_track(1)).unwrap();
    let tag = tag_repo::create_tag(&conn, "Keep").unwrap();
    tag_repo::add_tags_to_tracks(&mut conn, &[track_id], &[tag.id]).unwrap();

    library_repo::delete_track(&conn, track_id).unwrap();
    let tags = tag_repo::get_all_tags(&conn).unwrap();
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0].track_count, 0);
}

#[test]
fn merge_tags_moves_relationships_without_duplicates() {
    let mut conn = create_test_db();
    let first_track = library_repo::insert_track(&conn, &create_test_track(1)).unwrap();
    let second_track = library_repo::insert_track(&conn, &create_test_track(2)).unwrap();
    let source = tag_repo::create_tag(&conn, "Source").unwrap();
    let target = tag_repo::create_tag(&conn, "Target").unwrap();

    tag_repo::add_tags_to_tracks(&mut conn, &[first_track, second_track], &[source.id]).unwrap();
    tag_repo::add_tags_to_tracks(&mut conn, &[first_track], &[target.id]).unwrap();

    let merged = tag_repo::merge_tags(&mut conn, source.id, target.id).unwrap();
    assert_eq!(merged.track_count, 2);
    assert_eq!(tag_repo::get_all_tags(&conn).unwrap().len(), 1);
}

#[test]
fn get_tracks_by_tag_returns_only_assigned_tracks() {
    let mut conn = create_test_db();
    let first_track = library_repo::insert_track(&conn, &create_test_track(1)).unwrap();
    library_repo::insert_track(&conn, &create_test_track(2)).unwrap();
    let tag = tag_repo::create_tag(&conn, "Selected").unwrap();
    tag_repo::add_tags_to_tracks(&mut conn, &[first_track], &[tag.id]).unwrap();

    let tracks = tag_repo::get_tracks_by_tag(&conn, tag.id).unwrap();
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].id, first_track);
}

#[test]
fn delete_empty_tags_keeps_tags_in_use() {
    let mut conn = create_test_db();
    let track_id = library_repo::insert_track(&conn, &create_test_track(1)).unwrap();
    let used = tag_repo::create_tag(&conn, "Used").unwrap();
    tag_repo::create_tag(&conn, "Empty").unwrap();
    tag_repo::add_tags_to_tracks(&mut conn, &[track_id], &[used.id]).unwrap();

    assert_eq!(tag_repo::delete_empty_tags(&conn).unwrap(), 1);
    let remaining = tag_repo::get_all_tags(&conn).unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].id, used.id);
    assert_eq!(remaining[0].track_count, 1);
}

#[test]
fn batch_assignment_status_distinguishes_full_partial_and_unused_tags() {
    let mut conn = create_test_db();
    let first_track = library_repo::insert_track(&conn, &create_test_track(1)).unwrap();
    let second_track = library_repo::insert_track(&conn, &create_test_track(2)).unwrap();
    let full = tag_repo::create_tag(&conn, "Full").unwrap();
    let partial = tag_repo::create_tag(&conn, "Partial").unwrap();
    let unused = tag_repo::create_tag(&conn, "Unused").unwrap();

    tag_repo::add_tags_to_tracks(&mut conn, &[first_track, second_track], &[full.id]).unwrap();
    tag_repo::add_tags_to_tracks(&mut conn, &[first_track], &[partial.id]).unwrap();

    let assignments =
        tag_repo::get_tag_assignments_for_tracks(&conn, &[first_track, first_track, second_track])
            .unwrap();
    let count = |id| {
        assignments
            .iter()
            .find(|assignment| assignment.id == id)
            .unwrap()
            .assigned_count
    };
    assert_eq!(count(full.id), 2);
    assert_eq!(count(partial.id), 1);
    assert_eq!(count(unused.id), 0);
}
