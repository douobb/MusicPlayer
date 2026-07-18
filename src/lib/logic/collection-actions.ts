import type { Track } from '$lib/types';
import { getPlayerState } from '$lib/state/playerState.svelte';
import { startPlayingTrack } from '$lib/logic/playback-actions';
import { batchAddToPlaylist } from '$lib/api/playlist';

const player = getPlayerState();

export async function playCollectionTrack(track: Track, tracks: Track[]): Promise<void> {
  await startPlayingTrack(track, tracks);
}

export async function playAll(tracks: Track[]): Promise<void> {
  if (tracks.length) await startPlayingTrack(tracks[0], tracks);
}

export async function shuffleAll(
  tracks: Track[],
  random: () => number = Math.random,
): Promise<void> {
  const shuffled = [...tracks];
  for (let i = shuffled.length - 1; i > 0; i--) {
    const j = Math.floor(random() * (i + 1));
    [shuffled[i], shuffled[j]] = [shuffled[j], shuffled[i]];
  }
  if (shuffled.length) await startPlayingTrack(shuffled[0], shuffled);
}

export function addAllToQueue(tracks: Track[]): void {
  player.playQueue = [...player.playQueue, ...tracks];
}

export async function addAllToPlaylist(playlistId: number, tracks: Track[]): Promise<void> {
  if (tracks.length)
    await batchAddToPlaylist(
      playlistId,
      tracks.map((track) => track.id),
    );
}
