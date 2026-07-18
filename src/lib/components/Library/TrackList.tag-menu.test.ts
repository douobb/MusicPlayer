import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockPlaylist, createMockTrack } from '$lib/test-helpers';
import { getPlaylistState } from '$lib/state/playlistState.svelte';

const tagApi = vi.hoisted(() => ({
  getTagAssignmentsForTracks: vi.fn(),
  addTagsToTracks: vi.fn(),
  removeTagsFromTracks: vi.fn(),
}));
vi.mock('$lib/api/tag', () => tagApi);
const playlistApi = vi.hoisted(() => ({ batchAddToPlaylist: vi.fn() }));
vi.mock('$lib/api/playlist', () => playlistApi);
vi.mock('$lib/logic/error-handler', () => ({ warnNonCritical: vi.fn() }));

import TrackList from './TrackList.svelte';

class TestResizeObserver {
  constructor(private callback: ResizeObserverCallback) {}

  observe(target: Element) {
    this.callback(
      [{ target, contentRect: { height: 400 } } as ResizeObserverEntry],
      this as unknown as ResizeObserver,
    );
  }

  disconnect() {}
  unobserve() {}
}

describe('TrackList Tag 子選單', () => {
  const originalResizeObserver = globalThis.ResizeObserver;
  const tracks = [
    createMockTrack({ id: 1, title: 'Song 1' }),
    createMockTrack({ id: 2, title: 'Song 2' }),
  ];

  beforeEach(() => {
    vi.clearAllMocks();
    globalThis.ResizeObserver = TestResizeObserver as unknown as typeof ResizeObserver;
    getPlaylistState().playlists = [];
    tagApi.getTagAssignmentsForTracks.mockResolvedValue([
      { id: 1, name: 'Full', assigned_count: 2 },
      { id: 2, name: 'Partial', assigned_count: 1 },
      { id: 3, name: 'None', assigned_count: 0 },
    ]);
  });

  afterEach(() => {
    globalThis.ResizeObserver = originalResizeObserver;
  });

  it('以可搜尋子選單區分可新增與可移除的 Tag', async () => {
    render(TrackList, {
      tracks,
      onplay: vi.fn(),
    });

    const first = await screen.findByText('Song 1');
    const second = screen.getByText('Song 2');
    await fireEvent.click(first);
    await fireEvent.click(second, { ctrlKey: true });
    await fireEvent.contextMenu(second, { clientX: 100, clientY: 100 });

    expect(screen.getByRole('menuitem', { name: /新增 Tag/ })).toBeTruthy();
    expect(screen.queryByText('Full')).toBeNull();
    await fireEvent.click(screen.getByRole('menuitem', { name: /新增 Tag/ }));
    await waitFor(() => expect(tagApi.getTagAssignmentsForTracks).toHaveBeenCalledWith([1, 2]));

    expect(screen.queryByText('Full')).toBeNull();
    expect(screen.getByText('Partial')).toBeTruthy();
    expect(screen.getByText('部分')).toBeTruthy();
    expect(screen.getByText('None')).toBeTruthy();

    await fireEvent.input(screen.getByRole('textbox', { name: '搜尋選單項目' }), {
      target: { value: 'none' },
    });
    expect(screen.queryByText('Partial')).toBeNull();
    expect(screen.getByText('None')).toBeTruthy();

    await fireEvent.click(screen.getByRole('button', { name: '返回右鍵選單' }));
    await fireEvent.click(screen.getByRole('menuitem', { name: /移除 Tag/ }));
    expect(await screen.findByText('Full')).toBeTruthy();
    expect(screen.getByText('Partial')).toBeTruthy();
    expect(screen.queryByText('None')).toBeNull();
    expect(document.querySelector('.submenu-list')).toBeTruthy();
  });
  it('播放清單也改用可搜尋的第二層選單', async () => {
    getPlaylistState().playlists = Array.from({ length: 20 }, (_, index) =>
      createMockPlaylist({ id: index + 1, name: `Playlist ${index + 1}` }),
    );
    render(TrackList, { tracks: [tracks[0]], onplay: vi.fn() });

    const track = await screen.findByText('Song 1');
    await fireEvent.contextMenu(track, { clientX: 100, clientY: 100 });
    expect(screen.queryByText('Playlist 20')).toBeNull();
    await fireEvent.click(screen.getByRole('menuitem', { name: /加入播放清單/ }));

    const search = screen.getByRole('textbox', { name: '搜尋選單項目' });
    await fireEvent.input(search, { target: { value: 'Playlist 20' } });
    expect(screen.queryByText('Playlist 1')).toBeNull();
    await fireEvent.click(screen.getByRole('menuitem', { name: /Playlist 20/ }));
    expect(playlistApi.batchAddToPlaylist).toHaveBeenCalledWith(20, [1]);
  });
});
