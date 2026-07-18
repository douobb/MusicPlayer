import { beforeEach, describe, expect, it, vi } from 'vitest';
const invoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...args: unknown[]) => invoke(...args) }));
import * as api from './tag';

describe('tag API', () => {
  beforeEach(() => invoke.mockReset());
  it('maps CRUD and merge arguments', async () => {
    invoke.mockResolvedValue({});
    await api.createTag('Rock');
    await api.renameTag(1, 'Jazz');
    await api.deleteTag(1);
    await api.deleteEmptyTags();
    await api.mergeTags(1, 2);
    expect(invoke.mock.calls).toEqual([
      ['create_tag', { name: 'Rock' }],
      ['rename_tag', { id: 1, name: 'Jazz' }],
      ['delete_tag', { id: 1 }],
      ['delete_empty_tags'],
      ['merge_tags', { sourceTagId: 1, targetTagId: 2 }],
    ]);
  });
  it('maps queries and batch associations', async () => {
    invoke.mockResolvedValue([]);
    await api.getAllTags();
    await api.getTagsForTrack(3);
    await api.getTagAssignmentsForTracks([3, 4]);
    await api.addTagsToTracks([3, 4], [1]);
    await api.removeTagsFromTracks([3], [1, 2]);
    await api.getTracksByTag(2);
    expect(invoke.mock.calls).toEqual([
      ['get_all_tags'],
      ['get_tags_for_track', { trackId: 3 }],
      ['get_tag_assignments_for_tracks', { trackIds: [3, 4] }],
      ['add_tags_to_tracks', { trackIds: [3, 4], tagIds: [1] }],
      ['remove_tags_from_tracks', { trackIds: [3], tagIds: [1, 2] }],
      ['get_tracks_by_tag', { tagId: 2 }],
    ]);
  });
});
