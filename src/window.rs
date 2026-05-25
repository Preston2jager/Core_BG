use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

pub fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

pub unsafe fn enable_dpi_awareness() {
    let user32_name = b"user32.dll\0";
    let user32 = windows_sys::Win32::System::LibraryLoader::GetModuleHandleA(user32_name.as_ptr());
    if user32 != 0 {
        let set_context_name = b"SetProcessDpiAwarenessContext\0";
        let func = windows_sys::Win32::System::LibraryLoader::GetProcAddress(user32, set_context_name.as_ptr());
        if let Some(set_context) = func {
            let set_context: extern "system" fn(isize) -> BOOL = std::mem::transmute(set_context);
            if set_context(-4) != 0 {
                return;
            }
        }
        
        let set_aware_name = b"SetProcessDPIAware\0";
        let func = windows_sys::Win32::System::LibraryLoader::GetProcAddress(user32, set_aware_name.as_ptr());
        if let Some(set_aware) = func {
            let set_aware: extern "system" fn() -> BOOL = std::mem::transmute(set_aware);
            set_aware();
        }
    }
}




pub struct DesktopInfo {
    pub parent_hwnd: HWND,
}

pub unsafe fn get_desktop_info() -> DesktopInfo {
    let progman = FindWindowW(to_wide("Progman").as_ptr(), std::ptr::null());
    
    SendMessageTimeoutW(
        progman,
        0x052C,
        0,
        0,
        SMTO_NORMAL,
        1000,
        std::ptr::null_mut(),
    );

    std::thread::sleep(std::time::Duration::from_millis(1000));

    struct WindowLayer {
        hwnd: HWND,
        class: String,
        has_shell: bool,
    }
    let mut layers: Vec<WindowLayer> = Vec::new();

    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let layers = &mut *(lparam as *mut Vec<WindowLayer>);
        let mut class_buf = [0u16; 256];
        let len = GetClassNameW(hwnd, class_buf.as_mut_ptr(), 256);
        if len > 0 {
            let class_name = String::from_utf16_lossy(&class_buf[..len as usize]);
            if class_name == "WorkerW" || class_name == "Progman" {
                let shell = FindWindowExW(hwnd, 0, to_wide("SHELLDLL_DefView").as_ptr(), std::ptr::null());
                layers.push(WindowLayer {
                    hwnd,
                    class: class_name,
                    has_shell: shell != 0,
                });
            }
        }
        1
    }
    
    EnumWindows(Some(enum_proc), &mut layers as *mut Vec<WindowLayer> as LPARAM);

    let mut shell_layer_idx = None;
    for (i, layer) in layers.iter().enumerate() {
        if layer.has_shell {
            shell_layer_idx = Some(i);
        }
    }

    let mut target_parent = 0;
    if let Some(idx) = shell_layer_idx {
        for i in (idx + 1)..layers.len() {
            if layers[i].class == "WorkerW" {
                target_parent = layers[i].hwnd;
                break;
            }
        }
        if target_parent == 0 {
            target_parent = layers[idx].hwnd;
        }
    }

    if target_parent == 0 {
        target_parent = progman;
    }

    DesktopInfo {
        parent_hwnd: target_parent,
    }
}

pub unsafe fn register_tray_class(hinstance: HINSTANCE, wnd_proc: WNDPROC) -> BOOL {
    let class_name = to_wide("BGCoreV2TrayClass");
    let wc = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: wnd_proc,
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: hinstance,
        hIcon: 0,
        hCursor: LoadCursorW(0, IDC_ARROW),
        hbrBackground: 0,
        lpszMenuName: std::ptr::null(),
        lpszClassName: class_name.as_ptr(),
    };
    if RegisterClassW(&wc) == 0 {
        let err = GetLastError();
        if err != 1410 { // Class already exists is fine
            crate::app::log_msg(&format!("Failed to register tray class: {}", err));
            return 0;
        }
    }
    1
}

use windows_sys::Win32::Graphics::Gdi::{EnumDisplayMonitors, HMONITOR, HDC, GetMonitorInfoW, MONITORINFO};
use windows_sys::Win32::Graphics::Gdi::ScreenToClient;

pub struct MonitorBounds {
    pub rect: RECT,
}

pub unsafe fn get_monitor_bounds() -> Vec<MonitorBounds> {
    unsafe extern "system" fn enum_monitors_proc(
        hmonitor: HMONITOR,
        _hdc: HDC,
        _rect: *mut RECT,
        lparam: LPARAM,
    ) -> BOOL {
        let monitors = &mut *(lparam as *mut Vec<MonitorBounds>);
        
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(hmonitor, &mut info) != 0 {
            monitors.push(MonitorBounds { rect: info.rcMonitor });
        }
        1 // continue enumeration
    }

    let mut monitors = Vec::new();
    EnumDisplayMonitors(
        0,
        std::ptr::null(),
        Some(enum_monitors_proc),
        &mut monitors as *mut Vec<MonitorBounds> as LPARAM,
    );
    monitors
}

pub fn compute_renderer_size(win_w: i32, win_h: i32) -> (usize, usize) {
    let render_w = win_w as usize;
    let render_h = win_h as usize;
    (render_w.max(1), render_h.max(1))
}

pub unsafe fn sync_monitor_windows(
    instance: &wgpu::Instance,
    adapter: &wgpu::Adapter,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    hinstance: HINSTANCE,
    monitor_states: &mut Vec<crate::app::MonitorState>,
    shared_resources: &mut Option<std::sync::Arc<crate::renderer::SharedRenderResources>>,
    desktop_info: &DesktopInfo,
) {
    let current_monitors = get_monitor_bounds();
    let parent_hwnd = desktop_info.parent_hwnd;

    // Compute virtual screen bounds in physical pixels
    let mut v_left = 0;
    let mut v_top = 0;
    let mut v_right = 0;
    let mut v_bottom = 0;
    if !current_monitors.is_empty() {
        v_left = current_monitors[0].rect.left;
        v_top = current_monitors[0].rect.top;
        v_right = current_monitors[0].rect.right;
        v_bottom = current_monitors[0].rect.bottom;
        for m in &current_monitors[1..] {
            v_left = v_left.min(m.rect.left);
            v_top = v_top.min(m.rect.top);
            v_right = v_right.max(m.rect.right);
            v_bottom = v_bottom.max(m.rect.bottom);
        }
    }
    let v_rect = RECT { left: v_left, top: v_top, right: v_right, bottom: v_bottom };

    if current_monitors.len() != monitor_states.len() {
        crate::app::log_msg(&format!("Monitor count changed from {} to {}. Re-initializing windows.", monitor_states.len(), current_monitors.len()));
        
        while let Some(state) = monitor_states.pop() {
            let hwnd = state.hwnd;
            drop(state);
            DestroyWindow(hwnd);
        }

        let class_name_str = WALLPAPER_CLASS_NAME.lock().unwrap().clone();
        let class_name = to_wide(&class_name_str);
        for (idx, monitor) in current_monitors.iter().enumerate() {
            let rect = monitor.rect;
            
            let mut pt = POINT { x: rect.left, y: rect.top };
            ScreenToClient(parent_hwnd, &mut pt);
            let x = pt.x;
            let y = pt.y;
            let width = rect.right - rect.left;
            let height = rect.bottom - rect.top;
            
            crate::app::log_msg(&format!("Monitor {} at screen ({}, {}), relative to parent client ({}, {})", idx, rect.left, rect.top, x, y));

            let (ex_style, style) = (WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE, WS_POPUP);

            let hwnd = CreateWindowExW(
                ex_style,
                class_name.as_ptr(),
                to_wide(&format!("CPU Live Wallpaper {}", idx)).as_ptr(),
                style,
                x,
                y,
                width,
                height,
                0,
                0,
                hinstance,
                std::ptr::null(),
            );

            if hwnd != 0 {
                let (render_w, render_h) = compute_renderer_size(width, height);

                if shared_resources.is_none() {
                    let hwnd_val = std::num::NonZeroIsize::new(hwnd as isize).unwrap();
                    let mut win_handle = raw_window_handle::Win32WindowHandle::new(hwnd_val);
                    win_handle.hinstance = std::num::NonZeroIsize::new(hinstance as isize);
                    let temp_surface = unsafe { instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle { 
                        raw_display_handle: raw_window_handle::RawDisplayHandle::Windows(raw_window_handle::WindowsDisplayHandle::new()),
                        raw_window_handle: raw_window_handle::RawWindowHandle::Win32(win_handle),
                    })}.expect("Failed to create temporary surface");
                    
                    let caps = temp_surface.get_capabilities(adapter);
                    let format = caps.formats.iter().copied().find(|f| f.is_srgb()).unwrap_or(caps.formats[0]);
                    drop(temp_surface);
 
                    *shared_resources = Some(std::sync::Arc::new(crate::renderer::SharedRenderResources::new(
                        device,
                        queue,
                        format,
                    )));
                }
 
                let renderer = crate::renderer::Renderer::new(
                    instance, adapter, device, hwnd, hinstance,
                    shared_resources.as_ref().unwrap().clone(),
                    render_w, render_h, width as f32, height as f32,
                    rect,
                    v_rect,
                );

                SetParent(hwnd, parent_hwnd);
                std::thread::sleep(std::time::Duration::from_millis(100));

                let shell_view = FindWindowExW(parent_hwnd, 0, to_wide("SHELLDLL_DefView").as_ptr(), std::ptr::null());
                let insert_after = if shell_view != 0 { shell_view } else { 1 };

                SetWindowPos(hwnd, insert_after, x, y, width, height, SWP_NOACTIVATE | SWP_SHOWWINDOW);

                monitor_states.push(crate::app::MonitorState { hwnd, rect, width, height, renderer });
            }
        }
    } else {
        for (i, monitor) in current_monitors.iter().enumerate() {
            let rect = monitor.rect;
            let mut pt = POINT { x: rect.left, y: rect.top };
            ScreenToClient(parent_hwnd, &mut pt);
            let x = pt.x;
            let y = pt.y;
            let w = rect.right - rect.left;
            let h = rect.bottom - rect.top;
 
            let state = &mut monitor_states[i];
            if state.rect.left != rect.left || state.rect.top != rect.top ||
               (state.width != w || state.height != h) {
                SetWindowPos(state.hwnd, 1, x, y, w, h, SWP_NOACTIVATE);
                state.rect = rect;
                state.width = w;
                state.height = h;
                state.renderer.resize(device, w as usize, h as usize, rect, v_rect);
            }
        }
    }
}


pub unsafe fn register_wallpaper_class(hinstance: HINSTANCE, wnd_proc: WNDPROC) -> BOOL {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let class_id: u32 = rng.gen();
    let class_name_str = format!("BGCoreV2WallpaperClass_{}", class_id);
    let class_name = to_wide(&class_name_str);
    
    // Store class name globally or pass it around? 
    // For now, let's just use a fixed one but UNREGISTER it first if possible,
    // or better, just use a timestamped one.
    
    let wc = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: wnd_proc,
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: hinstance,
        hIcon: 0,
        hCursor: LoadCursorW(0, IDC_ARROW),
        hbrBackground: 0,
        lpszMenuName: std::ptr::null(),
        lpszClassName: class_name.as_ptr(),
    };
    let res = RegisterClassW(&wc);
    if res == 0 {
        crate::app::log_msg(&format!("Failed to register window class: {}", GetLastError()));
        0
    } else {
        crate::app::log_msg(&format!("Successfully registered window class: {}", class_name_str));
        // We need to use this same name in sync_monitor_windows.
        // Let's store it in a static Mutex.
        *WALLPAPER_CLASS_NAME.lock().unwrap() = class_name_str;
        1
    }
}

use std::sync::Mutex;
pub static WALLPAPER_CLASS_NAME: Mutex<String> = Mutex::new(String::new());

