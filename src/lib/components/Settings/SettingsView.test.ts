import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const api = vi.hoisted(() => ({
  getLibraryFolders: vi.fn(),
  addLibraryFolder: vi.fn(),
  rescanLibraryFolder: vi.fn(),
  rescanAllLibraryFolders: vi.fn(),
  setLibraryFolderWatching: vi.fn(),
  removeLibraryFolder: vi.fn(),
  openLibraryFolder: vi.fn(),
}));
vi.mock('$lib/api/library', () => api);
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }));
vi.mock('$lib/logic/error-handler', () => ({ notifyCritical: vi.fn() }));
import SettingsView from './SettingsView.svelte';
import { getPreferencesState } from '$lib/state/preferencesState.svelte';

const folder = {
  id: 1,
  folder_path: 'C:\\Music',
  enabled: true,
  track_count: 12,
  last_scan_at: '2026-07-18 12:00:00',
  last_error: null,
  last_added: 2,
  last_updated: 1,
  last_unchanged: 9,
  last_removed: 0,
  last_failed: 0,
};

describe('SettingsView library folders', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    api.getLibraryFolders.mockResolvedValue([folder]);
    api.rescanLibraryFolder.mockResolvedValue({
      folder_id: 1,
      added: 1,
      updated: 2,
      unchanged: 9,
      removed: 0,
      failed_files: [],
    });
    api.setLibraryFolderWatching.mockResolvedValue(undefined);
    api.removeLibraryFolder.mockResolvedValue(0);
    getPreferencesState().confirmDeletions = true;
  });

  it('顯示資料夾狀態並執行增量重新掃描', async () => {
    render(SettingsView);
    expect(await screen.findByText('C:\\Music')).toBeTruthy();
    expect(screen.getByText('12 首曲目')).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: '重新掃描' }));
    await waitFor(() => expect(api.rescanLibraryFolder).toHaveBeenCalledWith(1));
    expect(screen.getByRole('status').textContent).toContain('新增 1、更新 2、未變更 9');
  });

  it('可暫停監看，移除時明確選擇保留曲目', async () => {
    render(SettingsView);
    await screen.findByText('C:\\Music');
    await fireEvent.click(screen.getByRole('button', { name: '暫停監看' }));
    await waitFor(() => expect(api.setLibraryFolderWatching).toHaveBeenCalledWith(1, false));
    await fireEvent.click(screen.getByRole('button', { name: '移除' }));
    expect(screen.getByRole('dialog', { name: '移除媒體庫資料夾' })).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: '保留曲目' }));
    await waitFor(() => expect(api.removeLibraryFolder).toHaveBeenCalledWith(1, false));
  });
  it('可在一般設定關閉刪除確認並持久化', async () => {
    render(SettingsView);
    await fireEvent.click(screen.getByRole('button', { name: '一般' }));
    const checkbox = screen.getByRole('checkbox', { name: /刪除或移除前顯示確認/ });
    expect((checkbox as HTMLInputElement).checked).toBe(true);
    await fireEvent.click(checkbox);
    expect(getPreferencesState().confirmDeletions).toBe(false);
    expect(localStorage.getItem('musicplayer.confirm-deletions')).toBe('false');
  });
});
