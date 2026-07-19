use std::fs;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use musicplayer_taskbar_helper::{
    HelperMessage, HostMessage, TaskbarAction, TaskbarMode, TaskbarPreferenceMode, TaskbarSnapshot,
};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};

use crate::audio::SharedPlayer;
use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct TaskbarSettings {
    pub enabled: bool,
    #[serde(default)]
    pub mode: TaskbarPreferenceMode,
    #[serde(default)]
    pub offset_x: i32,
    #[serde(default = "default_enabled")]
    pub show_title_marquee: bool,
    #[serde(default = "default_enabled")]
    pub show_progress: bool,
    #[serde(default = "default_enabled")]
    pub hide_in_mini_player: bool,
}

const fn default_enabled() -> bool {
    true
}

impl Default for TaskbarSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: TaskbarPreferenceMode::Auto,
            offset_x: 0,
            show_title_marquee: true,
            show_progress: true,
            hide_in_mini_player: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct TaskbarStatus {
    pub supported: bool,
    pub enabled: bool,
    pub running: bool,
    pub visible: bool,
    pub mode: Option<TaskbarMode>,
    pub message: String,
}

impl Default for TaskbarStatus {
    fn default() -> Self {
        Self {
            supported: cfg!(windows),
            enabled: false,
            running: false,
            visible: false,
            mode: None,
            message: if cfg!(windows) {
                "工作列播放器已關閉".to_string()
            } else {
                "工作列播放器僅支援 Windows".to_string()
            },
        }
    }
}

struct ProcessState {
    child: Child,
    input: BufWriter<ChildStdin>,
}

pub struct TaskbarManager {
    process: Mutex<Option<ProcessState>>,
    settings: Mutex<TaskbarSettings>,
    status: Arc<Mutex<TaskbarStatus>>,
    desired_visibility: Mutex<bool>,
    generation: Arc<AtomicU64>,
    settings_path: PathBuf,
}

impl TaskbarManager {
    pub fn new(app: &tauri::AppHandle) -> Result<Self, AppError> {
        let settings_path = app
            .path()
            .app_data_dir()
            .map_err(|error| AppError::Generic(error.to_string()))?
            .join("windows-integration.json");
        let settings = load_settings(&settings_path)?;
        let mut status = TaskbarStatus::default();
        status.enabled = settings.enabled && status.supported;
        Ok(Self {
            process: Mutex::new(None),
            settings: Mutex::new(settings),
            status: Arc::new(Mutex::new(status)),
            desired_visibility: Mutex::new(true),
            generation: Arc::new(AtomicU64::new(0)),
            settings_path,
        })
    }

    pub fn settings(&self) -> Result<TaskbarSettings, AppError> {
        self.settings
            .lock()
            .map_err(|_| AppError::LockPoisoned)
            .map(|settings| settings.clone())
    }

    pub fn status(&self) -> Result<TaskbarStatus, AppError> {
        self.status
            .lock()
            .map_err(|_| AppError::LockPoisoned)
            .map(|status| status.clone())
    }

    pub fn restore(&self, app: &tauri::AppHandle) -> Result<(), AppError> {
        if cfg!(windows) && self.settings()?.enabled {
            self.start(app)?;
        }
        Ok(())
    }

    pub fn set_enabled(
        &self,
        app: &tauri::AppHandle,
        enabled: bool,
    ) -> Result<TaskbarStatus, AppError> {
        if enabled && !cfg!(windows) {
            return Err(AppError::Generic("工作列播放器僅支援 Windows".to_string()));
        }

        if enabled {
            self.start(app)?;
        } else {
            self.stop()?;
            *self
                .desired_visibility
                .lock()
                .map_err(|_| AppError::LockPoisoned)? = true;
        }
        {
            let mut settings = self.settings.lock().map_err(|_| AppError::LockPoisoned)?;
            settings.enabled = enabled;
            save_settings(&self.settings_path, &settings)?;
        }
        {
            let mut status = self.status.lock().map_err(|_| AppError::LockPoisoned)?;
            status.enabled = enabled;
            if !enabled {
                status.running = false;
                status.visible = false;
                status.mode = None;
                status.message = "工作列播放器已關閉".to_string();
            }
        }
        let status = self.status()?;
        let _ = app.emit("taskbar-status-changed", &status);
        Ok(status)
    }

    pub fn set_mode(
        &self,
        app: &tauri::AppHandle,
        mode: TaskbarPreferenceMode,
    ) -> Result<TaskbarStatus, AppError> {
        let enabled = {
            let mut settings = self.settings.lock().map_err(|_| AppError::LockPoisoned)?;
            settings.mode = mode;
            save_settings(&self.settings_path, &settings)?;
            settings.enabled
        };
        if enabled {
            self.stop()?;
            self.start(app)?;
        }
        let status = self.status()?;
        let _ = app.emit("taskbar-status-changed", &status);
        Ok(status)
    }

    pub fn set_offset_x(
        &self,
        app: &tauri::AppHandle,
        offset_x: i32,
    ) -> Result<TaskbarStatus, AppError> {
        let (enabled, offset_x) = {
            let mut settings = self.settings.lock().map_err(|_| AppError::LockPoisoned)?;
            settings.offset_x = offset_x.clamp(-1600, 0);
            save_settings(&self.settings_path, &settings)?;
            (settings.enabled, settings.offset_x)
        };
        if enabled {
            self.ensure_running(app)?;
            let mut process = self.process.lock().map_err(|_| AppError::LockPoisoned)?;
            if let Some(process) = process.as_mut() {
                write_host_message(&mut process.input, &HostMessage::SetOffset { offset_x })?;
            }
        }
        let status = self.status()?;
        let _ = app.emit("taskbar-status-changed", &status);
        Ok(status)
    }

    pub fn set_display_options(
        &self,
        show_title_marquee: bool,
        show_progress: bool,
    ) -> Result<TaskbarSettings, AppError> {
        let mut settings = self.settings.lock().map_err(|_| AppError::LockPoisoned)?;
        settings.show_title_marquee = show_title_marquee;
        settings.show_progress = show_progress;
        save_settings(&self.settings_path, &settings)?;
        Ok(settings.clone())
    }

    pub fn set_mini_player_behavior(
        &self,
        hide_in_mini_player: bool,
    ) -> Result<TaskbarSettings, AppError> {
        let mut settings = self.settings.lock().map_err(|_| AppError::LockPoisoned)?;
        settings.hide_in_mini_player = hide_in_mini_player;
        save_settings(&self.settings_path, &settings)?;
        Ok(settings.clone())
    }

    pub fn set_visible(
        &self,
        app: &tauri::AppHandle,
        visible: bool,
    ) -> Result<TaskbarStatus, AppError> {
        *self
            .desired_visibility
            .lock()
            .map_err(|_| AppError::LockPoisoned)? = visible;
        if !self.settings()?.enabled {
            return self.status();
        }
        self.ensure_running(app)?;
        let mut process = self.process.lock().map_err(|_| AppError::LockPoisoned)?;
        if let Some(process) = process.as_mut() {
            write_host_message(&mut process.input, &HostMessage::SetVisibility { visible })?;
        }
        drop(process);
        {
            let mut status = self.status.lock().map_err(|_| AppError::LockPoisoned)?;
            status.visible = visible;
            status.message = if visible {
                mode_status_message(status.mode)
            } else {
                "Mini Player 使用中，工作列播放器已暫時隱藏".to_string()
            };
        }
        let status = self.status()?;
        let _ = app.emit("taskbar-status-changed", &status);
        Ok(status)
    }

    pub fn update(
        &self,
        app: &tauri::AppHandle,
        snapshot: TaskbarSnapshot,
    ) -> Result<(), AppError> {
        if !self.settings()?.enabled {
            return Ok(());
        }
        self.ensure_running(app)?;
        let mut process = self.process.lock().map_err(|_| AppError::LockPoisoned)?;
        let Some(process) = process.as_mut() else {
            return Ok(());
        };
        write_host_message(&mut process.input, &HostMessage::Update { snapshot })
    }

    fn ensure_running(&self, app: &tauri::AppHandle) -> Result<(), AppError> {
        let exited = {
            let mut process = self.process.lock().map_err(|_| AppError::LockPoisoned)?;
            match process.as_mut() {
                Some(process) => process
                    .child
                    .try_wait()
                    .map_err(|error| AppError::Generic(error.to_string()))?
                    .is_some(),
                None => true,
            }
        };
        if exited {
            self.start(app)?;
        }
        Ok(())
    }

    fn start(&self, app: &tauri::AppHandle) -> Result<(), AppError> {
        if !cfg!(windows) {
            return Err(AppError::Generic("工作列播放器僅支援 Windows".to_string()));
        }
        let mut process_slot = self.process.lock().map_err(|_| AppError::LockPoisoned)?;
        if let Some(process) = process_slot.as_mut() {
            if process
                .child
                .try_wait()
                .map_err(|error| AppError::Generic(error.to_string()))?
                .is_none()
            {
                return Ok(());
            }
        }
        *process_slot = None;

        let executable =
            std::env::current_exe().map_err(|error| AppError::Generic(error.to_string()))?;
        let preference = self.settings()?.mode;
        let offset_x = self.settings()?.offset_x;
        let mode_argument = match preference {
            TaskbarPreferenceMode::Auto => "--taskbar-mode=auto",
            TaskbarPreferenceMode::Docked => "--taskbar-mode=docked",
        };
        let mut child = Command::new(executable)
            .arg("--taskbar-helper")
            .arg(mode_argument)
            .arg(format!("--taskbar-offset-x={offset_x}"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| AppError::Generic(format!("無法啟動工作列 helper：{error}")))?;
        let input = child
            .stdin
            .take()
            .ok_or_else(|| AppError::Generic("無法連接工作列 helper 輸入".to_string()))?;
        let output = child
            .stdout
            .take()
            .ok_or_else(|| AppError::Generic("無法連接工作列 helper 輸出".to_string()))?;

        *process_slot = Some(ProcessState {
            child,
            input: BufWriter::new(input),
        });
        let desired_visibility = *self
            .desired_visibility
            .lock()
            .map_err(|_| AppError::LockPoisoned)?;
        if !desired_visibility {
            let process = process_slot
                .as_mut()
                .ok_or_else(|| AppError::Generic("工作列 helper 行程狀態遺失".to_string()))?;
            write_host_message(
                &mut process.input,
                &HostMessage::SetVisibility { visible: false },
            )?;
        }
        drop(process_slot);
        let generation_id = self.generation.fetch_add(1, Ordering::AcqRel) + 1;

        {
            let mut status = self.status.lock().map_err(|_| AppError::LockPoisoned)?;
            status.enabled = true;
            status.running = true;
            status.visible = desired_visibility;
            status.mode = None;
            status.message = "工作列播放器啟動中…".to_string();
        }
        let status = Arc::clone(&self.status);
        let generation = Arc::clone(&self.generation);
        let app_handle = app.clone();
        std::thread::spawn(move || {
            read_helper_messages(output, &app_handle, &status, &generation, generation_id);
        });
        Ok(())
    }

    fn stop(&self) -> Result<(), AppError> {
        self.generation.fetch_add(1, Ordering::AcqRel);
        let mut process = self.process.lock().map_err(|_| AppError::LockPoisoned)?;
        if let Some(mut process_state) = process.take() {
            let _ = write_host_message(&mut process_state.input, &HostMessage::Shutdown);
            std::thread::spawn(move || {
                for _ in 0..10 {
                    if process_state.child.try_wait().ok().flatten().is_some() {
                        return;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                let _ = process_state.child.kill();
                let _ = process_state.child.wait();
            });
        }
        Ok(())
    }
}

fn write_host_message(
    input: &mut BufWriter<ChildStdin>,
    message: &HostMessage,
) -> Result<(), AppError> {
    serde_json::to_writer(&mut *input, message)
        .map_err(|error| AppError::Generic(error.to_string()))?;
    input
        .write_all(b"\n")
        .and_then(|()| input.flush())
        .map_err(|error| AppError::Generic(error.to_string()))
}

fn read_helper_messages(
    output: std::process::ChildStdout,
    app: &tauri::AppHandle,
    status: &Arc<Mutex<TaskbarStatus>>,
    generation: &AtomicU64,
    generation_id: u64,
) {
    for line in BufReader::new(output).lines() {
        if !is_current_generation(generation, generation_id) {
            return;
        }
        let Ok(line) = line else {
            break;
        };
        match serde_json::from_str::<HelperMessage>(&line) {
            Ok(HelperMessage::Ready { mode }) => {
                update_status(status, app, |status| {
                    status.running = true;
                    status.mode = Some(mode);
                    status.message = if status.visible {
                        mode_status_message(Some(mode))
                    } else {
                        "Mini Player 使用中，工作列播放器已暫時隱藏".to_string()
                    };
                });
            }
            Ok(HelperMessage::Action { action }) => handle_action(app, action),
            Ok(HelperMessage::Error { message }) => {
                update_status(status, app, |status| {
                    status.message = message;
                });
            }
            Err(error) => {
                update_status(status, app, |status| {
                    status.message = format!("helper 回傳無效訊息：{error}");
                });
            }
        }
    }
    if !is_current_generation(generation, generation_id) {
        return;
    }
    update_status(status, app, |status| {
        status.running = false;
        status.visible = false;
        status.mode = None;
        if status.enabled {
            status.message = "工作列 helper 已停止，將在下次狀態更新時重啟".to_string();
        }
    });
}

fn mode_status_message(mode: Option<TaskbarMode>) -> String {
    match mode {
        Some(TaskbarMode::Embedded) => "已嵌入 Windows 工作列".to_string(),
        Some(TaskbarMode::Docked) => "使用貼齊工作列模式".to_string(),
        Some(TaskbarMode::Unavailable) => "工作列整合目前無法使用".to_string(),
        None => "工作列播放器啟動中…".to_string(),
    }
}

fn is_current_generation(generation: &AtomicU64, generation_id: u64) -> bool {
    generation.load(Ordering::Acquire) == generation_id
}

fn update_status(
    status: &Arc<Mutex<TaskbarStatus>>,
    app: &tauri::AppHandle,
    update: impl FnOnce(&mut TaskbarStatus),
) {
    let Ok(mut status) = status.lock() else {
        return;
    };
    update(&mut status);
    let snapshot = status.clone();
    drop(status);
    let _ = app.emit("taskbar-status-changed", snapshot);
}

fn handle_action(app: &tauri::AppHandle, action: TaskbarAction) {
    match action {
        TaskbarAction::Previous => {
            let _ = app.emit("taskbar-prev", ());
        }
        TaskbarAction::Next => {
            let _ = app.emit("taskbar-next", ());
        }
        TaskbarAction::PlayPause => {
            let player = Arc::clone(app.state::<SharedPlayer>().inner());
            if let Ok(player) = player.lock() {
                if player.is_playing() {
                    player.pause();
                } else {
                    player.play();
                }
            }
        }
        TaskbarAction::AdjustVolume(delta) => {
            let player = Arc::clone(app.state::<SharedPlayer>().inner());
            if let Ok(mut player) = player.lock() {
                let volume = player.get_volume();
                player.set_volume(volume + delta);
            }
        }
        TaskbarAction::OpenMainWindow => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
    }
}

fn load_settings(path: &Path) -> Result<TaskbarSettings, AppError> {
    if !path.exists() {
        return Ok(TaskbarSettings::default());
    }
    let data = fs::read(path).map_err(|error| AppError::Generic(error.to_string()))?;
    serde_json::from_slice(&data).map_err(|error| AppError::Generic(error.to_string()))
}

fn save_settings(path: &Path, settings: &TaskbarSettings) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| AppError::Generic(error.to_string()))?;
    }
    let data = serde_json::to_vec_pretty(settings)
        .map_err(|error| AppError::Generic(error.to_string()))?;
    fs::write(path, data).map_err(|error| AppError::Generic(error.to_string()))
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn missing_settings_use_safe_disabled_default() {
        let temp = tempdir().unwrap();
        assert_eq!(
            load_settings(&temp.path().join("missing.json")).unwrap(),
            TaskbarSettings {
                enabled: false,
                mode: TaskbarPreferenceMode::Auto,
                offset_x: 0,
                show_title_marquee: true,
                show_progress: true,
                hide_in_mini_player: true,
            }
        );
    }

    #[test]
    fn settings_round_trip() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("nested").join("settings.json");
        let expected = TaskbarSettings {
            enabled: true,
            mode: TaskbarPreferenceMode::Docked,
            offset_x: -360,
            show_title_marquee: false,
            show_progress: true,
            hide_in_mini_player: false,
        };
        save_settings(&path, &expected).unwrap();
        assert_eq!(load_settings(&path).unwrap(), expected);
    }

    #[test]
    fn legacy_settings_enable_new_display_options_by_default() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("windows-integration.json");
        fs::write(&path, br#"{"enabled":true,"mode":"auto","offset_x":-120}"#).unwrap();

        let settings = load_settings(&path).unwrap();
        assert!(settings.show_title_marquee);
        assert!(settings.show_progress);
        assert!(settings.hide_in_mini_player);
    }

    #[test]
    fn stale_helper_generation_cannot_update_current_status() {
        let generation = AtomicU64::new(4);
        assert!(is_current_generation(&generation, 4));
        generation.fetch_add(1, Ordering::AcqRel);
        assert!(!is_current_generation(&generation, 4));
    }
}
