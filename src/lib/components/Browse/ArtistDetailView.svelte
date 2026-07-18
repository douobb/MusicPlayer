<script lang="ts">
  import type { ArtistRole, Track, TrackDetails } from '$lib/types';
  import TrackList from '../Library/TrackList.svelte';
  import TrackPropertiesDialog from '../Library/TrackPropertiesDialog.svelte';
  import CollectionToolbar from '../Common/CollectionToolbar.svelte';
  import StatusBar from '../Library/StatusBar.svelte';
  import { getPlaylistState } from '$lib/state/playlistState.svelte';
  import { getPlayerState } from '$lib/state/playerState.svelte';
  import { getLibraryState } from '$lib/state/libraryState.svelte';
  import * as libraryApi from '$lib/api/library';
  import { playCollectionTrack } from '$lib/logic/collection-actions';
  import { optimisticRemove } from '$lib/logic/track-actions';
  import { notifyCritical } from '$lib/logic/error-handler';

  let { artistId, artistName }: { artistId: number; artistName: string } = $props();
  const playlistState = getPlaylistState(),
    player = getPlayerState(),
    library = getLibraryState();
  let tracks = $state<Track[]>([]),
    isLoading = $state(true),
    role = $state<'all' | ArtistRole>('all');
  let showProperties = $state(false),
    propertiesDetails = $state<TrackDetails | null>(null);
  async function load() {
    isLoading = true;
    try {
      tracks = await libraryApi.getTracksByArtist(artistId, role === 'all' ? undefined : role);
    } catch (err) {
      notifyCritical('Load artist tracks', err);
    } finally {
      isLoading = false;
    }
  }
  async function handleRemove(items: Track[]) {
    await optimisticRemove(items, {
      getLocalTracks: () => tracks,
      setLocalTracks: (v) => (tracks = v),
    });
  }
  async function handleProperties(track: Track) {
    try {
      propertiesDetails = await libraryApi.getTrackDetails(track.id);
      showProperties = true;
    } catch (err) {
      notifyCritical('Get track details', err);
    }
  }
  async function handleSaveMetadata(update: {
    title: string;
    performers: string[];
    originalPerformers: string[];
  }) {
    if (!propertiesDetails) return;
    try {
      const updated = await libraryApi.updateTrackMetadata(
        propertiesDetails.id,
        update.title,
        update.performers,
        update.originalPerformers,
      );
      tracks = tracks.map((t) => (t.id === updated.id ? updated : t));
      library.allTracks = library.allTracks.map((t) => (t.id === updated.id ? updated : t));
      propertiesDetails = {
        ...propertiesDetails,
        title: updated.title,
        performers: updated.performers,
        original_performers: updated.original_performers,
      };
      if (player.currentTrack?.id === updated.id) player.currentTrack = updated;
    } catch (err) {
      notifyCritical('Update metadata', err);
    }
  }
  $effect(() => {
    load();
  });
</script>

<div class="view">
  <header>
    <button onclick={() => (playlistState.activeView = { kind: 'artists' })}>←</button>
    <h2>{artistName}</h2>
    <span>{tracks.length} 首</span>
  </header>
  <nav>
    <button class:active={role === 'all'} onclick={() => (role = 'all')}>全部相關作品</button
    ><button class:active={role === 'performer'} onclick={() => (role = 'performer')}
      >演唱作品</button
    ><button
      class:active={role === 'original_performer'}
      onclick={() => (role = 'original_performer')}>原唱作品</button
    >
  </nav>
  {#if isLoading}<p class="empty">載入中…</p>{:else}<CollectionToolbar {tracks} /><TrackList
      {tracks}
      currentTrackId={player.currentTrack?.id ?? null}
      onplay={(track) => playCollectionTrack(track, tracks)}
      onremove={handleRemove}
      onproperties={handleProperties}
    /><StatusBar {tracks} />{/if}
</div>
{#if showProperties && propertiesDetails}<TrackPropertiesDialog
    details={propertiesDetails}
    onclose={() => {
      showProperties = false;
      propertiesDetails = null;
    }}
    onsave={handleSaveMetadata}
  />{/if}

<style>
  .view {
    height: 100%;
    display: flex;
    flex-direction: column;
    padding: 20px;
    color: #eee;
  }

  header,
  nav {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  h2 {
    margin: 0;
  }

  header span {
    color: #888;
  }

  button {
    background: #2a2a4a;
    border: 0;
    border-radius: 5px;
    color: #ddd;
    padding: 7px 11px;
    cursor: pointer;
  }

  nav {
    margin-top: 12px;
  }

  nav button.active {
    background: #e94560;
    color: #fff;
  }

  .empty {
    text-align: center;
    color: #777;
    margin-top: 80px;
  }
</style>
