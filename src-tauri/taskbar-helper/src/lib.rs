use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HostMessage {
    Update { snapshot: TaskbarSnapshot },
    SetOffset { offset_x: i32 },
    SetVisibility { visible: bool },
    Shutdown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HelperMessage {
    Ready { mode: TaskbarMode },
    Action { action: TaskbarAction },
    Error { message: String },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskbarMode {
    Embedded,
    Docked,
    Unavailable,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TaskbarPreferenceMode {
    #[default]
    Auto,
    Docked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskbarAction {
    Previous,
    PlayPause,
    Next,
    AdjustVolume(f32),
    OpenMainWindow,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[allow(clippy::struct_excessive_bools)]
pub struct TaskbarSnapshot {
    pub title: String,
    pub artists: String,
    pub is_playing: bool,
    pub volume: f32,
    pub can_previous: bool,
    pub can_next: bool,
    pub position_secs: f64,
    pub duration_secs: f64,
    pub show_title_marquee: bool,
    pub show_progress: bool,
}

impl Default for TaskbarSnapshot {
    fn default() -> Self {
        Self {
            title: "MusicPlayer".to_string(),
            artists: String::new(),
            is_playing: false,
            volume: 0.5,
            can_previous: false,
            can_next: false,
            position_secs: 0.0,
            duration_secs: 0.0,
            show_title_marquee: true,
            show_progress: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    #[must_use]
    pub fn width(self) -> i32 {
        self.right - self.left
    }

    #[must_use]
    pub fn height(self) -> i32 {
        self.bottom - self.top
    }
}

#[must_use]
pub fn calculate_taskbar_window_rect(
    taskbar: Rect,
    notification_area: Option<Rect>,
    requested_width: i32,
) -> Option<Rect> {
    if taskbar.width() <= 0 || taskbar.height() <= 0 || taskbar.width() < taskbar.height() {
        return None;
    }

    let margin = 6;
    let height = (taskbar.height() - 4).clamp(24, 40);
    let available_right = notification_area
        .filter(|rect| rect.left > taskbar.left && rect.left <= taskbar.right)
        .map_or(taskbar.right - margin, |rect| rect.left - margin);
    let available_width = available_right - taskbar.left - margin;
    if available_width < 180 {
        return None;
    }

    let width = requested_width.clamp(180, available_width.min(360));
    let left = available_right - width;
    let top = taskbar.top + (taskbar.height() - height) / 2;
    Some(Rect {
        left,
        top,
        right: left + width,
        bottom: top + height,
    })
}

#[cfg(windows)]
mod composition;

#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub use windows::run_stdio;

#[cfg(not(windows))]
pub fn run_stdio(_preference: TaskbarPreferenceMode, _offset_x: i32) -> Result<(), String> {
    Err("工作列播放器僅支援 Windows".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_round_trip_preserves_snapshot() {
        let message = HostMessage::Update {
            snapshot: TaskbarSnapshot {
                title: "Track".to_string(),
                artists: "Artist".to_string(),
                is_playing: true,
                volume: 0.8,
                can_previous: true,
                can_next: false,
                position_secs: 42.0,
                duration_secs: 180.0,
                show_title_marquee: true,
                show_progress: true,
            },
        };
        let json = serde_json::to_string(&message).unwrap();
        assert_eq!(serde_json::from_str::<HostMessage>(&json).unwrap(), message);
    }

    #[test]
    fn offset_update_uses_stable_protocol_shape() {
        let message = HostMessage::SetOffset { offset_x: -360 };
        assert_eq!(
            serde_json::to_string(&message).unwrap(),
            r#"{"type":"set_offset","offset_x":-360}"#
        );
    }

    #[test]
    fn visibility_update_uses_stable_protocol_shape() {
        let message = HostMessage::SetVisibility { visible: false };
        assert_eq!(
            serde_json::to_string(&message).unwrap(),
            r#"{"type":"set_visibility","visible":false}"#
        );
    }

    #[test]
    fn preference_mode_uses_stable_json_values() {
        assert_eq!(
            serde_json::to_string(&TaskbarPreferenceMode::Auto).unwrap(),
            r#""auto""#
        );
        assert_eq!(
            serde_json::from_str::<TaskbarPreferenceMode>(r#""docked""#).unwrap(),
            TaskbarPreferenceMode::Docked
        );
    }

    #[test]
    fn horizontal_layout_stays_before_notification_area() {
        let rect = calculate_taskbar_window_rect(
            Rect {
                left: 0,
                top: 1040,
                right: 1920,
                bottom: 1080,
            },
            Some(Rect {
                left: 1680,
                top: 1040,
                right: 1920,
                bottom: 1080,
            }),
            300,
        )
        .unwrap();
        assert_eq!(rect.right, 1674);
        assert_eq!(rect.width(), 300);
        assert_eq!(rect.height(), 36);
    }

    #[test]
    fn vertical_or_too_small_taskbar_uses_fallback() {
        assert_eq!(
            calculate_taskbar_window_rect(
                Rect {
                    left: 0,
                    top: 0,
                    right: 48,
                    bottom: 1080,
                },
                None,
                300,
            ),
            None
        );
        assert_eq!(
            calculate_taskbar_window_rect(
                Rect {
                    left: 0,
                    top: 0,
                    right: 170,
                    bottom: 40,
                },
                None,
                300,
            ),
            None
        );
    }
}
