<script lang="ts">
  import type { Track } from '$lib/types';
  import { getPlaylistState } from '$lib/state/playlistState.svelte';
  import {
    playAll,
    shuffleAll,
    addAllToQueue,
    addAllToPlaylist,
  } from '$lib/logic/collection-actions';
  import { notifyCritical } from '$lib/logic/error-handler';
  let { tracks }: { tracks: Track[] } = $props();
  const playlistState = getPlaylistState();
  let playlistId = $state<number | null>(null);
  async function addPlaylist() {
    if (playlistId === null) return;
    try {
      await addAllToPlaylist(playlistId, tracks);
    } catch (err) {
      notifyCritical('Add to playlist', err);
    }
  }
</script>

<div class="actions">
  <button onclick={() => playAll(tracks)} disabled={!tracks.length}>播放全部</button><button
    onclick={() => shuffleAll(tracks)}
    disabled={!tracks.length}>隨機播放</button
  ><button onclick={() => addAllToQueue(tracks)} disabled={!tracks.length}>加入佇列</button><select
    bind:value={playlistId}
    ><option value={null}>選擇播放清單</option>{#each playlistState.playlists as pl (pl.id)}<option
        value={pl.id}>{pl.name}</option
      >{/each}</select
  ><button onclick={addPlaylist} disabled={playlistId === null || !tracks.length}
    >加入播放清單</button
  >
</div>

<style>
  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    margin: 12px 0;
  }

  button,
  select {
    background: #2a2a4a;
    border: 1px solid #3a3a5a;
    border-radius: 5px;
    color: #eee;
    padding: 6px 10px;
  }

  button {
    cursor: pointer;
  }

  button:disabled {
    opacity: 0.5;
  }
</style>
