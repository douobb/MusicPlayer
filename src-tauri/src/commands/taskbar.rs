use musicplayer_taskbar_helper::{TaskbarPreferenceMode, TaskbarSnapshot};
use tauri::{AppHandle, State};

use crate::error::AppError;
use crate::taskbar::{TaskbarManager, TaskbarSettings, TaskbarStatus};

#[tauri::command]
pub fn get_taskbar_settings(manager: State<TaskbarManager>) -> Result<TaskbarSettings, AppError> {
    manager.settings()
}

#[tauri::command]
pub fn get_taskbar_status(manager: State<TaskbarManager>) -> Result<TaskbarStatus, AppError> {
    manager.status()
}

#[tauri::command]
pub fn set_taskbar_player_enabled(
    enabled: bool,
    app: AppHandle,
    manager: State<TaskbarManager>,
) -> Result<TaskbarStatus, AppError> {
    manager.set_enabled(&app, enabled)
}

#[tauri::command]
pub fn set_taskbar_player_mode(
    mode: TaskbarPreferenceMode,
    app: AppHandle,
    manager: State<TaskbarManager>,
) -> Result<TaskbarStatus, AppError> {
    manager.set_mode(&app, mode)
}

#[tauri::command]
pub fn set_taskbar_player_offset(
    offset_x: i32,
    app: AppHandle,
    manager: State<TaskbarManager>,
) -> Result<TaskbarStatus, AppError> {
    manager.set_offset_x(&app, offset_x)
}

#[tauri::command]
pub fn set_taskbar_player_display_options(
    show_title_marquee: bool,
    show_progress: bool,
    manager: State<TaskbarManager>,
) -> Result<TaskbarSettings, AppError> {
    manager.set_display_options(show_title_marquee, show_progress)
}

#[tauri::command]
pub fn set_taskbar_player_mini_mode_behavior(
    hide_in_mini_player: bool,
    manager: State<TaskbarManager>,
) -> Result<TaskbarSettings, AppError> {
    manager.set_mini_player_behavior(hide_in_mini_player)
}

#[tauri::command]
pub fn set_taskbar_player_visible(
    visible: bool,
    app: AppHandle,
    manager: State<TaskbarManager>,
) -> Result<TaskbarStatus, AppError> {
    manager.set_visible(&app, visible)
}

#[tauri::command]
pub fn update_taskbar_player(
    snapshot: TaskbarSnapshot,
    app: AppHandle,
    manager: State<TaskbarManager>,
) -> Result<(), AppError> {
    manager.update(&app, snapshot)
}
