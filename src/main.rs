#![windows_subsystem = "windows"]

mod window;
mod cpu;
mod tray;
mod renderer;
mod app;
mod settings_win;

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::Sleep;
use crate::app::{log_msg, STATE, WallpaperApp};
use crate::window::to_wide;

static mut APP_PTR: *mut WallpaperApp = std::ptr::null_mut();

#[link(name = "User32")]
#[link(name = "Gdi32")]
extern "system" {}

#[link(name = "Kernel32")]
extern "system" {
    fn CreateFileW(
        lpFileName: *const u16,
        dwDesiredAccess: u32,
        dwShareMode: u32,
        lpSecurityAttributes: *const std::ffi::c_void,
        dwCreationDisposition: u32,
        dwFlagsAndAttributes: u32,
        hTemplateFile: HANDLE,
    ) -> HANDLE;
    fn SetStdHandle(nStdHandle: u32, hHandle: HANDLE) -> BOOL;
}

const STD_INPUT_HANDLE: u32 = -10i32 as u32;
const STD_OUTPUT_HANDLE: u32 = -11i32 as u32;
const STD_ERROR_HANDLE: u32 = -12i32 as u32;
const GENERIC_READ: u32 = 0x80000000;
const GENERIC_WRITE: u32 = 0x40000000;
const FILE_SHARE_READ: u32 = 1;
const FILE_SHARE_WRITE: u32 = 2;
const OPEN_EXISTING: u32 = 3;

// Wallpaper window proc (runs on main thread)
unsafe extern "system" fn wnd_proc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match message {
        WM_NCHITTEST => HTTRANSPARENT as isize,
        WM_MOUSEACTIVATE => MA_NOACTIVATE as isize,
        WM_DESTROY => {
            log_msg("Wallpaper window destroyed.");
            0
        }
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            let _hdc = BeginPaint(hwnd, &mut ps);
            EndPaint(hwnd, &ps);
            0
        }
        _ => DefWindowProcW(hwnd, message, wparam, lparam),
    }
}

// Tray window proc (runs on dedicated background thread)
unsafe extern "system" fn tray_wnd_proc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match message {
        WM_DESTROY => {
            log_msg("Tray window destroyed. Exiting tray thread.");
            STATE.lock().unwrap().should_exit = true;
            PostQuitMessage(0);
            0
        }
        tray::WM_TRAY_CALLBACK => {
            let event = lparam as u32;
            if event == WM_RBUTTONUP || event == WM_LBUTTONUP {
                let bg_effect_enabled = {
                    let state = STATE.lock().unwrap();
                    state.bg_effect_enabled
                };
                tray::show_context_menu(hwnd, bg_effect_enabled);
            }
            0
        }
        WM_COMMAND => {
            let id = (wparam & 0xFFFF) as usize;
            log_msg(&format!("TrayWndProc: WM_COMMAND received, id = {}", id));
            match id {
                tray::ID_EXIT => {
                    log_msg("Menu: Exit clicked");
                    STATE.lock().unwrap().should_exit = true;
                    PostQuitMessage(0);
                }
                tray::ID_REFRESH => {
                    log_msg("Menu: Refresh Wallpaper clicked");
                    STATE.lock().unwrap().pending_refresh = true;
                }
                tray::ID_SETTINGS => {
                    log_msg("Menu: Color Presets clicked");
                    STATE.lock().unwrap().pending_settings_show = true;
                }
                tray::ID_BG_EFFECT => {
                    let mut state = STATE.lock().unwrap();
                    state.bg_effect_enabled = !state.bg_effect_enabled;
                    log_msg(&format!("Menu: Toggle Wallpaper Load Effect, now = {}", state.bg_effect_enabled));
                    app::save_settings(&state);
                }
                _ => {}
            }
            0
        }
        _ => DefWindowProcW(hwnd, message, wparam, lparam),
    }
}

unsafe fn redirect_std_handles_to_nul() {
    #[link(name = "msvcrt")]
    extern "C" {
        fn _open(filename: *const u8, oflag: i32, pmode: i32) -> i32;
        fn _dup2(fd1: i32, fd2: i32) -> i32;
    }

    let nul_name = to_wide("NUL");
    let h_nul = CreateFileW(
        nul_name.as_ptr(),
        GENERIC_READ | GENERIC_WRITE,
        FILE_SHARE_READ | FILE_SHARE_WRITE,
        std::ptr::null(),
        OPEN_EXISTING,
        0,
        0,
    );
    if h_nul != -1i32 as isize {
        SetStdHandle(STD_INPUT_HANDLE, h_nul);
        SetStdHandle(STD_OUTPUT_HANDLE, h_nul);
        SetStdHandle(STD_ERROR_HANDLE, h_nul);
    }

    let fd = _open(b"NUL\0".as_ptr(), 2, 0);
    if fd >= 0 {
        _dup2(fd, 0);
        _dup2(fd, 1);
        _dup2(fd, 2);
    }
}

fn main() {
    std::env::set_var("WGPU_DX12_PRESENTATION_SYSTEM", "Visual");
    unsafe {
        redirect_std_handles_to_nul();
    }
    std::panic::set_hook(Box::new(|info| {
        let payload = info.payload();
        let msg = if let Some(s) = payload.downcast_ref::<&str>() {
            *s
        } else if let Some(s) = payload.downcast_ref::<String>() {
            s.as_str()
        } else {
            "No string payload"
        };
        let location = info.location().map(|loc| format!("{}:{}:{}", loc.file(), loc.line(), loc.column())).unwrap_or_else(|| "unknown location".to_string());
        log_msg(&format!("PANIC occurred at {}: {}", location, msg));
    }));
    unsafe {
        window::enable_dpi_awareness();
    }

    let _ = std::fs::File::create("wallpaper.log");
    log_msg("Application starting");

    // Load settings from settings.txt at startup
    {
        let loaded = app::load_settings();
        let mut state = STATE.lock().unwrap();
        *state = loaded;
    }

    unsafe {
        // Set process priority to High so it remains fluid when CPU is 100%
        use windows_sys::Win32::System::Threading::*;
        SetPriorityClass(GetCurrentProcess(), HIGH_PRIORITY_CLASS);
        windows_sys::Win32::Media::timeBeginPeriod(1);
    }

    let hinstance = unsafe { GetModuleHandleW(std::ptr::null()) };

    // Register wallpaper window class
    unsafe {
        if window::register_wallpaper_class(hinstance, Some(wnd_proc)) == 0 {
            return;
        }
    }

    let mut app = WallpaperApp::new(hinstance);
    unsafe {
        APP_PTR = &mut app;
    }
    
    // Spawn dedicated background thread for the system tray
    std::thread::spawn(move || unsafe {
        log_msg("Tray background thread started");
        if window::register_tray_class(hinstance, Some(tray_wnd_proc)) == 0 {
            log_msg("Failed to register tray window class");
            return;
        }
        let tray_hwnd = CreateWindowExW(
            0,
            to_wide("BGCoreV2TrayClass").as_ptr(),
            to_wide("BGCoreV2TrayWindow").as_ptr(),
            WS_POPUP,
            0,
            0,
            0,
            0,
            0,
            0,
            hinstance,
            std::ptr::null(),
        );
        if tray_hwnd != 0 {
            tray::add_tray_icon(tray_hwnd);
            let mut msg = MSG {
                hwnd: 0,
                message: 0,
                wParam: 0,
                lParam: 0,
                time: 0,
                pt: POINT { x: 0, y: 0 },
            };
            while GetMessageW(&mut msg, 0, 0, 0) != 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            tray::remove_tray_icon(tray_hwnd);
            DestroyWindow(tray_hwnd);
            log_msg("Tray background thread exiting message loop cleanly");
        } else {
            log_msg("Failed to create tray window");
        }
    });
    
    if app.desktop_info.parent_hwnd == 0 {
        log_msg("Startup Error: Failed to find desktop background parent window");
        unsafe {
            MessageBoxW(
                0,
                to_wide("Failed to find desktop background parent window").as_ptr(),
                to_wide("Wallpaper Startup Error").as_ptr(),
                MB_ICONERROR | MB_OK,
            );
        }
        return;
    }

    app.sync_monitors();
    app.update_logo();

    if app.monitor_states.is_empty() {
        log_msg("Startup Error: Failed to create initial wallpaper windows");
        unsafe {
            MessageBoxW(
                0,
                to_wide("Failed to create initial wallpaper windows").as_ptr(),
                to_wide("Wallpaper Startup Error").as_ptr(),
                MB_ICONERROR | MB_OK,
            );
        }
        return;
    }

    let mut msg = MSG {
        hwnd: 0,
        message: 0,
        wParam: 0,
        lParam: 0,
        time: 0,
        pt: POINT { x: 0, y: 0 },
    };

    let mut last_tick = std::time::Instant::now();
    let mut last_cpu_poll = std::time::Instant::now();
    let mut frame_count = 0;
    let mut log_timer = std::time::Instant::now();

    app.cpu_monitor.refresh();
    let mut overall_cpu = app.cpu_monitor.get_overall_usage();
    let mut core_usages = app.cpu_monitor.get_core_usages();

    log_msg("Entering event loop");

    while msg.message != WM_QUIT {
        unsafe {
            while PeekMessageW(&mut msg, 0, 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        // Check thread communication flags
        let (pending_refresh, pending_logo_update, pending_settings_show) = {
            let mut state = STATE.lock().unwrap();
            let r = (state.pending_refresh, state.pending_logo_update, state.pending_settings_show);
            state.pending_refresh = false;
            state.pending_logo_update = false;
            state.pending_settings_show = false;
            r
        };
        if pending_refresh {
            app.reload_wallpaper();
        }
        if pending_logo_update {
            app.update_logo();
        }
        if pending_settings_show {
            unsafe {
                let hinst = GetModuleHandleW(std::ptr::null());
                settings_win::show_settings_window(hinst);
            }
        }

        if STATE.lock().unwrap().should_exit {
            log_msg("Event loop: Exit requested via state");
            break;
        }

        let (is_paused, fps, glow) = {
            let state = STATE.lock().unwrap();
            (state.is_paused, state.fps, state.glow)
        };

        if is_paused {
            unsafe { Sleep(100) };
            continue;
        }

        let target_duration = std::time::Duration::from_nanos(1_000_000_000 / fps as u64);
        let now = std::time::Instant::now();

        if now.duration_since(last_tick) > target_duration * 3 {
            last_tick = now - target_duration;
        }

        let elapsed = now.duration_since(last_tick);

        if elapsed >= target_duration {
            let delta_time = target_duration.as_secs_f32();
            last_tick += target_duration;
            frame_count += 1;

            if now.duration_since(last_cpu_poll) >= std::time::Duration::from_secs(1) {
                app.cpu_monitor.refresh();
                overall_cpu = app.cpu_monitor.get_overall_usage();
                core_usages = app.cpu_monitor.get_core_usages();
                last_cpu_poll = now;

                // Monitor check logic
                let monitors_changed = unsafe {
                    let current = window::get_monitor_bounds();
                    if current.len() != app.monitor_states.len() {
                        true
                    } else {
                        let mut changed = false;
                        for (i, m) in current.iter().enumerate() {
                            if app.monitor_states[i].rect.left != m.rect.left
                                || app.monitor_states[i].rect.top != m.rect.top
                                || app.monitor_states[i].rect.right != m.rect.right
                                || app.monitor_states[i].rect.bottom != m.rect.bottom
                            {
                                changed = true;
                                break;
                            }
                        }
                        changed
                    }
                };

                let parent_invalid = unsafe {
                    IsWindow(app.desktop_info.parent_hwnd) == 0
                };

                if monitors_changed || parent_invalid {
                    log_msg("Monitor layout or parent window changed. Re-syncing windows.");
                    app.desktop_info = unsafe { window::get_desktop_info() };
                    app.sync_monitors();
                }
            }

            app.update_and_draw(delta_time, overall_cpu, &core_usages, glow);

            if now.duration_since(log_timer) >= std::time::Duration::from_secs(5) {
                if !app.monitor_states.is_empty() {
                    let first = &app.monitor_states[0];
                    log_msg(&format!(
                        "Stats: Rendered {} frames in 5s. Screens: {}. Primary: {}x{}. CPU: {:.1}%",
                        frame_count,
                        app.monitor_states.len(),
                        first.width,
                        first.height,
                        overall_cpu
                    ));
                }
                frame_count = 0;
                log_timer = now;
            }
        } else {
            let remaining = target_duration - elapsed;
            if remaining.as_millis() > 3 {
                unsafe {
                    Sleep((remaining.as_millis() - 2) as u32);
                }
            } else {
                std::hint::spin_loop();
            }
        }
    }

    log_msg("Exited event loop");
    unsafe {
        while let Some(state) = app.monitor_states.pop() {
            let hwnd = state.hwnd;
            drop(state);
            DestroyWindow(hwnd);
        }
        windows_sys::Win32::Media::timeEndPeriod(1);
    }
    log_msg("Application terminated cleanly");
}
