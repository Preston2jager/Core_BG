use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use windows_sys::Win32::Graphics::Gdi::HBRUSH;
use crate::window::to_wide;
use std::sync::Mutex;

const SS_LEFT: u32 = 0;
const COLOR_WINDOW: u32 = 5;

// Control IDs
const ID_CHECKBOX_BG_EFFECT: i32 = 3005;
const ID_COMBO_COLOR: i32 = 3006;
const ID_COMBO_GLOW: i32 = 3007;
const ID_COMBO_FPS: i32 = 3008;

const BS_GROUPBOX: u32 = 0x0007;
const BS_AUTOCHECKBOX: u32 = 0x0003;
const BM_GETCHECK: u32 = 0x00F0;
const BM_SETCHECK: u32 = 0x00F1;
const BST_CHECKED: u32 = 0x0001;
const BST_UNCHECKED: u32 = 0x0000;

const CBS_DROPDOWNLIST: u32 = 0x0003;
const CB_ADDSTRING: u32 = 0x0143;
const CB_SETCURSEL: u32 = 0x014E;
const CB_GETCURSEL: u32 = 0x0147;
const CBN_SELCHANGE: u32 = 1;
const WS_VSCROLL: u32 = 0x00200000;

pub static SETTINGS_HWND: Mutex<HWND> = Mutex::new(0);

unsafe extern "system" fn settings_wnd_proc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match message {
        WM_CREATE => {
            let pcs = lparam as *const CREATESTRUCTW;
            let hinst = (*pcs).hInstance;

            let (color_preset, glow, fps, bg_effect_enabled) = {
                let s = crate::app::STATE.lock().unwrap();
                (s.color_preset, s.glow, s.fps, s.bg_effect_enabled)
            };

            let static_class = to_wide("STATIC");
            let button_class = to_wide("BUTTON");
            let combobox_class = to_wide("COMBOBOX");

            let create_control = |class: &[u16], title: &str, style: u32, x: i32, y: i32, w: i32, h: i32, id: i32| {
                CreateWindowExW(
                    0,
                    class.as_ptr(),
                    to_wide(title).as_ptr(),
                    WS_CHILD | WS_VISIBLE | style,
                    x, y, w, h,
                    hwnd,
                    id as HMENU,
                    hinst,
                    std::ptr::null(),
                )
            };

            // 1. Visual Settings GroupBox
            create_control(&button_class, "Visual Settings", BS_GROUPBOX, 20, 20, 360, 175, 0);

            // 1a. Color Preset Combo
            create_control(&static_class, "Color Preset:", SS_LEFT, 40, 45, 100, 20, 0);
            let c_preset = create_control(&combobox_class, "", CBS_DROPDOWNLIST | WS_VSCROLL, 160, 41, 200, 200, ID_COMBO_COLOR);
            SendMessageW(c_preset, CB_ADDSTRING, 0, to_wide("Atomic Starlink").as_ptr() as LPARAM);
            SendMessageW(c_preset, CB_ADDSTRING, 0, to_wide("Cyberpunk").as_ptr() as LPARAM);
            SendMessageW(c_preset, CB_ADDSTRING, 0, to_wide("Acid Green").as_ptr() as LPARAM);
            SendMessageW(c_preset, CB_ADDSTRING, 0, to_wide("Solar Flame").as_ptr() as LPARAM);
            SendMessageW(c_preset, CB_ADDSTRING, 0, to_wide("Deep Ocean").as_ptr() as LPARAM);
            SendMessageW(c_preset, CB_ADDSTRING, 0, to_wide("Emerald Pulse").as_ptr() as LPARAM);
            SendMessageW(c_preset, CB_ADDSTRING, 0, to_wide("Crimson Nova").as_ptr() as LPARAM);
            SendMessageW(c_preset, CB_ADDSTRING, 0, to_wide("Violet Night").as_ptr() as LPARAM);
            SendMessageW(c_preset, CB_ADDSTRING, 0, to_wide("Amber Ghost").as_ptr() as LPARAM);
            SendMessageW(c_preset, CB_ADDSTRING, 0, to_wide("Frost Byte").as_ptr() as LPARAM);
            
            let color_idx = match color_preset {
                crate::app::ColorPreset::AtomicStarlink => 0,
                crate::app::ColorPreset::Cyberpunk => 1,
                crate::app::ColorPreset::AcidGreen => 2,
                crate::app::ColorPreset::SolarFlame => 3,
                crate::app::ColorPreset::DeepOcean => 4,
                crate::app::ColorPreset::EmeraldPulse => 5,
                crate::app::ColorPreset::CrimsonNova => 6,
                crate::app::ColorPreset::VioletNight => 7,
                crate::app::ColorPreset::AmberGhost => 8,
                crate::app::ColorPreset::FrostByte => 9,
            };
            SendMessageW(c_preset, CB_SETCURSEL, color_idx, 0);

            // 1b. Glow Intensity Combo
            create_control(&static_class, "Glow Intensity:", SS_LEFT, 40, 85, 100, 20, 0);
            let c_glow = create_control(&combobox_class, "", CBS_DROPDOWNLIST | WS_VSCROLL, 160, 81, 200, 200, ID_COMBO_GLOW);
            SendMessageW(c_glow, CB_ADDSTRING, 0, to_wide("Low").as_ptr() as LPARAM);
            SendMessageW(c_glow, CB_ADDSTRING, 0, to_wide("Medium").as_ptr() as LPARAM);
            SendMessageW(c_glow, CB_ADDSTRING, 0, to_wide("High").as_ptr() as LPARAM);
            SendMessageW(c_glow, CB_SETCURSEL, glow as WPARAM, 0);

            // 1c. FPS Limit Combo
            create_control(&static_class, "FPS Limit:", SS_LEFT, 40, 125, 100, 20, 0);
            let c_fps = create_control(&combobox_class, "", CBS_DROPDOWNLIST | WS_VSCROLL, 160, 121, 200, 200, ID_COMBO_FPS);
            SendMessageW(c_fps, CB_ADDSTRING, 0, to_wide("30 FPS").as_ptr() as LPARAM);
            SendMessageW(c_fps, CB_ADDSTRING, 0, to_wide("60 FPS").as_ptr() as LPARAM);
            SendMessageW(c_fps, CB_SETCURSEL, if fps == 30 { 0 } else { 1 }, 0);

            // 1d. Wallpaper Load Effect Checkbox
            let chk = create_control(&button_class, "Enable Wallpaper Load Effect (>60% CPU)", BS_AUTOCHECKBOX, 40, 165, 320, 25, ID_CHECKBOX_BG_EFFECT);
            SendMessageW(chk, BM_SETCHECK, if bg_effect_enabled { BST_CHECKED } else { BST_UNCHECKED } as WPARAM, 0);

            0
        }
        WM_COMMAND => {
            let id = (wparam & 0xFFFF) as i32;
            let code = ((wparam >> 16) & 0xFFFF) as u32;
            
            if lparam != 0 {
                let ctrl_hwnd = lparam as HWND;
                if id == ID_CHECKBOX_BG_EFFECT && code == 0 { // BN_CLICKED is 0
                    let is_checked = SendMessageW(ctrl_hwnd, BM_GETCHECK, 0, 0) == BST_CHECKED as LRESULT;
                    let mut state = crate::app::STATE.lock().unwrap();
                    state.bg_effect_enabled = is_checked;
                    crate::app::save_settings(&state);
                } else if code == CBN_SELCHANGE {
                    let sel_idx = SendMessageW(ctrl_hwnd, CB_GETCURSEL, 0, 0) as i32;
                    if sel_idx >= 0 {
                        let mut state = crate::app::STATE.lock().unwrap();
                        match id {
                            ID_COMBO_COLOR => {
                                state.color_preset = match sel_idx {
                                    0 => crate::app::ColorPreset::AtomicStarlink,
                                    1 => crate::app::ColorPreset::Cyberpunk,
                                    2 => crate::app::ColorPreset::AcidGreen,
                                    3 => crate::app::ColorPreset::SolarFlame,
                                    4 => crate::app::ColorPreset::DeepOcean,
                                    5 => crate::app::ColorPreset::EmeraldPulse,
                                    6 => crate::app::ColorPreset::CrimsonNova,
                                    7 => crate::app::ColorPreset::VioletNight,
                                    8 => crate::app::ColorPreset::AmberGhost,
                                    9 => crate::app::ColorPreset::FrostByte,
                                    _ => crate::app::ColorPreset::AtomicStarlink,
                                };
                            }
                            ID_COMBO_GLOW => {
                                state.glow = sel_idx as u8;
                            }
                            ID_COMBO_FPS => {
                                state.fps = if sel_idx == 0 { 30 } else { 60 };
                            }
                            _ => {}
                        }
                        crate::app::save_settings(&state);
                    }
                }
            }
            0
        }
        WM_CLOSE => {
            DestroyWindow(hwnd);
            0
        }
        WM_DESTROY => {
            *SETTINGS_HWND.lock().unwrap() = 0;
            0
        }
        _ => DefWindowProcW(hwnd, message, wparam, lparam),
    }
}

pub unsafe fn show_settings_window(hinstance: HINSTANCE) {
    let mut open_hwnd = SETTINGS_HWND.lock().unwrap();
    if *open_hwnd != 0 {
        ShowWindow(*open_hwnd, SW_RESTORE);
        SetForegroundWindow(*open_hwnd);
        return;
    }

    let class_name = to_wide("BGCoreV2SettingsClass");

    let wc = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(settings_wnd_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: hinstance,
        hIcon: LoadIconW(0, IDI_APPLICATION),
        hCursor: LoadCursorW(0, IDC_ARROW),
        hbrBackground: (COLOR_WINDOW + 1) as HBRUSH,
        lpszMenuName: std::ptr::null(),
        lpszClassName: class_name.as_ptr(),
    };

    RegisterClassW(&wc);

    let mut rect = RECT { left: 0, top: 0, right: 400, bottom: 220 };
    let style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX;
    AdjustWindowRectEx(&mut rect, style, 0, 0);

    let hwnd = CreateWindowExW(
        0,
        class_name.as_ptr(),
        to_wide("Wallpaper Color & Effect Settings").as_ptr(),
        style,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        rect.right - rect.left,
        rect.bottom - rect.top,
        0,
        0,
        hinstance,
        std::ptr::null(),
    );

    if hwnd != 0 {
        ShowWindow(hwnd, SW_SHOW);
        *open_hwnd = hwnd;
    }
}
