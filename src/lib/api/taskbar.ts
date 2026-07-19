import { invoke } from '@tauri-apps/api/core';
import type {
  TaskbarPreferenceMode,
  TaskbarSettings,
  TaskbarSnapshot,
  TaskbarStatus,
} from '$lib/types';

export async function getTaskbarSettings(): Promise<TaskbarSettings> {
  return invoke('get_taskbar_settings');
}

export async function getTaskbarStatus(): Promise<TaskbarStatus> {
  return invoke('get_taskbar_status');
}

export async function setTaskbarPlayerEnabled(enabled: boolean): Promise<TaskbarStatus> {
  return invoke('set_taskbar_player_enabled', { enabled });
}

export async function setTaskbarPlayerMode(mode: TaskbarPreferenceMode): Promise<TaskbarStatus> {
  return invoke('set_taskbar_player_mode', { mode });
}

export async function setTaskbarPlayerOffset(offsetX: number): Promise<TaskbarStatus> {
  return invoke('set_taskbar_player_offset', { offsetX });
}

export async function setTaskbarPlayerDisplayOptions(
  showTitleMarquee: boolean,
  showProgress: boolean,
): Promise<TaskbarSettings> {
  return invoke('set_taskbar_player_display_options', { showTitleMarquee, showProgress });
}

export async function setTaskbarPlayerMiniModeBehavior(
  hideInMiniPlayer: boolean,
): Promise<TaskbarSettings> {
  return invoke('set_taskbar_player_mini_mode_behavior', { hideInMiniPlayer });
}

export async function setTaskbarPlayerVisible(visible: boolean): Promise<TaskbarStatus> {
  return invoke('set_taskbar_player_visible', { visible });
}

export async function updateTaskbarPlayer(snapshot: TaskbarSnapshot): Promise<void> {
  return invoke('update_taskbar_player', { snapshot });
}
