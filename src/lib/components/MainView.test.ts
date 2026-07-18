import { render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn((command: string) => {
    if (command === 'get_library_folders') return Promise.resolve([]);
    return Promise.resolve([]);
  }),
}));
vi.mock('$lib/logic/error-handler', () => ({
  notifyCritical: vi.fn(),
  warnNonCritical: vi.fn(),
  notifyImportResult: vi.fn(),
}));
import MainView from './MainView.svelte';
import { getPlaylistState } from '$lib/state/playlistState.svelte';

const playlistState = getPlaylistState();

describe('MainView routing', () => {
  it('設定狀態會顯示設定頁，而不是落入空白播放清單', async () => {
    playlistState.activeView = { kind: 'settings' };
    const view = render(MainView);
    expect(await screen.findByRole('heading', { name: '設定' })).toBeTruthy();
    expect(screen.getByRole('button', { name: '新增資料夾' })).toBeTruthy();
    view.unmount();
    playlistState.activeView = { kind: 'library' };
  });
});
