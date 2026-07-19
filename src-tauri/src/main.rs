// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(windows)]
    if std::env::args().any(|arg| arg == "--taskbar-helper") {
        let mode = if std::env::args().any(|arg| arg == "--taskbar-mode=docked") {
            musicplayer_taskbar_helper::TaskbarPreferenceMode::Docked
        } else {
            musicplayer_taskbar_helper::TaskbarPreferenceMode::Auto
        };
        let offset_x = std::env::args()
            .find_map(|arg| arg.strip_prefix("--taskbar-offset-x=").map(str::to_owned))
            .and_then(|value| value.parse::<i32>().ok())
            .unwrap_or_default()
            .clamp(-1600, 0);
        if let Err(error) = musicplayer_taskbar_helper::run_stdio(mode, offset_x) {
            eprintln!("[musicplayer-taskbar] {error}");
        }
        return;
    }

    musicplayer_lib::run();
}
