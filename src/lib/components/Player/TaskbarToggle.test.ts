import { fireEvent, render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const taskbar = vi.hoisted(() => ({
  supported: true,
  enabled: false,
  busy: false,
  message: '工作列播放器已關閉',
  setEnabled: vi.fn(),
}));
vi.mock('$lib/state/taskbarState.svelte', () => ({
  getTaskbarState: () => taskbar,
}));
vi.mock('$lib/logic/error-handler', () => ({ warnNonCritical: vi.fn() }));
import TaskbarToggle from './TaskbarToggle.svelte';

describe('TaskbarToggle', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    taskbar.supported = true;
    taskbar.enabled = false;
    taskbar.busy = false;
    taskbar.setEnabled.mockResolvedValue(undefined);
  });

  it('主播放器可直接開啟工作列播放器', async () => {
    render(TaskbarToggle);
    await fireEvent.click(screen.getByRole('button', { name: '開啟工作列播放器' }));
    expect(taskbar.setEnabled).toHaveBeenCalledWith(true);
  });

  it('非 Windows 平台停用開關', () => {
    taskbar.supported = false;
    render(TaskbarToggle);
    expect(
      (screen.getByRole('button', { name: '開啟工作列播放器' }) as HTMLButtonElement).disabled,
    ).toBe(true);
  });
});
