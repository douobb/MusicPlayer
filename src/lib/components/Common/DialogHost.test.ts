import { fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { askConfirmation, askText, getDialogState } from '$lib/state/dialogState.svelte';
import { getPreferencesState } from '$lib/state/preferencesState.svelte';
import DialogHost from './DialogHost.svelte';

describe('DialogHost', () => {
  beforeEach(() => {
    getDialogState().cancel();
    getPreferencesState().confirmDeletions = true;
  });

  afterEach(() => {
    getDialogState().cancel();
    getPreferencesState().confirmDeletions = true;
  });

  it('使用應用程式內建輸入框，不顯示開發伺服器位址', async () => {
    render(DialogHost);
    const result = askText({ title: '重新命名 Artist', value: 'Artist 1' });

    expect(await screen.findByRole('dialog', { name: '重新命名 Artist' })).toBeTruthy();
    expect(document.body.textContent).not.toContain('127.0.0.1');
    const input = screen.getByRole('textbox', { name: '重新命名 Artist' });
    await fireEvent.input(input, { target: { value: 'Artist 2' } });
    await fireEvent.click(screen.getByRole('button', { name: '確定' }));

    await expect(result).resolves.toBe('Artist 2');
  });

  it('刪除確認預設顯示，取消後不執行', async () => {
    render(DialogHost);
    const result = askConfirmation({
      title: '刪除 Tag？',
      message: '測試',
      destructive: true,
    });

    expect(await screen.findByRole('dialog', { name: '刪除 Tag？' })).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: '取消' }));
    await expect(result).resolves.toBe(false);
  });

  it('關閉刪除確認後直接採用動作', async () => {
    render(DialogHost);
    getPreferencesState().confirmDeletions = false;

    await expect(
      askConfirmation({ title: '刪除 Tag？', message: '測試', destructive: true }),
    ).resolves.toBe(true);
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(localStorage.getItem('musicplayer.confirm-deletions')).toBe('false');
  });
});
