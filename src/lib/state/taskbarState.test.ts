import { beforeEach, describe, expect, it, vi } from 'vitest';

const api = vi.hoisted(() => ({
  getTaskbarSettings: vi.fn(),
  getTaskbarStatus: vi.fn(),
  setTaskbarPlayerEnabled: vi.fn(),
  setTaskbarPlayerMode: vi.fn(),
  setTaskbarPlayerOffset: vi.fn(),
  setTaskbarPlayerDisplayOptions: vi.fn(),
  setTaskbarPlayerMiniModeBehavior: vi.fn(),
}));
vi.mock('$lib/api/taskbar', () => api);
import { applyTaskbarStatus, getTaskbarState } from './taskbarState.svelte';

describe('taskbar state', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    api.getTaskbarSettings.mockResolvedValue({
      enabled: true,
      mode: 'auto',
      offset_x: -320,
      show_title_marquee: true,
      show_progress: false,
      hide_in_mini_player: true,
    });
    api.getTaskbarStatus.mockResolvedValue({
      supported: true,
      enabled: true,
      running: true,
      visible: true,
      mode: 'embedded',
      message: '已嵌入 Windows 工作列',
    });
  });

  it('載入持久化設定與 helper 狀態', async () => {
    const state = getTaskbarState();
    await state.initialize();
    expect(state.enabled).toBe(true);
    expect(state.running).toBe(true);
    expect(state.mode).toBe('embedded');
    expect(state.preferenceMode).toBe('auto');
    expect(state.offsetX).toBe(-320);
    expect(state.showTitleMarquee).toBe(true);
    expect(state.showProgress).toBe(false);
    expect(state.hideInMiniPlayer).toBe(true);
  });

  it('切換開關並套用後端狀態事件', async () => {
    api.setTaskbarPlayerEnabled.mockResolvedValue({
      supported: true,
      enabled: false,
      running: false,
      visible: false,
      mode: null,
      message: '工作列播放器已關閉',
    });
    const state = getTaskbarState();
    await state.setEnabled(false);
    expect(api.setTaskbarPlayerEnabled).toHaveBeenCalledWith(false);
    expect(state.enabled).toBe(false);

    applyTaskbarStatus({
      supported: true,
      enabled: true,
      running: true,
      visible: true,
      mode: 'docked',
      message: '已降級',
    });
    expect(state.mode).toBe('docked');

    api.setTaskbarPlayerMode.mockResolvedValue(state.status);
    await state.setMode('docked');
    expect(api.setTaskbarPlayerMode).toHaveBeenCalledWith('docked');
    expect(state.preferenceMode).toBe('docked');

    api.setTaskbarPlayerOffset.mockResolvedValue(state.status);
    await state.setOffset(-360);
    expect(api.setTaskbarPlayerOffset).toHaveBeenCalledWith(-360);
    expect(state.offsetX).toBe(-360);

    api.setTaskbarPlayerDisplayOptions.mockResolvedValue({
      enabled: true,
      mode: 'docked',
      offset_x: -360,
      show_title_marquee: false,
      show_progress: true,
      hide_in_mini_player: true,
    });
    await state.setDisplayOptions(false, true);
    expect(api.setTaskbarPlayerDisplayOptions).toHaveBeenCalledWith(false, true);
    expect(state.showTitleMarquee).toBe(false);
    expect(state.showProgress).toBe(true);

    api.setTaskbarPlayerMiniModeBehavior.mockResolvedValue({
      enabled: true,
      mode: 'docked',
      offset_x: -360,
      show_title_marquee: false,
      show_progress: true,
      hide_in_mini_player: false,
    });
    await state.setMiniModeBehavior(false);
    expect(api.setTaskbarPlayerMiniModeBehavior).toHaveBeenCalledWith(false);
    expect(state.hideInMiniPlayer).toBe(false);
  });
});
