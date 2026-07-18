# MusicPlayer

English | [繁體中文](README.md)

This project is developed based on [twtrubiks/lyra-music](https://github.com/twtrubiks/lyra-music).

A local media player built with Tauri 2 + Svelte 5 + Rust. The initial version retains the upstream project's local music playback capabilities, with Lite and Full editions planned for the future.

## Edition Roadmap

> The following items are plans. Unfinished items are not currently supported features.

| Edition | Positioning | Planned features |
|---------|-------------|------------------|
| Lite | Local-only, offline-first, and resource-efficient | Music playback, media library, tags, playlists, Mini Player, System Tray, and Windows taskbar controls |
| Full | All features included | Everything in Lite, plus downloads, MP4 video playback, and dynamic music waveforms; video and waveforms can be toggled independently |

## Differences from Upstream

This project retains the upstream architecture while expanding it for its own product direction. See [Differences from the Upstream Project](docs/upstream-changes.md) for the main differences.

## Download

MusicPlayer does not have an official release yet.

## Technical Architecture

Further reading: [Why Rust](docs/why-rust.md), [Tauri 2 Introduction](docs/tauri2-introduction.md)

| Layer | Technology | Description |
|-------|------------|-------------|
| Frontend | Svelte 5 + TypeScript | Reactive state management using Svelte 5 runes |
| Build Tool | Vite 8 | Dev server and frontend bundling |
| Desktop Framework | Tauri 2 | Native windows, system tray, IPC communication |
| Backend | Rust | Audio processing, file scanning, database operations |
| Audio Engine | rodio 0.22 | Pure Rust implementation, no need for GStreamer, MPV, or other system audio frameworks |
| Metadata Parsing | lofty 0.24 | Read/write ID3/Vorbis/MP4 tags and cover art |
| File Watching | notify 8 | Real-time folder change detection, automatic music library updates |
| Database | SQLite (rusqlite, bundled) | WAL mode, schema migration management |
| Testing | Vitest + cargo test | 25 frontend test files, 17 backend integration tests |

## Current Features

**Local music playback** -- Supports MP3, FLAC, WAV, OGG, M4A, and AAC formats. The audio engine is based on rodio with full play / pause / stop / seek controls. Volume uses quadratic curve mapping (UI 0.5 maps to actual 0.25) for a more natural listening experience.

**Gapless playback** -- Pre-decodes the next track and appends it to the same sink for seamless transitions. Does not require matching sample rates between consecutive tracks.

**Playlists & resume playback** -- Create, edit, and delete playlists with drag-and-drop reordering. Each playlist records the last played track ID and position in seconds, automatically restoring playback progress when switching playlists.

**Mini Player + System Tray** -- Press `m` to switch to a compact 420x80 window (always-on-top). System tray supports Play/Pause, previous, next, show window, and quit. Closing the window automatically minimizes to the system tray.

**Tauri 2 + Svelte 5 + Rust architecture** -- Frontend and backend communicate through typed Tauri commands via IPC. The frontend manages state with Svelte 5 runes, while the backend handles audio decoding, file I/O, and database operations in Rust.

**Tags and shared collection actions** -- Albums have been replaced by an internal multi-tag system. It supports creating, renaming, deleting, merging, and cleaning empty tags, plus per-track and multi-select batch editing. The track context menu organizes playlists and tags into searchable, scrollable second-level menus, with “All/Partial” assignment states for multi-selection. All Music, Artist, Tag, and playlist views share play-all, shuffle, add-to-queue, and add-to-playlist actions.

**Multiple performers and original artists** -- Each track can store ordered lists of performers and original artists. Artists can be created, renamed, merged, and cleaned up, with works browsable by performer or original-artist role. Search, sorting, statistics, the player, and all track lists support multiple artists.

**Settings and library folder management** -- A Settings entry in the sidebar manages multiple library folders, manual incremental rescans, pausing or resuming watchers, and whether indexed tracks are retained when a folder is removed. Startup synchronization catches changes made while the app was closed, while unavailable folders do not cause tracks to be removed accidentally. Rename, merge, and delete actions use consistent in-app dialogs; deletion confirmation is enabled by default and can be disabled in General settings.

Other features:
- Artist and Tag browse views (search filtering, role-aware counts, and detail views)
- Track metadata editing (title, multiple performers, and original artists written back to the file)
- Real-time folder watching (add/modify/delete automatically syncs music library)
- Column header sorting (preferences persisted), play count tracking (Most Played ranking view)
- Recursive music library scanning with automatic metadata reading and cover art caching
- Playback modes (loop/repeat-one/shuffle), instant search filtering, multi-select operations, context menu, drag-and-drop import

## Prerequisites

- [Node.js](https://nodejs.org/) (LTS)
- [Rust toolchain](https://rustup.rs/) (rustup, Rust 1.87+)
- Tauri 2 system dependencies: see [Tauri Prerequisites](https://v2.tauri.app/start/prerequisites/) (macOS/Windows usually require no additional installation)

Linux (Debian/Ubuntu) additionally requires:

```
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
  libssl-dev libayatana-appindicator3-dev librsvg2-dev libasound2-dev
```

## Installation & Running

```bash
npm install           # Install frontend dependencies
npm run tauri dev     # Development mode (starts both Vite dev server and Tauri window)
npm run tauri build   # Production build
```

Build artifacts are located in `src-tauri/target/release/bundle/`, supporting deb, AppImage (Linux), dmg (macOS), and nsis/msi (Windows).

## Testing

```bash
npm run test                    # Frontend unit and component tests (Vitest, 25 test files)
npm run check                   # Type checking
cd src-tauri && cargo test      # Backend integration tests (17 test files, audio tests skipped by default)
cd src-tauri && cargo test --features audio-tests  # With audio tests (requires audio device)
npm run quality                 # Code quality checks (ESLint + Prettier + Stylelint + Clippy + rustfmt)
```

## Keyboard Shortcuts

All shortcuts are disabled when an input field is focused.

| Key | Action |
|-----|--------|
| `Space` | Play / Pause |
| `ArrowLeft` / `ArrowRight` | Rewind / Fast-forward 5 seconds |
| `ArrowUp` / `ArrowDown` | Increase / Decrease volume by 5% (when track list is not focused) |
| `n` / `p` | Next / Previous track |
| `s` | Toggle shuffle |
| `r` | Toggle repeat mode (off / repeat-all / repeat-one) |
| `m` / `Escape` | Toggle / Exit Mini Player |
| `Ctrl+F` / `Cmd+F` | Focus search box |
| `Ctrl+A` / `Cmd+A` | Select all tracks |

**When track list is focused:**

| Key | Action |
|-----|--------|
| `ArrowUp` / `ArrowDown` | Previous / Next track |
| `Shift+ArrowUp` / `Shift+ArrowDown` | Extend selection up / down |
| `Enter` | Play focused track |
| `Home` / `End` | Jump to first / last track |

## Project Structure

```
src/                              # Frontend (Svelte 5 + TypeScript)
  lib/
    api/                          # Tauri IPC call wrappers (playback, library, playlist, tag)
    components/                   # UI components (Player, Library, Browse, Tags, Playlist, Common)
    state/                        # Reactive state management (Svelte 5 runes)
    logic/                        # Pure function logic (playback modes, shortcuts, formatting, selection, sorting)
    types/                        # TypeScript type definitions
src-tauri/                        # Backend (Rust)
  src/
    audio/                        # Audio engine (rodio sink, gapless queue)
    scanner/                      # Folder scanning & file watching (walkdir, notify)
    metadata/                     # Metadata read/write & cover art caching (lofty)
    storage/                      # SQLite database (schema v12, WAL mode)
    commands/                     # Tauri command handlers
    models/                       # Data structures (track, artist, tag, playlist, player_state)
  tests/                          # 17 integration tests
```
