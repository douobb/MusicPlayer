<script lang="ts">
  import type { Track } from '$lib/types';
  import TrackList from '$lib/components/Library/TrackList.svelte';
  import StatusBar from '$lib/components/Library/StatusBar.svelte';
  import CollectionToolbar from '$lib/components/Common/CollectionToolbar.svelte';
  import { getPlaylistState } from '$lib/state/playlistState.svelte';
  import { getPlayerState } from '$lib/state/playerState.svelte';
  import * as tagApi from '$lib/api/tag';
  import { playCollectionTrack } from '$lib/logic/collection-actions';
  import { notifyCritical } from '$lib/logic/error-handler';

  let { tagId, tagName }: { tagId: number; tagName: string } = $props();
  const playlistState = getPlaylistState(),
    player = getPlayerState();
  let tracks = $state<Track[]>([]),
    loading = $state(true);
  async function load() {
    try {
      tracks = await tagApi.getTracksByTag(tagId);
    } catch (err) {
      notifyCritical('Load tag tracks', err);
    } finally {
      loading = false;
    }
  }
  async function remove(selected: Track[]) {
    try {
      await tagApi.removeTagsFromTracks(
        selected.map((t) => t.id),
        [tagId],
      );
      await load();
    } catch (err) {
      notifyCritical('Remove tag from tracks', err);
    }
  }
  $effect(() => {
    load();
  });
</script>

<div class="view">
  <header>
    <button onclick={() => (playlistState.activeView = { kind: 'tags' })}>←</button>
    <h2>{tagName}</h2>
    <span>{tracks.length} 首曲目</span>
  </header>
  <CollectionToolbar {tracks} />
  {#if loading}<p class="empty">載入中...</p>{:else}<TrackList
      {tracks}
      currentTrackId={player.currentTrack?.id ?? null}
      onplay={(track) => playCollectionTrack(track, tracks)}
      onremove={remove}
    /><StatusBar {tracks} />{/if}
</div>

<style>
  .view {
    height: 100%;
    display: flex;
    flex-direction: column;
    padding: 20px;
    color: #eee;
  }

  header {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  header h2 {
    margin: 0;
  }

  header span {
    color: #888;
  }

  button {
    background: #2a2a4a;
    border: 1px solid #3a3a5a;
    border-radius: 5px;
    color: #eee;
    padding: 7px 11px;
    cursor: pointer;
  }

  .empty {
    text-align: center;
    color: #777;
    margin-top: 80px;
  }
</style>
