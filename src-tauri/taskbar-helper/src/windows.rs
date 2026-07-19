use std::io::{BufRead, BufReader, BufWriter, Write};
use std::ptr::{null, null_mut};
use std::sync::mpsc::{self, Receiver};
use std::thread;

use windows_sys::Win32::Foundation::{
    COLORREF, GetLastError, HWND, LPARAM, LRESULT, RECT, SIZE, SetLastError, WPARAM,
};
use windows_sys::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BeginPaint, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, CreateCompatibleDC,
    CreateDIBSection, CreateFontW, CreateSolidBrush, DEFAULT_CHARSET, DEFAULT_GUI_FONT,
    DEFAULT_PITCH, DIB_RGB_COLORS, DT_CENTER, DT_END_ELLIPSIS, DT_LEFT, DT_NOPREFIX, DT_SINGLELINE,
    DT_VCENTER, DeleteDC, DeleteObject, DrawTextW, EndPaint, FF_DONTCARE, FW_NORMAL, FillRect,
    GetStockObject, GetTextExtentPoint32W, IntersectClipRect, InvalidateRect, OUT_DEFAULT_PRECIS,
    PAINTSTRUCT, RestoreDC, SaveDC, SelectObject, SetBkMode, SetTextColor, TRANSPARENT,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::HiDpi::{GetWindowDpiAwarenessContext, SetThreadDpiAwarenessContext};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, DestroyWindow,
    DispatchMessageW, FindWindowExW, FindWindowW, GWL_EXSTYLE, GWLP_USERDATA, GetClientRect,
    GetMessageW, GetWindowLongPtrW, GetWindowRect, HWND_TOPMOST, IDC_ARROW, IsWindow,
    IsWindowVisible, LoadCursorW, MSG, PostQuitMessage, RegisterClassW, SW_HIDE, SW_SHOWNOACTIVATE,
    SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetParent, SetTimer,
    SetWindowLongPtrW, SetWindowPos, ShowWindow, TranslateMessage, WM_CLOSE, WM_DESTROY,
    WM_ERASEBKGND, WM_LBUTTONUP, WM_NCCREATE, WM_PAINT, WM_TIMER, WNDCLASSW, WS_EX_NOACTIVATE,
    WS_EX_NOREDIRECTIONBITMAP, WS_EX_TOOLWINDOW, WS_POPUP, WS_VISIBLE,
};

use crate::{
    HelperMessage, HostMessage, Rect, TaskbarAction, TaskbarMode, TaskbarPreferenceMode,
    TaskbarSnapshot, calculate_taskbar_window_rect, composition::CompositionRenderer,
};

const WINDOW_WIDTH: i32 = 320;
const TIMER_ID: usize = 1;
const TIMER_INTERVAL_MS: u32 = 100;
const REPOSITION_TICKS: u32 = 20;
const BUTTON_WIDTH: i32 = 28;
const BUTTON_COUNT: i32 = 5;
const MARQUEE_GAP: i32 = 36;
const MARQUEE_SPEED: i32 = 2;

struct WindowState {
    receiver: Receiver<HostMessage>,
    output: BufWriter<std::io::Stdout>,
    snapshot: TaskbarSnapshot,
    taskbar: HWND,
    mode: TaskbarMode,
    offset_x: i32,
    composition: Option<CompositionRenderer>,
    visible: bool,
    tick: u32,
    marquee_tick: u32,
}

pub fn run_stdio(preference: TaskbarPreferenceMode, offset_x: i32) -> Result<(), String> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let input = std::io::stdin();
        for line in BufReader::new(input.lock()).lines() {
            let Ok(line) = line else {
                break;
            };
            match serde_json::from_str::<HostMessage>(&line) {
                Ok(message) => {
                    if sender.send(message).is_err() {
                        break;
                    }
                }
                Err(error) => eprintln!("[musicplayer-taskbar] 無法解析訊息：{error}"),
            }
        }
        let _ = sender.send(HostMessage::Shutdown);
    });

    // SAFETY: 所有 Win32 handle 都由此 helper process 建立並限制在同一 GUI thread 使用。
    unsafe { run_window_loop(receiver, preference, offset_x) }
}

unsafe fn run_window_loop(
    receiver: Receiver<HostMessage>,
    preference: TaskbarPreferenceMode,
    offset_x: i32,
) -> Result<(), String> {
    let class_name = wide("MusicPlayerTaskbarWindow");
    // SAFETY: null module name 取得目前 helper executable 的 module handle。
    let instance = unsafe { GetModuleHandleW(null()) };
    if instance.is_null() {
        return Err("無法取得 helper module handle".to_string());
    }

    let class = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(window_proc),
        hInstance: instance,
        lpszClassName: class_name.as_ptr(),
        // SAFETY: 載入系統提供的共用游標。
        hCursor: unsafe { LoadCursorW(null_mut(), IDC_ARROW) },
        ..unsafe { std::mem::zeroed() }
    };
    // SAFETY: WNDCLASSW 在此呼叫期間有效。
    if unsafe { RegisterClassW(&raw const class) } == 0 {
        return Err("無法註冊工作列視窗類別".to_string());
    }

    let (taskbar, taskbar_rect, notification_rect) =
        unsafe { locate_primary_taskbar() }.ok_or_else(|| "找不到 Windows 工作列".to_string())?;
    if preference == TaskbarPreferenceMode::Auto {
        // SAFETY: 以工作列所屬 DPI context 建立視窗，避免跨行程 SetParent 因 context 不同失敗。
        let taskbar_dpi_context = unsafe { GetWindowDpiAwarenessContext(taskbar) };
        if !taskbar_dpi_context.is_null() {
            unsafe { SetThreadDpiAwarenessContext(taskbar_dpi_context) };
        }
    }
    let target = calculate_taskbar_window_rect(taskbar_rect, notification_rect, WINDOW_WIDTH)
        .ok_or_else(|| "工作列沒有足夠的水平空間".to_string())?;

    let state = Box::new(WindowState {
        receiver,
        output: BufWriter::new(std::io::stdout()),
        snapshot: TaskbarSnapshot::default(),
        taskbar,
        mode: TaskbarMode::Docked,
        offset_x: offset_x.clamp(-1600, 0),
        composition: None,
        visible: true,
        tick: 0,
        marquee_tick: 0,
    });
    let state_ptr = Box::into_raw(state);
    let title = wide("MusicPlayer");
    let extended_style = if preference == TaskbarPreferenceMode::Auto {
        WS_EX_TOOLWINDOW | WS_EX_NOREDIRECTIONBITMAP
    } else {
        WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_NOREDIRECTIONBITMAP
    };
    // SAFETY: class 已註冊，state_ptr 會在 WM_DESTROY 中回收。
    let hwnd = unsafe {
        CreateWindowExW(
            extended_style,
            class_name.as_ptr(),
            title.as_ptr(),
            WS_POPUP | WS_VISIBLE,
            target.left,
            target.top,
            target.width(),
            target.height(),
            null_mut(),
            null_mut(),
            instance,
            state_ptr.cast(),
        )
    };

    if hwnd.is_null() {
        // SAFETY: 沒有視窗接手 state_ptr。
        drop(unsafe { Box::from_raw(state_ptr) });
        return Err("無法建立工作列播放器視窗".to_string());
    }

    if preference == TaskbarPreferenceMode::Auto {
        // TrafficMonitor 的 Windows 11 路徑也是先建立 top-level 視窗，再掛到 Shell_TrayWnd。
        // 保留 WS_POPUP 樣式可避免以 WS_CHILD 建立時被工作列合成層裁切成空白。
        match unsafe { attach_to_taskbar(hwnd, taskbar) } {
            Ok(()) => unsafe { (*state_ptr).mode = TaskbarMode::Embedded },
            Err(error) => {
                eprintln!("[musicplayer-taskbar] SetParent 失敗，錯誤碼：{error}");
            }
        }
    }

    unsafe { initialize_composition(hwnd, state_ptr, target) };

    // SAFETY: hwnd 是有效視窗，設定 timer 只用於同一 thread 的訊息輪詢。
    unsafe { SetTimer(hwnd, TIMER_ID, TIMER_INTERVAL_MS, None) };
    unsafe { position_window(hwnd, true) };
    unsafe { InvalidateRect(hwnd, null(), 0) };
    let mode = unsafe { (*state_ptr).mode };
    unsafe { write_message(&mut (*state_ptr).output, &HelperMessage::Ready { mode }) };

    let mut message: MSG = unsafe { std::mem::zeroed() };
    // SAFETY: 標準 Win32 message loop。
    while unsafe { GetMessageW(&raw mut message, null_mut(), 0, 0) } > 0 {
        unsafe {
            TranslateMessage(&raw const message);
            DispatchMessageW(&raw const message);
        }
    }
    Ok(())
}

unsafe fn initialize_composition(hwnd: HWND, state_ptr: *mut WindowState, target: Rect) {
    match CompositionRenderer::new(
        hwnd.cast(),
        u32::try_from(target.width()).unwrap_or_default(),
        u32::try_from(target.height()).unwrap_or_default(),
    ) {
        Ok(renderer) => unsafe { (*state_ptr).composition = Some(renderer) },
        Err(error) => {
            eprintln!("[musicplayer-taskbar] {error}");
            unsafe {
                if (*state_ptr).mode == TaskbarMode::Embedded {
                    (*state_ptr).mode = TaskbarMode::Docked;
                    SetParent(hwnd, null_mut());
                }
                disable_no_redirection(hwnd);
            }
        }
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_NCCREATE {
        let create = lparam as *const CREATESTRUCTW;
        // SAFETY: WM_NCCREATE 的 lparam 保證指向 CREATESTRUCTW。
        let state = unsafe { (*create).lpCreateParams.cast::<WindowState>() };
        // SAFETY: 儲存由 run_window_loop 傳入的 Box pointer。
        unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, state as isize) };
    }
    // SAFETY: pointer 僅在 WM_NCCREATE 後、WM_DESTROY 回收前讀取。
    let state_ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *mut WindowState;

    match message {
        WM_TIMER if !state_ptr.is_null() => {
            let state = unsafe { &mut *state_ptr };
            let mut force_reposition = false;
            while let Ok(message) = state.receiver.try_recv() {
                match message {
                    HostMessage::Update { snapshot } => {
                        if state.snapshot.title != snapshot.title
                            || state.snapshot.artists != snapshot.artists
                        {
                            state.marquee_tick = 0;
                        }
                        state.snapshot = snapshot;
                        // SAFETY: hwnd 有效，要求完整重繪。
                        unsafe { InvalidateRect(hwnd, null(), 0) };
                    }
                    HostMessage::SetOffset { offset_x } => {
                        state.offset_x = offset_x.clamp(-1600, 0);
                        force_reposition = true;
                    }
                    HostMessage::SetVisibility { visible } => {
                        state.visible = visible;
                        force_reposition = true;
                    }
                    HostMessage::Shutdown => {
                        // SAFETY: hwnd 由此 thread 擁有。
                        unsafe { DestroyWindow(hwnd) };
                        return 0;
                    }
                }
            }
            state.tick = state.tick.wrapping_add(1);
            state.marquee_tick = state.marquee_tick.wrapping_add(1);
            if state.snapshot.show_title_marquee {
                unsafe { InvalidateRect(hwnd, null(), 0) };
            }
            if force_reposition || state.tick.is_multiple_of(REPOSITION_TICKS) {
                unsafe { position_window(hwnd, force_reposition) };
            }
            0
        }
        WM_PAINT if !state_ptr.is_null() => {
            let state = unsafe { &mut *state_ptr };
            unsafe { paint_window(hwnd, state) };
            0
        }
        WM_LBUTTONUP if !state_ptr.is_null() => {
            let x = i32::from((lparam as u32 & 0xffff) as i16);
            let action = unsafe { action_at(hwnd, x, &(*state_ptr).snapshot) };
            if let Some(action) = action {
                unsafe {
                    write_message(&mut (*state_ptr).output, &HelperMessage::Action { action });
                }
            }
            0
        }
        WM_ERASEBKGND => 1,
        WM_CLOSE => {
            unsafe { DestroyWindow(hwnd) };
            0
        }
        WM_DESTROY => {
            if !state_ptr.is_null() {
                unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) };
                drop(unsafe { Box::from_raw(state_ptr) });
            }
            unsafe { PostQuitMessage(0) };
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

unsafe fn paint_window(hwnd: HWND, state: &mut WindowState) {
    let Some(renderer) = state.composition.as_mut() else {
        unsafe { paint(hwnd, &state.snapshot, state.marquee_tick) };
        return;
    };
    if let Err(error) =
        unsafe { paint_composition(hwnd, &state.snapshot, state.marquee_tick, renderer) }
    {
        let was_embedded = state.mode == TaskbarMode::Embedded;
        unsafe {
            write_message(&mut state.output, &HelperMessage::Error { message: error });
            if was_embedded {
                SetParent(hwnd, null_mut());
            }
            disable_no_redirection(hwnd);
        }
        state.composition = None;
        state.mode = TaskbarMode::Docked;
        unsafe {
            position_window(hwnd, true);
            InvalidateRect(hwnd, null(), 0);
            if was_embedded {
                write_message(
                    &mut state.output,
                    &HelperMessage::Ready {
                        mode: TaskbarMode::Docked,
                    },
                );
            }
        }
    }
    unsafe { validate_paint(hwnd) };
}

unsafe fn locate_primary_taskbar() -> Option<(HWND, Rect, Option<Rect>)> {
    let taskbar_class = wide("Shell_TrayWnd");
    // SAFETY: class name 是 null-terminated UTF-16。
    let taskbar = unsafe { FindWindowW(taskbar_class.as_ptr(), null()) };
    if taskbar.is_null() {
        return None;
    }
    let taskbar_rect = unsafe { window_rect(taskbar)? };
    let notification_class = wide("TrayNotifyWnd");
    // SAFETY: 搜尋 taskbar 的通知區 child window。
    let notification =
        unsafe { FindWindowExW(taskbar, null_mut(), notification_class.as_ptr(), null()) };
    let notification_rect = unsafe { window_rect(notification) };
    Some((taskbar, taskbar_rect, notification_rect))
}

unsafe fn window_rect(hwnd: HWND) -> Option<Rect> {
    if hwnd.is_null() || unsafe { IsWindow(hwnd) } == 0 {
        return None;
    }
    let mut rect: RECT = unsafe { std::mem::zeroed() };
    if unsafe { GetWindowRect(hwnd, &raw mut rect) } == 0 {
        return None;
    }
    Some(Rect {
        left: rect.left,
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
    })
}

unsafe fn position_window(hwnd: HWND, force: bool) {
    let Some((taskbar, taskbar_rect, notification_rect)) = (unsafe { locate_primary_taskbar() })
    else {
        return;
    };
    let Some(target) = calculate_taskbar_window_rect(taskbar_rect, notification_rect, WINDOW_WIDTH)
    else {
        return;
    };
    let state_ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *mut WindowState;
    if state_ptr.is_null() {
        return;
    }
    let state = unsafe { &mut *state_ptr };
    if state.taskbar != taskbar {
        if state.mode == TaskbarMode::Embedded {
            let taskbar_dpi_context = unsafe { GetWindowDpiAwarenessContext(taskbar) };
            if !taskbar_dpi_context.is_null() {
                unsafe { SetThreadDpiAwarenessContext(taskbar_dpi_context) };
            }
            if unsafe { attach_to_taskbar(hwnd, taskbar) }.is_err() {
                state.mode = TaskbarMode::Docked;
            }
        }
        state.taskbar = taskbar;
    }
    let offset_x = state.offset_x.clamp(taskbar_rect.left - target.left, 0);
    let target_left = target.left + offset_x;
    let (x, y, insert_after) = if state.mode == TaskbarMode::Embedded {
        (
            target_left - taskbar_rect.left,
            target.top - taskbar_rect.top,
            null_mut(),
        )
    } else {
        (target_left, target.top, HWND_TOPMOST)
    };
    if force || state.tick.is_multiple_of(REPOSITION_TICKS) {
        unsafe {
            ShowWindow(
                hwnd,
                if state.visible && IsWindowVisible(taskbar) != 0 {
                    SW_SHOWNOACTIVATE
                } else {
                    SW_HIDE
                },
            );
        }
        unsafe {
            SetWindowPos(
                hwnd,
                insert_after,
                x,
                y,
                target.width(),
                target.height(),
                SWP_NOACTIVATE,
            );
            InvalidateRect(hwnd, null(), 0);
        }
    }
}

unsafe fn attach_to_taskbar(hwnd: HWND, taskbar: HWND) -> Result<(), u32> {
    // SetParent 成功時回傳舊父視窗；top-level 視窗的舊父視窗為 null，因此必須搭配
    // GetLastError 判斷，不能只檢查回傳 handle，也不能以 WS_POPUP 的 GetParent 驗證。
    unsafe { SetLastError(0) };
    let previous_parent = unsafe { SetParent(hwnd, taskbar) };
    let error = unsafe { GetLastError() };
    if !previous_parent.is_null() || error == 0 {
        Ok(())
    } else {
        Err(error)
    }
}

unsafe fn disable_no_redirection(hwnd: HWND) {
    let style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
    unsafe {
        SetWindowLongPtrW(
            hwnd,
            GWL_EXSTYLE,
            style & !(isize::try_from(WS_EX_NOREDIRECTIONBITMAP).unwrap_or_default()),
        );
        SetWindowPos(
            hwnd,
            null_mut(),
            0,
            0,
            0,
            0,
            SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
}

unsafe fn paint(hwnd: HWND, snapshot: &TaskbarSnapshot, marquee_tick: u32) {
    let mut paint: PAINTSTRUCT = unsafe { std::mem::zeroed() };
    let hdc = unsafe { BeginPaint(hwnd, &raw mut paint) };
    let mut client: RECT = unsafe { std::mem::zeroed() };
    unsafe { GetClientRect(hwnd, &raw mut client) };
    unsafe { draw_surface(hdc, client, snapshot, marquee_tick, false) };
    unsafe { EndPaint(hwnd, &raw const paint) };
}

unsafe fn validate_paint(hwnd: HWND) {
    let mut paint: PAINTSTRUCT = unsafe { std::mem::zeroed() };
    unsafe {
        BeginPaint(hwnd, &raw mut paint);
        EndPaint(hwnd, &raw const paint);
    }
}

unsafe fn paint_composition(
    hwnd: HWND,
    snapshot: &TaskbarSnapshot,
    marquee_tick: u32,
    renderer: &mut CompositionRenderer,
) -> Result<(), String> {
    let mut client: RECT = unsafe { std::mem::zeroed() };
    unsafe { GetClientRect(hwnd, &raw mut client) };
    let width = u32::try_from(client.right).map_err(|_| "工作列播放器寬度無效")?;
    let height = u32::try_from(client.bottom).map_err(|_| "工作列播放器高度無效")?;
    if width == 0 || height == 0 {
        return Ok(());
    }

    let mut info: BITMAPINFO = unsafe { std::mem::zeroed() };
    info.bmiHeader.biSize = u32::try_from(std::mem::size_of_val(&info.bmiHeader))
        .map_err(|_| "BITMAPINFOHEADER 尺寸無效")?;
    info.bmiHeader.biWidth = i32::try_from(width).map_err(|_| "位圖寬度過大")?;
    info.bmiHeader.biHeight = -i32::try_from(height).map_err(|_| "位圖高度過大")?;
    info.bmiHeader.biPlanes = 1;
    info.bmiHeader.biBitCount = 32;
    info.bmiHeader.biCompression = BI_RGB;

    let mut bits = null_mut();
    let bitmap = unsafe {
        CreateDIBSection(
            null_mut(),
            &raw const info,
            DIB_RGB_COLORS,
            &raw mut bits,
            null_mut(),
            0,
        )
    };
    let memory_dc = unsafe { CreateCompatibleDC(null_mut()) };
    if bitmap.is_null() || memory_dc.is_null() || bits.is_null() {
        if !bitmap.is_null() {
            unsafe { DeleteObject(bitmap) };
        }
        if !memory_dc.is_null() {
            unsafe { DeleteDC(memory_dc) };
        }
        return Err("建立 DirectComposition 記憶體位圖失敗".to_string());
    }

    let previous_bitmap = unsafe { SelectObject(memory_dc, bitmap) };
    unsafe { draw_surface(memory_dc, client, snapshot, marquee_tick, true) };
    let length = width as usize * height as usize * 4;
    let pixels = unsafe { std::slice::from_raw_parts_mut(bits.cast::<u8>(), length) };
    for pixel in pixels.chunks_exact_mut(4) {
        pixel[3] = if pixel[0] == 0 && pixel[1] == 0 && pixel[2] == 0 {
            0
        } else {
            u8::MAX
        };
    }
    let result = renderer.render(pixels, width, height);
    unsafe {
        SelectObject(memory_dc, previous_bitmap);
        DeleteObject(bitmap);
        DeleteDC(memory_dc);
    }
    result
}

unsafe fn draw_surface(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    client: RECT,
    snapshot: &TaskbarSnapshot,
    marquee_tick: u32,
    transparent_background: bool,
) {
    let brush = unsafe {
        CreateSolidBrush(if transparent_background {
            0
        } else {
            0x0020_2020
        })
    };
    let font = unsafe { create_taskbar_font() };
    let selected_font = if font.is_null() {
        unsafe { GetStockObject(DEFAULT_GUI_FONT) }
    } else {
        font
    };
    unsafe {
        FillRect(hdc, &raw const client, brush);
        DeleteObject(brush);
        SetBkMode(hdc, TRANSPARENT.cast_signed());
        SetTextColor(hdc, COLORREF::from(0x00ee_eeee_u32));
    }
    let previous_font = unsafe { SelectObject(hdc, selected_font) };

    unsafe {
        draw_title(hdc, client, snapshot, marquee_tick);
        draw_controls(hdc, client, snapshot);
        draw_progress(hdc, client, snapshot);
        SelectObject(hdc, previous_font);
        if !font.is_null() {
            DeleteObject(font);
        }
    }
}

unsafe fn create_taskbar_font() -> windows_sys::Win32::Graphics::Gdi::HFONT {
    let font_name = wide("Segoe UI Variable Text");
    unsafe {
        CreateFontW(
            -12,
            0,
            0,
            0,
            i32::try_from(FW_NORMAL).unwrap_or(400),
            0,
            0,
            0,
            u32::from(DEFAULT_CHARSET),
            u32::from(OUT_DEFAULT_PRECIS),
            u32::from(CLIP_DEFAULT_PRECIS),
            u32::from(CLEARTYPE_QUALITY),
            u32::from(DEFAULT_PITCH | FF_DONTCARE),
            font_name.as_ptr(),
        )
    }
}

unsafe fn draw_title(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    client: RECT,
    snapshot: &TaskbarSnapshot,
    marquee_tick: u32,
) {
    unsafe { SetTextColor(hdc, COLORREF::from(0x00ee_eeee_u32)) };
    let controls_width = BUTTON_WIDTH * BUTTON_COUNT;
    let mut text_rect = RECT {
        left: 8,
        top: 0,
        right: client.right - controls_width - 4,
        bottom: client.bottom,
    };
    let display = if snapshot.artists.is_empty() {
        snapshot.title.clone()
    } else {
        format!("{} — {}", snapshot.title, snapshot.artists)
    };
    let display = wide(&display);
    let mut text_size: SIZE = unsafe { std::mem::zeroed() };
    let text_length = i32::try_from(display.len().saturating_sub(1)).unwrap_or(i32::MAX);
    unsafe { GetTextExtentPoint32W(hdc, display.as_ptr(), text_length, &raw mut text_size) };
    let viewport_width = text_rect.right - text_rect.left;
    if snapshot.show_title_marquee && text_size.cx > viewport_width {
        let offset = marquee_offset(text_size.cx, viewport_width, marquee_tick);
        let saved_dc = unsafe { SaveDC(hdc) };
        unsafe {
            IntersectClipRect(
                hdc,
                text_rect.left,
                text_rect.top,
                text_rect.right,
                text_rect.bottom,
            );
        }
        for left in [
            text_rect.left - offset,
            text_rect.left - offset + text_size.cx + MARQUEE_GAP,
        ] {
            let mut marquee_rect = RECT {
                left,
                top: text_rect.top,
                right: left + text_size.cx + 1,
                bottom: text_rect.bottom,
            };
            unsafe {
                DrawTextW(
                    hdc,
                    display.as_ptr(),
                    -1,
                    &raw mut marquee_rect,
                    DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
                );
            }
        }
        unsafe { RestoreDC(hdc, saved_dc) };
    } else {
        unsafe {
            DrawTextW(
                hdc,
                display.as_ptr(),
                -1,
                &raw mut text_rect,
                DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
            );
        }
    }
}

unsafe fn draw_controls(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    client: RECT,
    snapshot: &TaskbarSnapshot,
) {
    let controls_width = BUTTON_WIDTH * BUTTON_COUNT;
    let labels = [
        ("|<", snapshot.can_previous),
        (if snapshot.is_playing { "||" } else { ">" }, true),
        (">|", snapshot.can_next),
        ("-", snapshot.volume > 0.0),
        ("+", snapshot.volume < 1.0),
    ];
    for (index, (label, enabled)) in labels.into_iter().enumerate() {
        let left = client.right - controls_width + i32::try_from(index).unwrap_or(0) * BUTTON_WIDTH;
        let mut button_rect = RECT {
            left,
            top: 0,
            right: left + BUTTON_WIDTH,
            bottom: client.bottom,
        };
        unsafe {
            SetTextColor(
                hdc,
                if enabled {
                    COLORREF::from(0x00ff_ffff_u32)
                } else {
                    COLORREF::from(0x0070_7070_u32)
                },
            );
            let label = wide(label);
            DrawTextW(
                hdc,
                label.as_ptr(),
                -1,
                &raw mut button_rect,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
            );
        }
    }
}

unsafe fn draw_progress(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    client: RECT,
    snapshot: &TaskbarSnapshot,
) {
    if !snapshot.show_progress
        || !snapshot.duration_secs.is_finite()
        || snapshot.duration_secs <= 0.0
    {
        return;
    }
    let top = (client.bottom - 2).max(client.top);
    let track_rect = RECT {
        left: client.left,
        top,
        right: client.right,
        bottom: client.bottom,
    };
    let progress_rect = RECT {
        right: client.left
            + progress_width(
                client.right - client.left,
                snapshot.position_secs,
                snapshot.duration_secs,
            ),
        ..track_rect
    };
    let track_brush = unsafe { CreateSolidBrush(0x0048_4848) };
    let progress_brush = unsafe { CreateSolidBrush(0x00ee_eeee) };
    unsafe {
        FillRect(hdc, &raw const track_rect, track_brush);
        FillRect(hdc, &raw const progress_rect, progress_brush);
        DeleteObject(track_brush);
        DeleteObject(progress_brush);
    }
}

fn marquee_offset(text_width: i32, viewport_width: i32, tick: u32) -> i32 {
    if text_width <= viewport_width || text_width <= 0 {
        return 0;
    }
    let distance = text_width.saturating_add(MARQUEE_GAP);
    i32::try_from(tick)
        .unwrap_or(i32::MAX)
        .saturating_mul(MARQUEE_SPEED)
        .rem_euclid(distance)
}

fn progress_width(total_width: i32, position_secs: f64, duration_secs: f64) -> i32 {
    if total_width <= 0
        || !position_secs.is_finite()
        || !duration_secs.is_finite()
        || duration_secs <= 0.0
    {
        return 0;
    }
    let ratio = (position_secs / duration_secs).clamp(0.0, 1.0);
    (f64::from(total_width) * ratio).round() as i32
}

unsafe fn action_at(hwnd: HWND, x: i32, snapshot: &TaskbarSnapshot) -> Option<TaskbarAction> {
    let mut client: RECT = unsafe { std::mem::zeroed() };
    unsafe { GetClientRect(hwnd, &raw mut client) };
    let controls_left = client.right - BUTTON_WIDTH * BUTTON_COUNT;
    if x < controls_left {
        return Some(TaskbarAction::OpenMainWindow);
    }
    match (x - controls_left) / BUTTON_WIDTH {
        0 if snapshot.can_previous => Some(TaskbarAction::Previous),
        1 => Some(TaskbarAction::PlayPause),
        2 if snapshot.can_next => Some(TaskbarAction::Next),
        3 if snapshot.volume > 0.0 => Some(TaskbarAction::AdjustVolume(-0.05)),
        4 if snapshot.volume < 1.0 => Some(TaskbarAction::AdjustVolume(0.05)),
        _ => None,
    }
}

unsafe fn write_message(output: &mut BufWriter<std::io::Stdout>, message: &HelperMessage) {
    if serde_json::to_writer(&mut *output, message).is_ok() {
        let _ = output.write_all(b"\n");
        let _ = output.flush();
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marquee_only_moves_overflowing_text_and_wraps() {
        assert_eq!(marquee_offset(100, 120, 50), 0);
        assert_eq!(marquee_offset(200, 120, 1), 2);
        assert_eq!(marquee_offset(200, 120, 118), 0);
    }

    #[test]
    fn progress_width_clamps_invalid_and_out_of_range_values() {
        assert_eq!(progress_width(320, 45.0, 180.0), 80);
        assert_eq!(progress_width(320, -10.0, 180.0), 0);
        assert_eq!(progress_width(320, 200.0, 180.0), 320);
        assert_eq!(progress_width(320, 10.0, 0.0), 0);
        assert_eq!(progress_width(320, f64::NAN, 180.0), 0);
    }
}
