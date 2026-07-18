import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockTracks } from '$lib/test-helpers';
import type { Track } from '$lib/types';
const { player, startPlayingTrack, batchAddToPlaylist } = vi.hoisted(() => ({
  player: { playQueue: [] as Track[] },
  startPlayingTrack: vi.fn(),
  batchAddToPlaylist: vi.fn(),
}));
vi.mock('$lib/state/playerState.svelte', () => ({ getPlayerState: () => player }));
vi.mock('$lib/logic/playback-actions', () => ({
  startPlayingTrack: (...args: unknown[]) => startPlayingTrack(...args),
}));
vi.mock('$lib/api/playlist', () => ({
  batchAddToPlaylist: (...args: unknown[]) => batchAddToPlaylist(...args),
}));
import {
  playCollectionTrack,
  playAll,
  shuffleAll,
  addAllToQueue,
  addAllToPlaylist,
} from './collection-actions';

describe('collection actions', () => {
  const tracks = createMockTracks(3);
  beforeEach(() => {
    player.playQueue = [];
    startPlayingTrack.mockReset();
    batchAddToPlaylist.mockReset();
  });
  it('plays a selected track with the source order', async () => {
    await playCollectionTrack(tracks[1], tracks);
    expect(startPlayingTrack).toHaveBeenCalledWith(tracks[1], tracks);
  });
  it('plays all from the first track', async () => {
    await playAll(tracks);
    expect(startPlayingTrack).toHaveBeenCalledWith(tracks[0], tracks);
  });
  it('shuffles a copy before playback', async () => {
    await shuffleAll(tracks, () => 0);
    expect(startPlayingTrack.mock.calls[0][1].map((t: { id: number }) => t.id)).toEqual([2, 3, 1]);
    expect(tracks.map((t) => t.id)).toEqual([1, 2, 3]);
  });
  it('appends tracks to the current queue', () => {
    player.playQueue = [tracks[0]];
    addAllToQueue(tracks.slice(1));
    expect(player.playQueue.map((t) => t.id)).toEqual([1, 2, 3]);
  });
  it('batch-adds tracks to a playlist', async () => {
    await addAllToPlaylist(8, tracks);
    expect(batchAddToPlaylist).toHaveBeenCalledWith(8, [1, 2, 3]);
  });
});
