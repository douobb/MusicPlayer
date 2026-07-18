<script lang="ts">
  import type { TrackDetails, TagSummary } from '$lib/types';
  import {
    formatArtists,
    formatDuration,
    formatFileSize,
    formatSampleRate,
  } from '$lib/logic/format';
  import * as tagApi from '$lib/api/tag';
  import { notifyCritical } from '$lib/logic/error-handler';
  import { SvelteSet } from 'svelte/reactivity';

  type MetadataUpdate = { title: string; performers: string[]; originalPerformers: string[] };
  let {
    details,
    onclose,
    onsave,
  }: {
    details: TrackDetails;
    onclose: () => void;
    onsave?: (update: MetadataUpdate) => Promise<void>;
  } = $props();
  let isEditing = $state(false),
    isSaving = $state(false),
    editTitle = $state('');
  let performerNames = $state<string[]>([]),
    originalNames = $state<string[]>([]);
  let newPerformer = $state(''),
    newOriginal = $state(''),
    newTagName = $state('');
  let allTags = $state<TagSummary[]>([]);
  const selectedTagIds = new SvelteSet<number>();
  let initialTagIds = new Set<number>();

  function startEditing() {
    editTitle = details.title;
    performerNames = details.performers.map((a) => a.name);
    originalNames = details.original_performers.map((a) => a.name);
    isEditing = true;
  }
  function addCredit(role: 'performer' | 'original') {
    const value = (role === 'performer' ? newPerformer : newOriginal).trim();
    if (!value) return;
    const list = role === 'performer' ? performerNames : originalNames;
    if (!list.some((n) => n.toLowerCase() === value.toLowerCase())) {
      if (role === 'performer') performerNames = [...list, value];
      else originalNames = [...list, value];
    }
    if (role === 'performer') newPerformer = '';
    else newOriginal = '';
  }
  function removeCredit(role: 'performer' | 'original', index: number) {
    if (role === 'performer') performerNames = performerNames.filter((_, i) => i !== index);
    else originalNames = originalNames.filter((_, i) => i !== index);
  }
  function moveCredit(role: 'performer' | 'original', index: number, direction: -1 | 1) {
    const list = [...(role === 'performer' ? performerNames : originalNames)];
    const target = index + direction;
    if (target < 0 || target >= list.length) return;
    [list[index], list[target]] = [list[target], list[index]];
    if (role === 'performer') performerNames = list;
    else originalNames = list;
  }
  function replaceSelectedTags(ids: Iterable<number>) {
    selectedTagIds.clear();
    for (const id of ids) selectedTagIds.add(id);
  }
  function toggleTag(id: number) {
    if (selectedTagIds.has(id)) selectedTagIds.delete(id);
    else selectedTagIds.add(id);
  }
  async function createTag() {
    const name = newTagName.trim();
    if (!name) return;
    try {
      const tag = await tagApi.createTag(name);
      allTags = [...allTags, tag].sort((a, b) => a.name.localeCompare(b.name));
      selectedTagIds.add(tag.id);
      newTagName = '';
    } catch (err) {
      notifyCritical('Create tag', err);
    }
  }
  async function saveEditing() {
    if (!onsave || isSaving || performerNames.length === 0) return;
    isSaving = true;
    try {
      await onsave({
        title: editTitle,
        performers: performerNames,
        originalPerformers: originalNames,
      });
      const added = [...selectedTagIds].filter((id) => !initialTagIds.has(id));
      const removed = [...initialTagIds].filter((id) => !selectedTagIds.has(id));
      if (added.length) await tagApi.addTagsToTracks([details.id], added);
      if (removed.length) await tagApi.removeTagsFromTracks([details.id], removed);
      initialTagIds = new Set(selectedTagIds);
      isEditing = false;
    } finally {
      isSaving = false;
    }
  }
  $effect(() => {
    (async () => {
      try {
        const [tags, assigned] = await Promise.all([
          tagApi.getAllTags(),
          tagApi.getTagsForTrack(details.id),
        ]);
        allTags = tags;
        initialTagIds = new Set(assigned.map((t) => t.id));
        replaceSelectedTags(initialTagIds);
      } catch (err) {
        notifyCritical('Load tags', err);
      }
    })();
  });
  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      if (isEditing) isEditing = false;
      else onclose();
    }
  }
  function handleBackdrop(e: MouseEvent) {
    if (e.target === e.currentTarget) onclose();
  }
</script>

<svelte:window onkeydown={handleKeydown} />
<div class="backdrop" role="presentation" onclick={handleBackdrop}>
  <div class="dialog" role="dialog" aria-label="Track properties">
    <header>
      <h3>曲目屬性</h3>
      <button onclick={onclose} aria-label="Close">×</button>
    </header>
    <main>
      <div class="row">
        <span class="label">標題</span>{#if isEditing}<input
            class="wide"
            bind:value={editTitle}
          />{:else}<span>{details.title}</span>{/if}
      </div>
      <div class="row credits">
        <span class="label">演唱者</span>{#if isEditing}<div class="credit-editor">
            {#each performerNames as name, i (name)}<div class="chip">
                <span>{i + 1}. {name}</span><button
                  onclick={() => moveCredit('performer', i, -1)}
                  disabled={i === 0}>↑</button
                ><button
                  onclick={() => moveCredit('performer', i, 1)}
                  disabled={i === performerNames.length - 1}>↓</button
                ><button onclick={() => removeCredit('performer', i)}>×</button>
              </div>{/each}
            <div class="add">
              <input
                placeholder="新增演唱者"
                bind:value={newPerformer}
                onkeydown={(e) => e.key === 'Enter' && (e.preventDefault(), addCredit('performer'))}
              /><button onclick={() => addCredit('performer')}>新增</button>
            </div>
            {#if performerNames.length === 0}<span class="error">至少需要一位演唱者</span>{/if}
          </div>{:else}<span>{formatArtists(details.performers)}</span>{/if}
      </div>
      <div class="row credits">
        <span class="label">原唱</span>{#if isEditing}<div class="credit-editor">
            {#each originalNames as name, i (name)}<div class="chip">
                <span>{i + 1}. {name}</span><button
                  onclick={() => moveCredit('original', i, -1)}
                  disabled={i === 0}>↑</button
                ><button
                  onclick={() => moveCredit('original', i, 1)}
                  disabled={i === originalNames.length - 1}>↓</button
                ><button onclick={() => removeCredit('original', i)}>×</button>
              </div>{/each}
            <div class="add">
              <input
                placeholder="新增原唱（選填）"
                bind:value={newOriginal}
                onkeydown={(e) => e.key === 'Enter' && (e.preventDefault(), addCredit('original'))}
              /><button onclick={() => addCredit('original')}>新增</button>
            </div>
          </div>{:else}<span
            >{details.original_performers.length
              ? formatArtists(details.original_performers)
              : '—'}</span
          >{/if}
      </div>
      <div class="row credits">
        <span class="label">Tags</span>
        <div class="tags">
          {#each allTags as tag (tag.id)}<label class="tag"
              ><input
                type="checkbox"
                checked={selectedTagIds.has(tag.id)}
                disabled={!isEditing}
                onchange={() => toggleTag(tag.id)}
              />{tag.name}</label
            >{/each}{#if isEditing}<div class="add">
              <input placeholder="新增 Tag" bind:value={newTagName} /><button onclick={createTag}
                >新增</button
              >
            </div>{/if}
        </div>
      </div>
      <div class="row">
        <span class="label">時長</span><span>{formatDuration(details.duration_secs)}</span>
      </div>
      <hr />
      <div class="row"><span class="label">格式</span><span>{details.format || '—'}</span></div>
      {#if details.bitrate_kbps != null}<div class="row">
          <span class="label">位元率</span><span>{details.bitrate_kbps} kbps</span>
        </div>{/if}{#if details.sample_rate_hz != null}<div class="row">
          <span class="label">取樣率</span><span>{formatSampleRate(details.sample_rate_hz)}</span>
        </div>{/if}
      <div class="row">
        <span class="label">檔案大小</span><span>{formatFileSize(details.file_size_bytes)}</span>
      </div>
      <div class="row">
        <span class="label">路徑</span><span class="path">{details.file_path}</span>
      </div>
      {#if onsave}<footer>
          {#if isEditing}<button onclick={() => (isEditing = false)}>取消</button><button
              class="primary"
              onclick={saveEditing}
              disabled={isSaving || performerNames.length === 0}
              >{isSaving ? '儲存中…' : '儲存'}</button
            >{:else}<button onclick={startEditing}>編輯</button>{/if}
        </footer>{/if}
    </main>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 2000;
    background: rgb(0 0 0/60%);
    display: grid;
    place-items: center;
  }

  .dialog {
    width: 560px;
    max-width: 92vw;
    max-height: 86vh;
    overflow: auto;
    background: #1e1e3a;
    border: 1px solid #3a3a5a;
    border-radius: 10px;
    color: #ddd;
  }

  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 13px 20px;
    border-bottom: 1px solid #2a2a4a;
  }

  h3 {
    margin: 0;
  }

  main {
    padding: 16px 20px;
  }

  .row {
    display: flex;
    gap: 12px;
    align-items: baseline;
    margin: 9px 0;
    font-size: 13px;
  }

  .credits {
    align-items: flex-start;
  }

  .label {
    width: 78px;
    flex: none;
    text-align: right;
    color: #888;
  }

  .wide,
  .credit-editor,
  .tags {
    flex: 1;
  }

  .credit-editor,
  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .chip,
  .tag,
  .add {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .chip,
  .tag {
    background: #2a2a4a;
    border-radius: 14px;
    padding: 3px 7px;
  }

  .chip button {
    padding: 1px 4px;
  }

  .add {
    width: 100%;
  }

  input {
    background: #16213e;
    border: 1px solid #3a3a5a;
    border-radius: 4px;
    padding: 6px 8px;
    color: #eee;
  }

  .add input {
    flex: 1;
  }

  button {
    background: #2a2a4a;
    border: 0;
    border-radius: 4px;
    padding: 6px 11px;
    color: #ddd;
    cursor: pointer;
  }

  button:disabled {
    opacity: 0.4;
  }

  .primary {
    background: #e94560;
    color: white;
  }

  .error {
    color: #ff8092;
    font-size: 12px;
  }

  .path {
    font: 11px monospace;
    word-break: break-all;
  }

  hr {
    border: 0;
    border-top: 1px solid #2a2a4a;
    margin: 13px 0;
  }

  footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 16px;
    padding-top: 12px;
    border-top: 1px solid #2a2a4a;
  }
</style>
