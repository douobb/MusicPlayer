import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { TrackDetails } from '$lib/types';

const tagApi = vi.hoisted(() => ({
  getAllTags: vi.fn(),
  getTagsForTrack: vi.fn(),
  createTag: vi.fn(),
  addTagsToTracks: vi.fn(),
  removeTagsFromTracks: vi.fn(),
}));

vi.mock('$lib/api/tag', () => tagApi);
vi.mock('$lib/logic/error-handler', () => ({ notifyCritical: vi.fn() }));

import TrackPropertiesDialog from './TrackPropertiesDialog.svelte';

const details: TrackDetails = {
  id: 1,
  file_path: 'C:/music/test.wav',
  title: 'Test Song',
  performers: [
    { artist_id: 1, name: 'Singer A', position: 0 },
    { artist_id: 2, name: 'Singer B', position: 1 },
  ],
  original_performers: [{ artist_id: 3, name: 'Original A', position: 0 }],
  duration_secs: 123,
  file_size_bytes: 1024,
  bitrate_kbps: 320,
  sample_rate_hz: 44100,
  channels: 2,
  format: 'WAV',
  bits_per_sample: 16,
};

describe('TrackPropertiesDialog artist credits', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    tagApi.getAllTags.mockResolvedValue([]);
    tagApi.getTagsForTrack.mockResolvedValue([]);
  });

  it('新增、排序多人演唱者與原唱後，依順序送出', async () => {
    const onsave = vi.fn().mockResolvedValue(undefined);
    render(TrackPropertiesDialog, { details, onclose: vi.fn(), onsave });

    await fireEvent.click(screen.getByRole('button', { name: '編輯' }));
    const dialog = screen.getByRole('dialog');
    await fireEvent.click(within(dialog).getAllByRole('button', { name: '↓' })[0]);

    await fireEvent.input(screen.getByPlaceholderText('新增演唱者'), {
      target: { value: 'Singer C' },
    });
    await fireEvent.click(
      screen.getByPlaceholderText('新增演唱者').nextElementSibling as HTMLElement,
    );

    await fireEvent.input(screen.getByPlaceholderText('新增原唱（選填）'), {
      target: { value: 'Original B' },
    });
    await fireEvent.click(
      screen.getByPlaceholderText('新增原唱（選填）').nextElementSibling as HTMLElement,
    );
    await fireEvent.click(screen.getByRole('button', { name: '儲存' }));

    await waitFor(() =>
      expect(onsave).toHaveBeenCalledWith({
        title: 'Test Song',
        performers: ['Singer B', 'Singer A', 'Singer C'],
        originalPerformers: ['Original A', 'Original B'],
      }),
    );
  });

  it('沒有演唱者時禁止儲存', async () => {
    render(TrackPropertiesDialog, {
      details: { ...details, performers: [details.performers[0]] },
      onclose: vi.fn(),
      onsave: vi.fn(),
    });
    await fireEvent.click(screen.getByRole('button', { name: '編輯' }));
    const removeButtons = screen.getAllByRole('button', { name: '×' });
    await fireEvent.click(removeButtons[0]);
    expect(screen.getByText('至少需要一位演唱者')).toBeTruthy();
    expect((screen.getByRole('button', { name: '儲存' }) as HTMLButtonElement).disabled).toBe(true);
  });
});
