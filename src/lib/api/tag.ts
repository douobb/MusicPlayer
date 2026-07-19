import { invoke } from '@tauri-apps/api/core';
import type { TagAssignment, TagStatistics, TagSummary, Track } from '$lib/types';

export async function createTag(name: string): Promise<TagSummary> {
  return invoke('create_tag', { name });
}

export async function renameTag(id: number, name: string): Promise<TagSummary> {
  return invoke('rename_tag', { id, name });
}

export async function deleteTag(id: number): Promise<void> {
  return invoke('delete_tag', { id });
}

export async function deleteEmptyTags(): Promise<number> {
  return invoke('delete_empty_tags');
}

export async function mergeTags(sourceTagId: number, targetTagId: number): Promise<TagSummary> {
  return invoke('merge_tags', { sourceTagId, targetTagId });
}

export async function getAllTags(): Promise<TagSummary[]> {
  return invoke('get_all_tags');
}

export async function getTagStatistics(): Promise<TagStatistics> {
  return invoke('get_tag_statistics');
}

export async function getTagsForTrack(trackId: number): Promise<TagSummary[]> {
  return invoke('get_tags_for_track', { trackId });
}

export async function getTagAssignmentsForTracks(trackIds: number[]): Promise<TagAssignment[]> {
  return invoke('get_tag_assignments_for_tracks', { trackIds });
}
export async function addTagsToTracks(trackIds: number[], tagIds: number[]): Promise<void> {
  return invoke('add_tags_to_tracks', { trackIds, tagIds });
}

export async function removeTagsFromTracks(trackIds: number[], tagIds: number[]): Promise<void> {
  return invoke('remove_tags_from_tracks', { trackIds, tagIds });
}

export async function getTracksByTag(tagId: number): Promise<Track[]> {
  return invoke('get_tracks_by_tag', { tagId });
}
