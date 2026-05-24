use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use windows_sys::Win32::UI::Shell::*;
use super::window::to_wide;

pub const WM_TRAY_CALLBACK: u32 = WM_USER + 1;

// Menu Item IDs
pub const ID_EXIT: usize = 1001;
pub const ID_REFRESH: usize = 1008;
pub const ID_SETTINGS: usize = 1009;
pub const ID_BG_EFFECT: usize = 1010;

pub unsafe fn add_tray_icon(hwnd: HWND) -> bool {
    let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = 1;
    nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    nid.uCallbackMessage = WM_TRAY_CALLBACK;
    
    // Load standard application icon
    nid.hIcon = LoadIconW(0, IDI_APPLICATION);
    
    let tip = to_wide("CPU Star Wallpaper");
    let len = tip.len().min(127);
    nid.szTip[..len].copy_from_slice(&tip[..len]);

    Shell_NotifyIconW(NIM_ADD, &nid) != 0
}

pub unsafe fn remove_tray_icon(hwnd: HWND) -> bool {
    let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = 1;

    Shell_NotifyIconW(NIM_DELETE, &nid) != 0
}

pub unsafe fn show_context_menu(
    hwnd: HWND, 
    bg_effect_enabled: bool,
) {
    let mut point = POINT { x: 0, y: 0 };
    GetCursorPos(&mut point);

    let menu = CreatePopupMenu();
    if menu == 0 {
        return;
    }

    // 1. Wallpaper Load Effect Toggle
    let bg_effect_flags = if bg_effect_enabled { MF_STRING | MF_CHECKED } else { MF_STRING };
    AppendMenuW(menu, bg_effect_flags, ID_BG_EFFECT, to_wide("Wallpaper Load Effect").as_ptr());

    AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
    
    // 2. Refresh & Settings options
    AppendMenuW(menu, MF_STRING, ID_REFRESH, to_wide("Refresh Wallpaper").as_ptr());
    AppendMenuW(menu, MF_STRING, ID_SETTINGS, to_wide("Color Presets...").as_ptr());
    
    AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());

    // 4. Exit option
    AppendMenuW(menu, MF_STRING, ID_EXIT, to_wide("Exit").as_ptr());

    // Necessary Win32 ceremony for popup menus to ensure they close on clicking outside
    SetForegroundWindow(hwnd);
    
    TrackPopupMenu(
        menu,
        TPM_RIGHTBUTTON | TPM_LEFTALIGN,
        point.x,
        point.y,
        0,
        hwnd,
        std::ptr::null(),
    );

    PostMessageW(hwnd, WM_NULL, 0, 0);
    DestroyMenu(menu);
}
