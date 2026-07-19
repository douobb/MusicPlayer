import { beforeEach, describe, expect, it, vi } from 'vitest';

const invoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...args: unknown[]) => invoke(...args) }));
import * as api from './taskbar';

describe('taskbar API', () => {
  beforeEach(() => invoke.mockReset());

  it('對應設定、狀態與開關命令', async () => {
    invoke.mockResolvedValue({});
    await api.getTaskbarSettings();
    await api.getTaskbarStatus();
    await api.setTaskbarPlayerEnabled(true);
    await api.setTaskbarPlayerMode('docked');
    await api.setTaskbarPlayerOffset(-360);
    await api.setTaskbarPlayerDisplayOptions(false, true);
    await api.setTaskbarPlayerMiniModeBehavior(false);
    await api.setTaskbarPlayerVisible(false);
    expect(invoke.mock.calls).toEqual([
      ['get_taskbar_settings'],
      ['get_taskbar_status'],
      ['set_taskbar_player_enabled', { enabled: true }],
      ['set_taskbar_player_mode', { mode: 'docked' }],
      ['set_taskbar_player_offset', { offsetX: -360 }],
      ['set_taskbar_player_display_options', { showTitleMarquee: false, showProgress: true }],
      ['set_taskbar_player_mini_mode_behavior', { hideInMiniPlayer: false }],
      ['set_taskbar_player_visible', { visible: false }],
    ]);
  });

  it('傳送工作列播放器快照', async () => {
    invoke.mockResolvedValue(undefined);
    const snapshot = {
      title: 'Track',
      artists: 'Artist',
      is_playing: true,
      volume: 0.8,
      can_previous: true,
      can_next: false,
      position_secs: 42,
      duration_secs: 180,
      show_title_marquee: true,
      show_progress: true,
    };
    await api.updateTaskbarPlayer(snapshot);
    expect(invoke).toHaveBeenCalledWith('update_taskbar_player', { snapshot });
  });
});
