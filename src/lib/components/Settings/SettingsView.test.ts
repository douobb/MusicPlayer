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
const taskbarState = vi.hoisted(() => ({
  supported: true,
  enabled: false,
  running: false,
  visible: false,
  mode: null as 'embedded' | 'docked' | 'unavailable' | null,
  message: '工作列播放器已關閉',
  preferenceMode: 'auto' as 'auto' | 'docked',
  offsetX: 0,
  showTitleMarquee: true,
  showProgress: true,
  hideInMiniPlayer: true,
  busy: false,
  initialize: vi.fn(),
  setEnabled: vi.fn(),
  setMode: vi.fn(),
  setOffset: vi.fn(),
  setDisplayOptions: vi.fn(),
  setMiniModeBehavior: vi.fn(),
}));
vi.mock('$lib/api/library', () => api);
vi.mock('$lib/state/taskbarState.svelte', () => ({
  getTaskbarState: () => taskbarState,
}));
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
    taskbarState.supported = true;
    taskbarState.enabled = false;
    taskbarState.running = false;
    taskbarState.visible = false;
    taskbarState.mode = null;
    taskbarState.message = '工作列播放器已關閉';
    taskbarState.preferenceMode = 'auto';
    taskbarState.offsetX = 0;
    taskbarState.showTitleMarquee = true;
    taskbarState.showProgress = true;
    taskbarState.hideInMiniPlayer = true;
    taskbarState.busy = false;
    taskbarState.initialize.mockResolvedValue(undefined);
    taskbarState.setEnabled.mockResolvedValue(undefined);
    taskbarState.setMode.mockResolvedValue(undefined);
    taskbarState.setOffset.mockResolvedValue(undefined);
    taskbarState.setDisplayOptions.mockResolvedValue(undefined);
    taskbarState.setMiniModeBehavior.mockResolvedValue(undefined);
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

  it('可在 Windows 整合設定啟用工作列播放器', async () => {
    render(SettingsView);
    await fireEvent.click(screen.getByRole('button', { name: 'Windows 整合' }));
    const checkbox = screen.getByRole('checkbox', { name: /啟用工作列播放器/ });
    expect((checkbox as HTMLInputElement).checked).toBe(false);
    await fireEvent.click(checkbox);
    expect(taskbarState.setEnabled).toHaveBeenCalledWith(true);
    expect(screen.getByRole('status', { name: '工作列播放器狀態' }).textContent).toContain(
      '工作列播放器已關閉',
    );

    await fireEvent.change(screen.getByRole('combobox', { name: '工作列播放器執行模式' }), {
      target: { value: 'docked' },
    });
    expect(taskbarState.setMode).toHaveBeenCalledWith('docked');
    expect(
      screen.getByText('僅支援 Windows 10／11 的主螢幕水平工作列；其他作業系統不會啟動 helper。'),
    ).toBeTruthy();
    expect(screen.getByRole('option', { name: '嵌入工作列（建議）' })).toBeTruthy();
    expect(screen.getByRole('status', { name: '目前工作列播放器水平偏移' }).textContent).toBe(
      '0 px',
    );
    await fireEvent.click(screen.getByRole('button', { name: '工作列播放器向左移動' }));
    expect(taskbarState.setOffset).toHaveBeenCalledWith(-10);
    expect(
      (
        screen.getByRole('button', {
          name: '工作列播放器向右移動',
        }) as HTMLButtonElement
      ).disabled,
    ).toBe(true);
    const marquee = screen.getByRole('checkbox', { name: '標題動態滾動' });
    const progress = screen.getByRole('checkbox', { name: '播放進度條' });
    expect((marquee as HTMLInputElement).checked).toBe(true);
    expect((progress as HTMLInputElement).checked).toBe(true);
    await fireEvent.click(marquee);
    expect(taskbarState.setDisplayOptions).toHaveBeenCalledWith(false, true);
    await fireEvent.click(progress);
    expect(taskbarState.setDisplayOptions).toHaveBeenCalledWith(true, false);
    const miniCoordination = screen.getByRole('checkbox', {
      name: 'Mini Player 開啟時隱藏工作列播放器',
    });
    expect((miniCoordination as HTMLInputElement).checked).toBe(true);
    await fireEvent.click(miniCoordination);
    expect(taskbarState.setMiniModeBehavior).toHaveBeenCalledWith(false);
  });
});
