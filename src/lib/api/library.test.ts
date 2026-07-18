import { beforeEach, describe, expect, it, vi } from 'vitest';
const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...args: unknown[]) => mockInvoke(...args) }));
import {
  addLibraryFolder,
  createArtist,
  deleteUnusedArtists,
  getAllArtists,
  getTracksByArtist,
  mergeArtists,
  removeLibraryFolder,
  rescanLibraryFolder,
  setLibraryFolderWatching,
  renameArtist,
  updateTrackMetadata,
} from './library';

describe('library API', () => {
  beforeEach(() => mockInvoke.mockReset());
  it('updates title and ordered artist credits', async () => {
    mockInvoke.mockResolvedValueOnce({});
    await updateTrackMetadata(7, 'Title', ['Artist A', 'Artist B'], ['Original']);
    expect(mockInvoke).toHaveBeenCalledWith('update_track_metadata', {
      id: 7,
      title: 'Title',
      performers: ['Artist A', 'Artist B'],
      originalPerformers: ['Original'],
    });
  });

  it('invokes artist queries with role filters', async () => {
    mockInvoke.mockResolvedValue([]);
    await getAllArtists();
    await getTracksByArtist(5, 'original_performer');
    expect(mockInvoke).toHaveBeenNthCalledWith(1, 'get_all_artists');
    expect(mockInvoke).toHaveBeenNthCalledWith(2, 'get_tracks_by_artist', {
      artistId: 5,
      role: 'original_performer',
    });
  });

  it('invokes artist management commands', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await createArtist('Artist A');
    await renameArtist(1, 'Artist B');
    await mergeArtists(1, 2);
    await deleteUnusedArtists();
    expect(mockInvoke).toHaveBeenNthCalledWith(1, 'create_artist', { name: 'Artist A' });
    expect(mockInvoke).toHaveBeenNthCalledWith(2, 'rename_artist', { id: 1, name: 'Artist B' });
    expect(mockInvoke).toHaveBeenNthCalledWith(3, 'merge_artists', { sourceId: 1, targetId: 2 });
    expect(mockInvoke).toHaveBeenNthCalledWith(4, 'delete_unused_artists');
  });

  it('invokes library folder management commands', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await addLibraryFolder('C:/Music');
    await rescanLibraryFolder(3);
    await setLibraryFolderWatching(3, false);
    await removeLibraryFolder(3, true);
    expect(mockInvoke).toHaveBeenNthCalledWith(1, 'add_library_folder', { folderPath: 'C:/Music' });
    expect(mockInvoke).toHaveBeenNthCalledWith(2, 'rescan_library_folder', { folderId: 3 });
    expect(mockInvoke).toHaveBeenNthCalledWith(3, 'set_library_folder_watching', {
      folderId: 3,
      enabled: false,
    });
    expect(mockInvoke).toHaveBeenNthCalledWith(4, 'remove_library_folder', {
      folderId: 3,
      removeTracks: true,
    });
  });
});
