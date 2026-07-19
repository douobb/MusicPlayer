import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
const { getAllTags, getTagStatistics, createTag, deleteTag } = vi.hoisted(() => ({
  getAllTags: vi.fn(),
  getTagStatistics: vi.fn(),
  createTag: vi.fn(),
  deleteTag: vi.fn(),
}));
vi.mock('$lib/api/tag', () => ({
  getAllTags,
  getTagStatistics,
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
    getTagStatistics.mockReset();
    getTagStatistics.mockResolvedValue({
      tag_count: 0,
      tagged_track_count: 0,
      untagged_track_count: 0,
      assignment_count: 0,
      average_tags_per_tagged_track: 0,
      most_used_tag: null,
    });
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
  it('顯示不重複曲目與 Tag 關聯摘要', async () => {
    getAllTags.mockResolvedValue([{ id: 1, name: 'Rock', track_count: 2 }]);
    getTagStatistics.mockResolvedValue({
      tag_count: 2,
      tagged_track_count: 3,
      untagged_track_count: 4,
      assignment_count: 5,
      average_tags_per_tagged_track: 1.666,
      most_used_tag: { id: 1, name: 'Rock', track_count: 2 },
    });

    render(TagListView);

    const summary = await screen.findByRole('region', { name: 'Tag 摘要統計' });
    expect(within(summary).getByText('已標記曲目').nextElementSibling?.textContent).toBe('3');
    expect(within(summary).getByText('未標記曲目').nextElementSibling?.textContent).toBe('4');
    expect(within(summary).getByText('Tag 關聯').nextElementSibling?.textContent).toBe('5');
    expect(within(summary).getByText('平均 Tag 數').nextElementSibling?.textContent).toBe('1.7');
    expect(within(summary).getByText('Rock · 2 首')).toBeTruthy();
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
