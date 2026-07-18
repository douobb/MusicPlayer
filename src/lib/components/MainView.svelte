<script lang="ts">
  import LibraryView from '$lib/components/Library/LibraryView.svelte';
  import PlaylistPanel from '$lib/components/Playlist/PlaylistPanel.svelte';
  import ArtistListView from '$lib/components/Browse/ArtistListView.svelte';
  import ArtistDetailView from '$lib/components/Browse/ArtistDetailView.svelte';
  import TagListView from '$lib/components/Tags/TagListView.svelte';
  import TagDetailView from '$lib/components/Tags/TagDetailView.svelte';
  import MostPlayedView from '$lib/components/Browse/MostPlayedView.svelte';
  import SettingsView from '$lib/components/Settings/SettingsView.svelte';
  import { getPlaylistState } from '$lib/state/playlistState.svelte';

  const playlistState = getPlaylistState();
</script>

{#if playlistState.activeView.kind === 'library'}
  <LibraryView />
{:else if playlistState.activeView.kind === 'artists'}
  <ArtistListView />
{:else if playlistState.activeView.kind === 'artist-detail'}
  <ArtistDetailView
    artistId={playlistState.activeView.artistId}
    artistName={playlistState.activeView.artistName}
  />
{:else if playlistState.activeView.kind === 'tags'}
  <TagListView />
{:else if playlistState.activeView.kind === 'tag-detail'}
  <TagDetailView
    tagId={playlistState.activeView.tagId}
    tagName={playlistState.activeView.tagName}
  />
{:else if playlistState.activeView.kind === 'most-played'}
  <MostPlayedView />
{:else if playlistState.activeView.kind === 'settings'}
  <SettingsView />
{:else if playlistState.activeView.kind === 'playlist'}
  <PlaylistPanel />
{:else}
  <div class="route-error" role="alert">無法顯示目前頁面，請返回 All Music。</div>
{/if}

<style>
  .route-error {
    display: grid;
    height: 100%;
    place-items: center;
    color: #888;
  }
</style>
