# MusicPlayer

[English](README.en.md) | 繁體中文

本專案基於 [twtrubiks/lyra-music](https://github.com/twtrubiks/lyra-music) 開發。

基於 Tauri 2 + Svelte 5 + Rust 的本機媒體播放器。目前初版延續上游的本機音樂播放能力，未來將提供 Lite 與完整版。

## 版本規劃

> 下列為開發規劃，尚未完成的項目不代表目前已支援。

| 版本 | 定位 | 規劃功能 |
|------|------|----------|
| Lite | 純本機、離線優先、低資源佔用 | 音樂播放、媒體庫、Tag、播放清單、Mini Player、System Tray、Windows 工作列控制 |
| 完整版 | 包含所有功能 | Lite 全部功能，加上下載、MP4 視訊播放與動態音樂波形；視訊與波形可獨立開關 |

## 與上游專案的差異

本專案保留上游架構並依自身產品定位擴充，主要差異請參閱 [與上游專案的差異](docs/upstream-changes.md)。

## 下載

MusicPlayer 尚未提供正式發佈版本。

## 技術架構

延伸閱讀：[為什麼選擇 Rust](docs/why-rust.md)、[Tauri 2 介紹](docs/tauri2-introduction.md)

| 層級 | 技術 | 說明 |
|------|------|------|
| 前端 | Svelte 5 + TypeScript | 使用 Svelte 5 runes 做響應式狀態管理 |
| 建置工具 | Vite 8 | 開發伺服器與前端打包 |
| 桌面框架 | Tauri 2 | 原生視窗、系統匣、IPC 通訊 |
| 後端 | Rust | 音訊處理、檔案掃描、資料庫操作 |
| 音訊引擎 | rodio 0.22 | 純 Rust 實作，不需要安裝 GStreamer、MPV 等系統音訊框架 |
| 元資料解析 | lofty 0.24 | 讀寫 ID3/Vorbis/MP4 標籤與封面圖 |
| 檔案監視 | notify 8 | 即時偵測資料夾變化，自動更新音樂庫 |
| 資料庫 | SQLite (rusqlite, bundled) | WAL mode，schema migration 管理 |
| 測試 | Vitest + cargo test | 前端 28 個測試檔、Rust workspace 自動測試 |

## 目前功能

**本地音樂播放** -- 支援 MP3、FLAC、WAV、OGG、M4A、AAC 格式。音訊引擎基於 rodio，play / pause / stop / seek 完整控制，音量使用二次曲線映射（UI 0.5 對應實際 0.25），聽感更自然。

**Gapless playback** -- 預先解碼下一首並 append 到同一 sink，實現無縫銜接。不要求前後曲目 sample rate 一致。

**播放清單與斷點續播** -- 建立、編輯、刪除播放清單，支援拖曳排序。每個播放清單記錄最後播放的曲目 ID 與秒數位置，切換播放清單時自動恢復上次播放進度。

**Mini Player + System Tray** -- 按 `m` 切換為 420x80 精簡視窗（always-on-top）。系統匣支援 Play/Pause、上一首、下一首、顯示視窗、退出。關閉視窗時自動最小化到系統匣。

**Windows 工作列播放器** -- 此功能僅支援 Windows 10／11 的主螢幕水平工作列，可從主播放器或「設定 > Windows 整合」啟用。預設採用嵌入工作列模式，不相容時可安全降級或手動切換為貼齊模式；可用左右按鈕即時調整水平位置以避開其他工具。透明背景與 Windows 字體可融入工作列，並提供曲名、演唱者、可關閉的標題動態滾動與進度條，以及基本播放與音量控制。預設在 Mini Player 開啟時暫時隱藏，離開或縮至 System Tray 後自動恢復，亦可在設定中允許兩者同時顯示。

**Tauri 2 + Svelte 5 + Rust 架構** -- 前後端透過型別化的 Tauri commands 進行 IPC 通訊。前端以 Svelte 5 runes 管理狀態，後端以 Rust 處理音訊解碼、檔案 I/O、資料庫操作。

**Tag 分類與共用播放操作** -- Album 已改為可讓同一曲目擁有多個分類的 Tag 系統，支援建立、重新命名、刪除、合併、清理空 Tag、單曲與多選批次編輯。Tags 視圖提供 Tag 數、已標記／未標記曲目、關聯數、平均 Tag 數與最常使用 Tag 等摘要。曲目右鍵選單將播放清單與 Tag 收納至可搜尋、可捲動的第二層選單，多選時以「全部／部分」標示 Tag 套用狀態。All Music、Artist、Tag 與播放清單共用播放全部、隨機播放、加入佇列及加入播放清單操作。

**多人演唱者與原唱** -- 每首曲目可依順序標註多位演唱者及多位原唱；Artist 支援建立、重新命名、合併與清理，並可依演唱／原唱角色瀏覽作品。搜尋、排序、統計、播放器及各曲目列表皆支援多人資訊。

**設定與媒體庫資料夾管理** -- 側邊欄提供設定入口，可管理多個媒體庫資料夾、手動增量重新掃描、暫停／恢復監看，以及選擇移除資料夾時是否保留索引曲目。啟動時會快速同步停機期間的變更，無法存取的資料夾不會造成曲目被誤刪。重新命名、合併與刪除操作使用一致的應用程式內建彈框，刪除確認預設開啟，亦可在一般設定關閉。

其他功能：
- Artist 與 Tag 瀏覽視圖（搜尋過濾、角色統計、詳情視圖）
- 曲目元資料編輯（標題、多人演唱者與原唱資訊寫回檔案）
- 資料夾即時監視（新增／修改／刪除自動同步媒體庫）
- 欄標題排序（偏好持久化）、播放計數追蹤（Most Played 排行視圖）
- 音樂庫遞迴掃描，自動讀取 metadata 與封面快取
- 播放模式（循環/單曲/隨機）、即時搜尋過濾、多選操作、右鍵選單、拖放匯入

## 前置需求

- [Node.js](https://nodejs.org/) (LTS)
- [Rust toolchain](https://rustup.rs/) (rustup, Rust 1.87+)
- Tauri 2 系統依賴：參考 [Tauri Prerequisites](https://v2.tauri.app/start/prerequisites/)（macOS/Windows 通常不需要額外安裝）

Linux（Debian/Ubuntu）額外需要：

```
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
  libssl-dev libayatana-appindicator3-dev librsvg2-dev libasound2-dev
```

## 安裝與啟動

```bash
npm install           # 安裝前端依賴
npm run tauri dev     # 開發模式（同時啟動 Vite dev server 和 Tauri 視窗）
npm run tauri build   # 正式建置
```

建置產物位於 `src-tauri/target/release/bundle/`，支援 deb、AppImage（Linux）、dmg（macOS）、nsis/msi（Windows）。

## 測試

```bash
npm run test                    # 前端單元與元件測試 (Vitest, 28 個測試檔)
npm run check                   # 類型檢查
cd src-tauri && cargo test --workspace  # Rust workspace 自動測試（音訊測試預設跳過）
cd src-tauri && cargo test --workspace --features audio-tests  # 含音訊測試 (需音訊裝置)
npm run quality                 # 程式碼品質檢查 (ESLint + Prettier + Stylelint + Clippy + rustfmt)
```

## 鍵盤快捷鍵

所有快捷鍵在輸入框聚焦時不生效。

| 按鍵 | 動作 |
|------|------|
| `Space` | 播放 / 暫停 |
| `ArrowLeft` / `ArrowRight` | 快退 / 快進 5 秒 |
| `ArrowUp` / `ArrowDown` | 音量增加 / 降低 5%（曲目列表未聚焦時） |
| `n` / `p` | 下一首 / 上一首 |
| `s` | 切換隨機播放 |
| `r` | 切換循環模式（off / repeat-all / repeat-one） |
| `m` / `Escape` | 切換 / 退出 Mini Player |
| `Ctrl+F` / `Cmd+F` | 聚焦搜尋框 |
| `Ctrl+A` / `Cmd+A` | 全選曲目 |

**曲目列表聚焦時：**

| 按鍵 | 動作 |
|------|------|
| `ArrowUp` / `ArrowDown` | 上一首 / 下一首曲目 |
| `Shift+ArrowUp` / `Shift+ArrowDown` | 向上 / 向下擴展選取 |
| `Enter` | 播放聚焦曲目 |
| `Home` / `End` | 跳到第一首 / 最後一首 |

## 專案結構

```
src/                              # 前端 (Svelte 5 + TypeScript)
  lib/
    api/                          # Tauri IPC 呼叫封裝 (playback, library, playlist, tag)
    components/                   # UI 元件 (Player, Library, Browse, Tags, Playlist, Common)
    state/                        # 響應式狀態管理 (Svelte 5 runes)
    logic/                        # 純函式邏輯 (播放模式、快捷鍵、格式化、選取、排序)
    types/                        # TypeScript 型別定義
src-tauri/                        # 後端 (Rust)
  src/
    audio/                        # 音訊引擎 (rodio sink, gapless queue)
    scanner/                      # 資料夾掃描與檔案監視 (walkdir, notify)
    metadata/                     # 元資料讀寫與封面快取 (lofty)
    storage/                      # SQLite 資料庫 (schema v12, WAL mode)
    commands/                     # Tauri command handlers
    models/                       # 資料結構定義 (track, artist, tag, playlist, player_state)
  tests/                          # 17 個整合測試
```
