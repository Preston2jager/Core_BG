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

use windows_sys::Win32::Graphics::Gdi::{EnumDisplayMonitors, HMONITOR, HDC};

pub struct MonitorBounds {
    pub rect: RECT,
}

pub unsafe fn get_monitor_bounds() -> Vec<MonitorBounds> {
    unsafe extern "system" fn enum_monitors_proc(
        _hmonitor: HMONITOR,
        _hdc: HDC,
        rect: *mut RECT,
        lparam: LPARAM,
    ) -> BOOL {
        let monitors = &mut *(lparam as *mut Vec<MonitorBounds>);
        monitors.push(MonitorBounds { rect: *rect });
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
    let virtual_left = GetSystemMetrics(SM_XVIRTUALSCREEN);
    let virtual_top = GetSystemMetrics(SM_YVIRTUALSCREEN);
    let virtual_width = GetSystemMetrics(SM_CXVIRTUALSCREEN);
    let virtual_height = GetSystemMetrics(SM_CYVIRTUALSCREEN);
    crate::app::log_msg(&format!("Virtual Screen: ({}, {}) {}x{}", virtual_left, virtual_top, virtual_width, virtual_height));

    let parent_hwnd = desktop_info.parent_hwnd;

    if parent_hwnd != 0 {
        let mut parent_rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        GetWindowRect(parent_hwnd, &mut parent_rect);
        let parent_w = parent_rect.right - parent_rect.left;
        let parent_h = parent_rect.bottom - parent_rect.top;
        // Only resize if it's already reasonably large or if it's the known Progman/WorkerW
        // Avoid resizing tiny utility windows that we might have picked up by mistake
        let is_likely_wallpaper_window = parent_w > 500 && parent_h > 500;
        
        if (parent_rect.left != virtual_left || parent_rect.top != virtual_top ||
           parent_w != virtual_width || parent_h != virtual_height) && is_likely_wallpaper_window {
            crate::app::log_msg(&format!("Resizing parent window {} from ({}, {}) {}x{} to cover virtual screen ({}, {}) {}x{}",
                parent_hwnd, parent_rect.left, parent_rect.top, parent_w, parent_h,
                virtual_left, virtual_top, virtual_width, virtual_height));
            SetWindowPos(
                parent_hwnd,
                0,
                virtual_left,
                virtual_top,
                virtual_width,
                virtual_height,
                SWP_NOZORDER | SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
        }
    }

    let current_monitors = get_monitor_bounds();

    if current_monitors.len() != monitor_states.len() {
        crate::app::log_msg(&format!("Monitor count changed from {} to {}. Re-initializing windows.", monitor_states.len(), current_monitors.len()));
        
        while let Some(state) = monitor_states.pop() {
            let hwnd = state.hwnd;
            drop(state); // Drop the state, which drops the Renderer and the Surface, while the hwnd is still valid!
            DestroyWindow(hwnd); // Now destroy the window!
        }

        let class_name_str = WALLPAPER_CLASS_NAME.lock().unwrap().clone();
        let class_name = to_wide(&class_name_str);
        for (idx, monitor) in current_monitors.iter().enumerate() {
            let rect = monitor.rect;
            let x = rect.left - virtual_left;
            let y = rect.top - virtual_top;
            let width = rect.right - rect.left;
            let height = rect.bottom - rect.top;
            let label = if parent_hwnd == desktop_info.parent_hwnd {
                "WORKERW"
            } else if parent_hwnd != 0 {
                "PROGMAN"
            } else {
                "TOP"
            };
            crate::app::log_msg(&format!("Attempting to create window for monitor {} at ({}, {}) with parent {} ({})", idx, x, y, parent_hwnd, label));

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
                0, // Create as top-level initially
                0,
                hinstance,
                std::ptr::null(),
            );

            if hwnd != 0 {
                crate::app::log_msg(&format!("Successfully created window {} for monitor {}.", hwnd, idx));
                
                let (render_w, render_h) = compute_renderer_size(width, height);

                // Initialize shared resources on the first successful window if not already done
                if shared_resources.is_none() {
                    let hwnd_val = std::num::NonZeroIsize::new(hwnd as isize).unwrap();
                    let mut win_handle = raw_window_handle::Win32WindowHandle::new(hwnd_val);
                    win_handle.hinstance = std::num::NonZeroIsize::new(hinstance as isize);
                    let temp_surface = unsafe { instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle { 
                        raw_display_handle: raw_window_handle::RawDisplayHandle::Windows(raw_window_handle::WindowsDisplayHandle::new()),
                        raw_window_handle: raw_window_handle::RawWindowHandle::Win32(win_handle),
                    })}.expect("Failed to create temporary surface for format detection");
                    
                    let caps = temp_surface.get_capabilities(adapter);
                    let format = caps.formats.iter().copied().find(|f| f.is_srgb()).unwrap_or(caps.formats[0]);
                    
                    drop(temp_surface); // release surface before initializing resource
 
                    *shared_resources = Some(std::sync::Arc::new(crate::renderer::SharedRenderResources::new(
                        device,
                        queue,
                        format,
                    )));
                }
 
                let renderer = crate::renderer::Renderer::new(
                    instance,
                    adapter,
                    device,
                    hwnd,
                    hinstance,
                    shared_resources.as_ref().unwrap().clone(),
                    render_w,
                    render_h,
                    width as f32,
                    height as f32,
                );

                crate::app::log_msg(&format!("Parenting window {} to parent {}", hwnd, parent_hwnd));
                SetParent(hwnd, parent_hwnd);

                // Give the OS a moment to stabilize the hierarchy
                std::thread::sleep(std::time::Duration::from_millis(100));

                // Find the shell view sibling to place our window behind it
                let shell_view = FindWindowExW(parent_hwnd, 0, to_wide("SHELLDLL_DefView").as_ptr(), std::ptr::null());
                let insert_after = if shell_view != 0 {
                    crate::app::log_msg(&format!("Found shell view sibling {}. Placing wallpaper behind it.", shell_view));
                    shell_view // Place AFTER shell_view (i.e., behind it)
                } else {
                    1 // HWND_BOTTOM
                };

                // Ensure it is at the bottom of the z-order among children
                SetWindowPos(
                    hwnd,
                    insert_after,
                    x,
                    y,
                    width,
                    height,
                    SWP_NOACTIVATE | SWP_SHOWWINDOW,
                );

                monitor_states.push(crate::app::MonitorState {
                    hwnd,
                    rect,
                    width,
                    height,
                    renderer,
                });
            } else {
                let err = unsafe { GetLastError() };
                crate::app::log_msg(&format!("Failed to create window for monitor at ({}, {}): GetLastError={}", rect.left, rect.top, err));
            }
        }
    } else {
        for (i, monitor) in current_monitors.iter().enumerate() {
            let rect = monitor.rect;
            let x = rect.left - virtual_left;
            let y = rect.top - virtual_top;
            let w = rect.right - rect.left;
            let h = rect.bottom - rect.top;
 
            let state = &mut monitor_states[i];
            if state.rect.left != rect.left || state.rect.top != rect.top ||
               (state.width != w || state.height != h) {
                
                crate::app::log_msg(&format!("Monitor {} resized/moved from ({}, {}) {}x{} to ({}, {}) {}x{}. Adjusting window.", 
                    i, state.rect.left, state.rect.top, state.width, state.height, rect.left, rect.top, w, h));
                
                SetWindowPos(state.hwnd, 1, x, y, w, h, SWP_NOACTIVATE);
                state.rect = rect;
                state.width = w;
                state.height = h;
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

