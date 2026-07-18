import { invoke } from '@tauri-apps/api/core';
import type {
  Track,
  TrackDetails,
  ArtistSummary,
  ArtistRole,
  ImportResult,
  LibraryFolder,
  FolderSyncResult,
} from '$lib/types';

export async function getAllTracks(): Promise<Track[]> {
  return invoke('get_all_tracks');
}

export async function getTrackCover(id: number): Promise<string | null> {
  return invoke('get_track_cover', { id });
}

export async function searchTracks(query: string): Promise<Track[]> {
  return invoke('search_tracks', { query });
}

export async function removeTrack(id: number): Promise<void> {
  return invoke('remove_track', { id });
}

export async function trashTrack(id: number): Promise<void> {
  return invoke('trash_track', { id });
}

export interface BatchTrashResult {
  succeeded_ids: number[];
  failed: { id: number; error: string }[];
}

export async function trashTracks(ids: number[]): Promise<BatchTrashResult> {
  return invoke('trash_tracks', { ids });
}

export async function removeTracks(ids: number[]): Promise<BatchTrashResult> {
  return invoke('remove_tracks', { ids });
}

export async function getTrackDetails(id: number): Promise<TrackDetails> {
  return invoke('get_track_details', { id });
}

export async function importPaths(paths: string[]): Promise<ImportResult> {
  return invoke('import_paths', { paths });
}

export async function updateTrackMetadata(
  id: number,
  title: string,
  performers: string[],
  originalPerformers: string[],
): Promise<Track> {
  return invoke('update_track_metadata', { id, title, performers, originalPerformers });
}

export async function createArtist(name: string): Promise<number> {
  return invoke('create_artist', { name });
}
export async function renameArtist(id: number, name: string): Promise<void> {
  return invoke('rename_artist', { id, name });
}
export async function mergeArtists(sourceId: number, targetId: number): Promise<void> {
  return invoke('merge_artists', { sourceId, targetId });
}
export async function deleteUnusedArtists(): Promise<number> {
  return invoke('delete_unused_artists');
}
export async function getAllArtists(): Promise<ArtistSummary[]> {
  return invoke('get_all_artists');
}

export async function getTracksByArtist(artistId: number, role?: ArtistRole): Promise<Track[]> {
  return invoke('get_tracks_by_artist', { artistId, role: role ?? null });
}

export async function getLibraryFolders(): Promise<LibraryFolder[]> {
  return invoke('get_library_folders');
}

export async function addLibraryFolder(folderPath: string): Promise<FolderSyncResult> {
  return invoke('add_library_folder', { folderPath });
}

export async function rescanLibraryFolder(folderId: number): Promise<FolderSyncResult> {
  return invoke('rescan_library_folder', { folderId });
}

export async function rescanAllLibraryFolders(): Promise<FolderSyncResult[]> {
  return invoke('rescan_all_library_folders');
}

export async function setLibraryFolderWatching(folderId: number, enabled: boolean): Promise<void> {
  return invoke('set_library_folder_watching', { folderId, enabled });
}

export async function removeLibraryFolder(
  folderId: number,
  removeTracks: boolean,
): Promise<number> {
  return invoke('remove_library_folder', { folderId, removeTracks });
}

export async function openLibraryFolder(folderId: number): Promise<void> {
  return invoke('open_library_folder', { folderId });
}
export async function incrementPlayCount(trackId: number): Promise<void> {
  return invoke('increment_play_count', { trackId });
}

export async function getMostPlayedTracks(limit?: number): Promise<Track[]> {
  return invoke('get_most_played_tracks', { limit: limit ?? 50 });
}
