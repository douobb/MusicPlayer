<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import type { FolderSyncResult, LibraryFolder } from '$lib/types';
  import * as libraryApi from '$lib/api/library';
  import { notifyCritical } from '$lib/logic/error-handler';
  import { getPreferencesState } from '$lib/state/preferencesState.svelte';

  type Section = 'general' | 'library' | 'playback' | 'windows' | 'about';
  const preferences = getPreferencesState();
  let section = $state<Section>('library');
  let folders = $state<LibraryFolder[]>([]);
  let busyFolderId = $state<number | 'all' | 'add' | null>(null);
  let lastResult = $state<string | null>(null);
  let removingFolder = $state<LibraryFolder | null>(null);
  let isLoading = $state(true);
  let totalTracks = $derived(folders.reduce((sum, folder) => sum + folder.track_count, 0));

  async function reload() {
    folders = await libraryApi.getLibraryFolders();
  }

  function resultText(result: FolderSyncResult): string {
    return `新增 ${result.added}、更新 ${result.updated}、未變更 ${result.unchanged}、移除 ${result.removed}、失敗 ${result.failed_files.length}`;
  }

  function formatScanTime(value: string | null): string {
    if (!value) return '尚未同步';
    const parsed = new Date(`${value.replace(' ', 'T')}Z`);
    return Number.isNaN(parsed.getTime()) ? value : parsed.toLocaleString();
  }

  async function addFolder() {
    try {
      const selected = await open({ directory: true, multiple: false });
      if (!selected || Array.isArray(selected)) return;
      busyFolderId = 'add';
      const result = await libraryApi.addLibraryFolder(String(selected));
      lastResult = resultText(result);
      await reload();
    } catch (error) {
      notifyCritical('新增媒體庫資料夾', error);
    } finally {
      busyFolderId = null;
    }
  }

  async function rescan(folder: LibraryFolder) {
    busyFolderId = folder.id;
    try {
      const result = await libraryApi.rescanLibraryFolder(folder.id);
      lastResult = resultText(result);
      await reload();
    } catch (error) {
      notifyCritical('重新掃描資料夾', error);
      await reload();
    } finally {
      busyFolderId = null;
    }
  }

  async function rescanAll() {
    busyFolderId = 'all';
    try {
      const results = await libraryApi.rescanAllLibraryFolders();
      const total = results.reduce(
        (sum, result) => ({
          added: sum.added + result.added,
          updated: sum.updated + result.updated,
          unchanged: sum.unchanged + result.unchanged,
          removed: sum.removed + result.removed,
          failed: sum.failed + result.failed_files.length,
        }),
        { added: 0, updated: 0, unchanged: 0, removed: 0, failed: 0 },
      );
      lastResult = `新增 ${total.added}、更新 ${total.updated}、未變更 ${total.unchanged}、移除 ${total.removed}、失敗 ${total.failed}`;
      await reload();
    } catch (error) {
      notifyCritical('重新掃描所有資料夾', error);
    } finally {
      busyFolderId = null;
    }
  }

  async function toggleWatching(folder: LibraryFolder) {
    busyFolderId = folder.id;
    try {
      await libraryApi.setLibraryFolderWatching(folder.id, !folder.enabled);
      await reload();
    } catch (error) {
      notifyCritical(folder.enabled ? '暫停監看' : '恢復監看', error);
    } finally {
      busyFolderId = null;
    }
  }

  function requestFolderRemoval(folder: LibraryFolder) {
    if (preferences.confirmDeletions) removingFolder = folder;
    else void removeFolder(folder, false);
  }

  async function removeFolder(folder: LibraryFolder, removeTracks: boolean) {
    removingFolder = null;
    busyFolderId = folder.id;
    try {
      const removed = await libraryApi.removeLibraryFolder(folder.id, removeTracks);
      lastResult = removeTracks
        ? `已從媒體庫移除 ${removed} 首曲目；原始檔案未刪除`
        : '已停止管理資料夾，媒體庫曲目保留';
      await reload();
    } catch (error) {
      notifyCritical('移除媒體庫資料夾', error);
    } finally {
      busyFolderId = null;
    }
  }

  async function reveal(folder: LibraryFolder) {
    try {
      await libraryApi.openLibraryFolder(folder.id);
    } catch (error) {
      notifyCritical('開啟資料夾', error);
    }
  }

  $effect(() => {
    reload()
      .catch((error) => notifyCritical('載入媒體庫資料夾', error))
      .finally(() => (isLoading = false));
  });
</script>

<div class="settings-view">
  <header><h2>設定</h2></header>
  <div class="settings-layout">
    <nav aria-label="設定分類">
      <button class:active={section === 'general'} onclick={() => (section = 'general')}
        >一般</button
      >
      <button class:active={section === 'library'} onclick={() => (section = 'library')}
        >媒體庫</button
      >
      <button class:active={section === 'playback'} onclick={() => (section = 'playback')}
        >播放</button
      >
      <button class:active={section === 'windows'} onclick={() => (section = 'windows')}
        >Windows 整合</button
      >
      <button class:active={section === 'about'} onclick={() => (section = 'about')}>關於</button>
    </nav>

    <main>
      {#if section === 'library'}
        <div class="section-heading">
          <div>
            <h3>媒體庫資料夾</h3>
            <p>{folders.length} 個資料夾 · {totalTracks} 首索引曲目</p>
          </div>
          <div class="heading-actions">
            <button onclick={rescanAll} disabled={busyFolderId !== null || folders.length === 0}
              >全部重新掃描</button
            >
            <button class="primary" onclick={addFolder} disabled={busyFolderId !== null}
              >{busyFolderId === 'add' ? '新增中…' : '新增資料夾'}</button
            >
          </div>
        </div>
        <p class="description">重新掃描只處理新增、修改或移除的檔案，未變更曲目不會重新匯入。</p>
        {#if lastResult}<div class="sync-result" role="status">最近操作：{lastResult}</div>{/if}

        {#if isLoading}
          <p class="empty">正在載入…</p>
        {:else if folders.length === 0}
          <div class="empty">
            <strong>尚未加入媒體庫資料夾</strong><span
              >加入後會立即掃描，並在程式執行期間持續監看變更。</span
            >
          </div>
        {:else}
          <div class="folder-list">
            {#each folders as folder (folder.id)}
              <article class:error={folder.last_error}>
                <div class="folder-main">
                  <div class="folder-title">
                    <span class:paused={!folder.enabled}
                      >{folder.enabled ? '● 監看中' : 'Ⅱ 已暫停'}</span
                    ><strong title={folder.folder_path}>{folder.folder_path}</strong>
                  </div>
                  <div class="folder-meta">
                    <span>{folder.track_count} 首曲目</span><span
                      >最後同步：{formatScanTime(folder.last_scan_at)}</span
                    >
                  </div>
                  {#if folder.last_scan_at}<div class="summary">
                      新增 {folder.last_added} · 更新 {folder.last_updated} · 未變更 {folder.last_unchanged}
                      · 移除 {folder.last_removed} · 失敗 {folder.last_failed}
                    </div>{/if}
                  {#if folder.last_error}<div class="folder-error">
                      {folder.last_error}；索引曲目已保留。
                    </div>{/if}
                </div>
                <div class="folder-actions">
                  <button onclick={() => reveal(folder)}>開啟</button>
                  <button onclick={() => rescan(folder)} disabled={busyFolderId !== null}
                    >{busyFolderId === folder.id ? '處理中…' : '重新掃描'}</button
                  >
                  <button onclick={() => toggleWatching(folder)} disabled={busyFolderId !== null}
                    >{folder.enabled ? '暫停監看' : '恢復監看'}</button
                  >
                  <button
                    class="danger"
                    onclick={() => requestFolderRemoval(folder)}
                    disabled={busyFolderId !== null}>移除</button
                  >
                </div>
              </article>
            {/each}
          </div>
        {/if}
      {:else if section === 'general'}
        <section class="preferences-section">
          <h3>操作確認</h3>
          <label class="preference-row">
            <span
              ><strong>刪除或移除前顯示確認</strong><small
                >避免誤刪播放清單、Tag、曲目等內容；預設開啟。</small
              ></span
            >
            <input
              type="checkbox"
              checked={preferences.confirmDeletions}
              onchange={(event) => (preferences.confirmDeletions = event.currentTarget.checked)}
            />
          </label>
        </section>
      {:else if section === 'about'}
        <section class="placeholder">
          <h3>MusicPlayer</h3>
          <p>本機優先的音樂播放器，基於 twtrubiks/lyra-music 開發。</p>
        </section>
      {:else}
        <section class="placeholder">
          <h3>{section === 'playback' ? '播放設定' : 'Windows 整合'}</h3>
          <p>相關選項會隨對應功能完成後加入。</p>
        </section>
      {/if}
    </main>
  </div>
</div>

{#if removingFolder}
  <div class="backdrop" role="presentation">
    <div class="confirm" role="dialog" aria-label="移除媒體庫資料夾">
      <h3>移除資料夾？</h3>
      <p>{removingFolder.folder_path}</p>
      <p>
        這不會刪除硬碟上的原始音樂。請選擇是否同時移除媒體庫中的 {removingFolder.track_count} 首索引曲目。
      </p>
      <div class="confirm-actions">
        <button onclick={() => (removingFolder = null)}>取消</button><button
          onclick={() => removeFolder(removingFolder!, false)}>保留曲目</button
        ><button class="danger" onclick={() => removeFolder(removingFolder!, true)}
          >移除索引曲目</button
        >
      </div>
    </div>
  </div>
{/if}

<style>
  .settings-view {
    height: 100%;
    padding: 22px;
    color: #ddd;
    overflow: hidden;
  }

  header h2,
  h3 {
    margin: 0;
  }

  .settings-layout {
    display: grid;
    grid-template-columns: 150px 1fr;
    gap: 22px;
    height: calc(100% - 42px);
    margin-top: 18px;
  }

  nav {
    display: flex;
    flex-direction: column;
    gap: 4px;
    border-right: 1px solid #2a2a4a;
    padding-right: 12px;
  }

  nav button {
    text-align: left;
    background: transparent;
  }

  nav button.active {
    background: #2a2a4a;
    color: #fff;
  }

  main {
    overflow: auto;
    padding-right: 5px;
  }

  .section-heading,
  .folder-title,
  .folder-meta,
  .folder-actions,
  .heading-actions,
  .confirm-actions {
    display: flex;
    align-items: center;
    gap: 9px;
  }

  .section-heading {
    justify-content: space-between;
  }

  .section-heading p,
  .description {
    color: #888;
    margin: 5px 0;
    font-size: 13px;
  }

  button {
    border: 0;
    border-radius: 5px;
    background: #2a2a4a;
    color: #ddd;
    padding: 7px 11px;
    cursor: pointer;
  }

  button:disabled {
    opacity: 0.45;
    cursor: default;
  }

  .primary {
    background: #e94560;
    color: white;
  }

  .danger {
    color: #ff8799;
  }

  .sync-result {
    margin: 14px 0;
    padding: 9px 12px;
    border-radius: 5px;
    background: #16213e;
    font-size: 13px;
  }

  .folder-list {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin-top: 15px;
  }

  article {
    display: flex;
    justify-content: space-between;
    gap: 14px;
    padding: 14px;
    background: #17172d;
    border: 1px solid #2a2a4a;
    border-radius: 7px;
  }

  article.error {
    border-color: #8b4550;
  }

  .folder-main {
    min-width: 0;
    flex: 1;
  }

  .folder-title strong {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .folder-title span {
    color: #52c77a;
    font-size: 12px;
    flex: none;
  }

  .folder-title span.paused {
    color: #aaa;
  }

  .folder-meta,
  .summary {
    color: #888;
    font-size: 12px;
    margin-top: 8px;
  }

  .summary {
    color: #aaa;
  }

  .folder-error {
    color: #ff9bab;
    font-size: 12px;
    margin-top: 8px;
  }

  .folder-actions {
    align-self: center;
    flex-wrap: wrap;
    justify-content: flex-end;
    max-width: 270px;
  }

  .preferences-section {
    max-width: 680px;
  }

  .preference-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 20px;
    margin-top: 16px;
    padding: 15px;
    border: 1px solid #2a2a4a;
    border-radius: 7px;
    background: #17172d;
  }

  .preference-row span {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .preference-row small {
    color: #888;
  }

  .preference-row input {
    width: 18px;
    height: 18px;
    accent-color: #e94560;
  }

  .empty,
  .placeholder {
    display: flex;
    flex-direction: column;
    gap: 8px;
    color: #888;
    text-align: center;
    margin-top: 80px;
  }

  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 2100;
    display: grid;
    place-items: center;
    background: rgb(0 0 0 / 65%);
  }

  .confirm {
    width: 460px;
    max-width: 90vw;
    padding: 20px;
    border: 1px solid #3a3a5a;
    border-radius: 9px;
    background: #1e1e3a;
    color: #ddd;
  }

  .confirm p {
    color: #aaa;
    overflow-wrap: anywhere;
  }

  .confirm-actions {
    justify-content: flex-end;
    margin-top: 18px;
  }
</style>
