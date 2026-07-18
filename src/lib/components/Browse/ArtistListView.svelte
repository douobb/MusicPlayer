<script lang="ts">
  import type { ArtistSummary } from '$lib/types';
  import { getPlaylistState } from '$lib/state/playlistState.svelte';
  import * as libraryApi from '$lib/api/library';
  import { notifyCritical } from '$lib/logic/error-handler';
  import { askConfirmation, askText } from '$lib/state/dialogState.svelte';
  const playlistState = getPlaylistState();
  let artists = $state<ArtistSummary[]>([]),
    searchQuery = $state(''),
    newName = $state(''),
    isLoading = $state(true);
  let filtered = $derived(
    artists.filter((a) => a.name.toLowerCase().includes(searchQuery.trim().toLowerCase())),
  );
  async function reload() {
    artists = await libraryApi.getAllArtists();
  }
  async function create() {
    const name = newName.trim();
    if (!name) return;
    try {
      await libraryApi.createArtist(name);
      newName = '';
      await reload();
    } catch (err) {
      notifyCritical('Create artist', err);
    }
  }
  async function rename(artist: ArtistSummary) {
    const name = (
      await askText({ title: '重新命名 Artist', value: artist.name, confirmLabel: '重新命名' })
    )?.trim();
    if (!name || name === artist.name) return;
    try {
      await libraryApi.renameArtist(artist.id, name);
      await reload();
    } catch (err) {
      notifyCritical('Rename artist', err);
    }
  }
  async function merge(artist: ArtistSummary) {
    const targetName = (
      await askText({
        title: '合併 Artist',
        message: `請輸入要接收「${artist.name}」資料的 Artist 名稱。`,
        placeholder: '目標 Artist 名稱',
        confirmLabel: '合併',
      })
    )
      ?.trim()
      .toLowerCase();
    const target = artists.find((a) => a.name.toLowerCase() === targetName);
    if (!target || target.id === artist.id) return;
    try {
      await libraryApi.mergeArtists(artist.id, target.id);
      await reload();
    } catch (err) {
      notifyCritical('Merge artists', err);
    }
  }
  async function cleanup() {
    if (
      !(await askConfirmation({
        title: '清理未使用的 Artist？',
        message: '將刪除目前沒有任何曲目關聯的 Artist。',
        confirmLabel: '清理',
        danger: true,
        destructive: true,
      }))
    )
      return;
    try {
      await libraryApi.deleteUnusedArtists();
      await reload();
    } catch (err) {
      notifyCritical('Cleanup artists', err);
    }
  }
  $effect(() => {
    reload()
      .catch((err) => notifyCritical('Load artists', err))
      .finally(() => (isLoading = false));
  });
</script>

<div class="view">
  <header>
    <h2>Artists</h2>
    <input placeholder="搜尋 Artists..." bind:value={searchQuery} />
  </header>
  <form
    onsubmit={(e) => {
      e.preventDefault();
      create();
    }}
  >
    <input placeholder="新增 Artist" bind:value={newName} /><button>新增</button><button
      type="button"
      onclick={cleanup}>清理未使用 Artist</button
    >
  </form>
  <div class="list">
    {#if isLoading}<p class="empty">載入中…</p>{:else if filtered.length === 0}<p class="empty">
        尚無 Artist
      </p>{:else}{#each filtered as artist (artist.id)}<div class="artist">
          <button
            class="open"
            onclick={() =>
              (playlistState.activeView = {
                kind: 'artist-detail',
                artistId: artist.id,
                artistName: artist.name,
              })}
            ><strong>{artist.name}</strong><span
              >{artist.performer_track_count} 演唱 · {artist.original_track_count} 原唱</span
            ></button
          ><button title="重新命名" onclick={() => rename(artist)}>✎</button><button
            title="合併"
            onclick={() => merge(artist)}>⇢</button
          >
        </div>{/each}{/if}
  </div>
</div>

<style>
  .view {
    height: 100%;
    display: flex;
    flex-direction: column;
    padding: 20px;
    color: #eee;
  }

  header,
  form,
  .artist,
  .open {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  header {
    justify-content: space-between;
  }

  h2 {
    margin: 0;
  }

  form {
    margin: 16px 0;
  }

  .list {
    overflow: auto;
  }

  .artist {
    border-bottom: 1px solid #262640;
    padding: 6px;
  }

  .open {
    flex: 1;
    justify-content: space-between;
    background: transparent;
  }

  .open span {
    color: #888;
  }

  input {
    background: #16213e;
    border: 1px solid #3a3a5a;
    border-radius: 5px;
    color: #eee;
    padding: 7px 10px;
  }

  button {
    background: #2a2a4a;
    border: 0;
    border-radius: 5px;
    color: #ddd;
    padding: 7px 11px;
    cursor: pointer;
  }

  .empty {
    text-align: center;
    color: #777;
    margin-top: 80px;
  }
</style>
