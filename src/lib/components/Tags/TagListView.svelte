<script lang="ts">
  import type { TagSummary } from '$lib/types';
  import { getPlaylistState } from '$lib/state/playlistState.svelte';
  import * as tagApi from '$lib/api/tag';
  import { notifyCritical } from '$lib/logic/error-handler';
  import { askConfirmation, askText } from '$lib/state/dialogState.svelte';

  const playlistState = getPlaylistState();
  let tags = $state<TagSummary[]>([]),
    search = $state(''),
    newName = $state(''),
    sort = $state<'name' | 'count'>('name');
  let filtered = $derived(
    [...tags.filter((tag) => tag.name.toLowerCase().includes(search.trim().toLowerCase()))].sort(
      (a, b) => (sort === 'name' ? a.name.localeCompare(b.name) : b.track_count - a.track_count),
    ),
  );
  async function reload() {
    tags = await tagApi.getAllTags();
  }
  async function create() {
    const name = newName.trim();
    if (!name) return;
    try {
      await tagApi.createTag(name);
      newName = '';
      await reload();
    } catch (err) {
      notifyCritical('Create tag', err);
    }
  }
  async function clearEmpty() {
    if (
      !(await askConfirmation({
        title: '清理空 Tag？',
        message: '將刪除目前沒有套用至任何曲目的 Tag。',
        confirmLabel: '清理',
        danger: true,
        destructive: true,
      }))
    )
      return;
    try {
      await tagApi.deleteEmptyTags();
      await reload();
    } catch (err) {
      notifyCritical('Delete empty tags', err);
    }
  }
  async function rename(tag: TagSummary) {
    const name = (
      await askText({ title: '重新命名 Tag', value: tag.name, confirmLabel: '重新命名' })
    )?.trim();
    if (!name || name === tag.name) return;
    try {
      await tagApi.renameTag(tag.id, name);
      await reload();
    } catch (err) {
      notifyCritical('Rename tag', err);
    }
  }
  async function remove(tag: TagSummary) {
    if (
      !(await askConfirmation({
        title: `刪除 Tag「${tag.name}」？`,
        message: 'Tag 將從所有曲目移除，但曲目本身不會被刪除。',
        confirmLabel: '刪除',
        danger: true,
        destructive: true,
      }))
    )
      return;
    try {
      await tagApi.deleteTag(tag.id);
      await reload();
    } catch (err) {
      notifyCritical('Delete tag', err);
    }
  }
  async function merge(tag: TagSummary) {
    const targetName = await askText({
      title: '合併 Tag',
      message: `請輸入要接收「${tag.name}」曲目的 Tag 名稱。`,
      placeholder: '目標 Tag 名稱',
      confirmLabel: '合併',
    });
    const target = tags.find(
      (item) => item.name.toLowerCase() === targetName?.trim().toLowerCase(),
    );
    if (!target || target.id === tag.id) return;
    try {
      await tagApi.mergeTags(tag.id, target.id);
      await reload();
    } catch (err) {
      notifyCritical('Merge tags', err);
    }
  }
  $effect(() => {
    reload().catch((err) => notifyCritical('Load tags', err));
  });
</script>

<div class="view">
  <header>
    <h2>Tags</h2>
    <div class="tools">
      <input placeholder="搜尋 Tags..." bind:value={search} /><select bind:value={sort}
        ><option value="name">依名稱</option><option value="count">依曲目數</option></select
      >
    </div>
  </header>
  <form
    onsubmit={(e) => {
      e.preventDefault();
      create();
    }}
  >
    <input placeholder="新增 Tag" bind:value={newName} /><button>新增</button><button
      type="button"
      onclick={clearEmpty}>清理空 Tag</button
    >
  </form>
  <div class="list">
    {#if filtered.length === 0}<p class="empty">
        尚無 Tag
      </p>{:else}{#each filtered as tag (tag.id)}<div class="tag-row">
          <button
            class="open"
            onclick={() =>
              (playlistState.activeView = { kind: 'tag-detail', tagId: tag.id, tagName: tag.name })}
            ><strong>{tag.name}</strong><span>{tag.track_count} 首曲目</span></button
          ><button title="重新命名" onclick={() => rename(tag)}>✎</button><button
            title="合併"
            onclick={() => merge(tag)}>⇢</button
          ><button title="刪除" aria-label="刪除" onclick={() => remove(tag)}>×</button>
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
  .tools,
  form,
  .tag-row,
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

  input,
  select {
    background: #16213e;
    border: 1px solid #3a3a5a;
    border-radius: 5px;
    color: #eee;
    padding: 7px 10px;
  }

  form {
    margin: 16px 0;
  }

  form input {
    width: 260px;
  }

  .list {
    overflow: auto;
  }

  .tag-row {
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
