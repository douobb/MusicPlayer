import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
const { getAllTags, createTag, deleteTag } = vi.hoisted(() => ({
  getAllTags: vi.fn(),
  createTag: vi.fn(),
  deleteTag: vi.fn(),
}));
vi.mock('$lib/api/tag', () => ({
  getAllTags,
  createTag,
  renameTag: vi.fn(),
  deleteTag,
  deleteEmptyTags: vi.fn(),
  mergeTags: vi.fn(),
}));
vi.mock('$lib/state/playlistState.svelte', () => ({
  getPlaylistState: () => ({ activeView: { kind: 'tags' } }),
}));
import TagListView from './TagListView.svelte';
import DialogHost from '../Common/DialogHost.svelte';
import { getPreferencesState } from '$lib/state/preferencesState.svelte';

describe('TagListView', () => {
  beforeEach(() => {
    getAllTags.mockReset();
    createTag.mockReset();
    deleteTag.mockReset();
    getPreferencesState().confirmDeletions = true;
  });
  it('shows tag counts and filters by name', async () => {
    getAllTags.mockResolvedValue([
      { id: 1, name: 'Rock', track_count: 2 },
      { id: 2, name: 'Jazz', track_count: 1 },
    ]);
    render(TagListView);
    expect(await screen.findByText('Rock')).toBeTruthy();
    await fireEvent.input(screen.getByPlaceholderText('搜尋 Tags...'), {
      target: { value: 'jazz' },
    });
    expect(screen.queryByText('Rock')).toBeNull();
    expect(screen.getByText('Jazz')).toBeTruthy();
  });
  it('creates a trimmed tag and reloads the list', async () => {
    getAllTags.mockResolvedValue([]);
    createTag.mockResolvedValue({ id: 1, name: 'Focus', track_count: 0 });
    render(TagListView);
    await fireEvent.input(screen.getByPlaceholderText('新增 Tag'), {
      target: { value: '  Focus  ' },
    });
    await fireEvent.click(screen.getByRole('button', { name: '新增' }));
    await waitFor(() => expect(createTag).toHaveBeenCalledWith('Focus'));
    expect(getAllTags).toHaveBeenCalledTimes(2);
  });
  it('刪除 Tag 前會使用自訂彈框確認', async () => {
    getAllTags.mockResolvedValue([{ id: 1, name: 'Rock', track_count: 2 }]);
    deleteTag.mockResolvedValue(undefined);
    render(DialogHost);
    render(TagListView);

    await screen.findByText('Rock');
    await fireEvent.click(screen.getByRole('button', { name: '刪除' }));
    expect(deleteTag).not.toHaveBeenCalled();
    const dialog = screen.getByRole('dialog', { name: '刪除 Tag「Rock」？' });
    await fireEvent.click(within(dialog).getByRole('button', { name: '刪除' }));
    await waitFor(() => expect(deleteTag).toHaveBeenCalledWith(1));
  });
});
