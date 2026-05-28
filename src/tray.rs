use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use windows_sys::Win32::UI::Shell::*;
use super::window::to_wide;

pub const WM_TRAY_CALLBACK: u32 = WM_USER + 1;

// Menu Item IDs
pub const ID_EXIT: usize = 1001;
pub const ID_REFRESH: usize = 1008;
pub const ID_BG_EFFECT: usize = 1010;
pub const ID_SET_OFFSET: usize = 1012;
pub const ID_OPEN_FOLDER: usize = 1014;
pub const ID_SOURCE_SSH: usize = 1016;
pub const ID_SOURCE_CPU: usize = 1018;

// Color Preset IDs (1100 - 1109)
pub const ID_COLOR_BASE: usize = 1100;

pub unsafe fn add_tray_icon(hwnd: HWND) -> bool {
    let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = 1;
    nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    nid.uCallbackMessage = WM_TRAY_CALLBACK;
    
    // Load standard application icon
    nid.hIcon = LoadIconW(0, IDI_APPLICATION);
    
    let tip = to_wide("StarCore v0.1");
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
    current_preset: crate::app::ColorPreset,
    ssh_connected: bool,
    monitor_source: crate::app::MonitorSource,
) {
    let mut point = POINT { x: 0, y: 0 };
    GetCursorPos(&mut point);

    let menu = CreatePopupMenu();
    if menu == 0 {
        return;
    }

    // 0. Connection Status & Source selection
    let status_text = if ssh_connected { "SSH Server: Connected" } else { "SSH Server: Disconnected" };
    AppendMenuW(menu, MF_STRING | MF_GRAYED, 0, to_wide(status_text).as_ptr());
    
    let ssh_flags = if monitor_source == crate::app::MonitorSource::RemoteGpuSsh { MF_STRING | MF_CHECKED } else { MF_STRING };
    AppendMenuW(menu, ssh_flags, ID_SOURCE_SSH, to_wide("Monitor: Remote GPU (SSH)").as_ptr());
    
    let cpu_flags = if monitor_source == crate::app::MonitorSource::LocalCpu { MF_STRING | MF_CHECKED } else { MF_STRING };
    AppendMenuW(menu, cpu_flags, ID_SOURCE_CPU, to_wide("Monitor: Local CPU").as_ptr());

    AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());

    // 1. Color Presets (Primary Menu)
    let presets = [
        ("Atomic Starlink", crate::app::ColorPreset::AtomicStarlink),
        ("Cyberpunk", crate::app::ColorPreset::Cyberpunk),
        ("Acid Green", crate::app::ColorPreset::AcidGreen),
        ("Solar Flame", crate::app::ColorPreset::SolarFlame),
        ("Deep Ocean", crate::app::ColorPreset::DeepOcean),
        ("Emerald Pulse", crate::app::ColorPreset::EmeraldPulse),
        ("Crimson Nova", crate::app::ColorPreset::CrimsonNova),
        ("Violet Night", crate::app::ColorPreset::VioletNight),
        ("Amber Ghost", crate::app::ColorPreset::AmberGhost),
        ("Frost Byte", crate::app::ColorPreset::FrostByte),
    ];

    for (i, (name, preset)) in presets.iter().enumerate() {
        let flags = if *preset == current_preset { MF_STRING | MF_CHECKED } else { MF_STRING };
        AppendMenuW(menu, flags, ID_COLOR_BASE + i, to_wide(name).as_ptr());
    }
    
    AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());

    // 2. Toggles & Actions
    let bg_effect_flags = if bg_effect_enabled { MF_STRING | MF_CHECKED } else { MF_STRING };
    AppendMenuW(menu, bg_effect_flags, ID_BG_EFFECT, to_wide("Wallpaper Load Effect").as_ptr());
    AppendMenuW(menu, MF_STRING, ID_SET_OFFSET, to_wide("Set Position Offset").as_ptr());
    AppendMenuW(menu, MF_STRING, ID_OPEN_FOLDER, to_wide("Open Settings Folder").as_ptr());
    AppendMenuW(menu, MF_STRING, ID_REFRESH, to_wide("Refresh Wallpaper").as_ptr());

    AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());

    // 3. Exit
    AppendMenuW(menu, MF_STRING, ID_EXIT, to_wide("Exit").as_ptr());

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
