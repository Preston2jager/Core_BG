use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use windows_sys::Win32::Graphics::Gdi::HBRUSH;
use crate::window::to_wide;
use std::sync::Mutex;

const SS_LEFT: u32 = 0;
const COLOR_WINDOW: u32 = 5;

// Control IDs
const ID_LABEL_CORE_SIZE: i32 = 4001;
const ID_SLIDER_CORE_SIZE: i32 = 3001;

const ID_LABEL_ORBIT_R: i32 = 4002;
const ID_SLIDER_ORBIT_R: i32 = 3002;

const ID_LABEL_SAT_SIZE: i32 = 4003;
const ID_SLIDER_SAT_SIZE: i32 = 3003;

const ID_CHECKBOX_BG_EFFECT: i32 = 3005;
const ID_COMBO_COLOR: i32 = 3006;
const ID_COMBO_GLOW: i32 = 3007;
const ID_COMBO_FPS: i32 = 3008;

// Win32 Trackbar Messages
const TBS_HORZ: u32 = 0x0000;
const TBM_GETPOS: u32 = 1024;
const TBM_SETPOS: u32 = 1026;
const TBM_SETRANGEMIN: u32 = 1031;
const TBM_SETRANGEMAX: u32 = 1032;

// Win32 Button / ComboBox Messages & Styles
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

unsafe fn update_label_text(hwnd: HWND, label_id: i32, text: &str) {
    let hwnd_label = GetDlgItem(hwnd, label_id);
    if hwnd_label != 0 {
        SetWindowTextW(hwnd_label, to_wide(text).as_ptr());
    }
}

unsafe fn on_slider_scroll(hwnd: HWND, slider_id: i32, pos: i32) {
    let mut state = crate::app::STATE.lock().unwrap();
    match slider_id {
        ID_SLIDER_CORE_SIZE => {
            let val = pos as f32 / 100.0;
            state.core_size = val;
            update_label_text(hwnd, ID_LABEL_CORE_SIZE, &format!("Logo/Orbit Size Ratio: {:.2}x", val));
        }
        ID_SLIDER_ORBIT_R => {
            let val = pos as f32 / 100.0;
            state.core_orbit_r = val;
            update_label_text(hwnd, ID_LABEL_ORBIT_R, &format!("Orbit Radius: {:.2}x", val));
        }
        ID_SLIDER_SAT_SIZE => {
            let val = pos as f32 / 100.0;
            state.satellite_size = val;
            update_label_text(hwnd, ID_LABEL_SAT_SIZE, &format!("Satellite Size: {:.2}x", val));
        }
        _ => return,
    }
    crate::app::save_settings(&state);
}

unsafe extern "system" fn settings_wnd_proc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match message {
        WM_CREATE => {
            let pcs = lparam as *const CREATESTRUCTW;
            let hinst = (*pcs).hInstance;

            let (core_sz, orbit_r, sat_sz, color_preset, glow, fps, bg_effect_enabled) = {
                let s = crate::app::STATE.lock().unwrap();
                (s.core_size, s.core_orbit_r, s.satellite_size, s.color_preset, s.glow, s.fps, s.bg_effect_enabled)
            };

            let static_class = to_wide("STATIC");
            let trackbar_class = to_wide("msctls_trackbar32");
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

            // 1. Core Size Slider
            create_control(&static_class, &format!("Logo/Orbit Size Ratio: {:.2}x", core_sz), SS_LEFT, 20, 20, 360, 20, ID_LABEL_CORE_SIZE);
            let s1 = create_control(&trackbar_class, "", TBS_HORZ, 20, 40, 360, 30, ID_SLIDER_CORE_SIZE);
            SendMessageW(s1, TBM_SETRANGEMIN, 1, 10 as LPARAM);
            SendMessageW(s1, TBM_SETRANGEMAX, 1, 1000 as LPARAM);
            SendMessageW(s1, TBM_SETPOS, 1, (core_sz * 100.0) as i32 as LPARAM);

            // 2. Orbit Radius Slider
            create_control(&static_class, &format!("Orbit Radius: {:.2}x", orbit_r), SS_LEFT, 20, 90, 360, 20, ID_LABEL_ORBIT_R);
            let s2 = create_control(&trackbar_class, "", TBS_HORZ, 20, 110, 360, 30, ID_SLIDER_ORBIT_R);
            SendMessageW(s2, TBM_SETRANGEMIN, 1, 10 as LPARAM);
            SendMessageW(s2, TBM_SETRANGEMAX, 1, 1000 as LPARAM);
            SendMessageW(s2, TBM_SETPOS, 1, (orbit_r * 100.0) as i32 as LPARAM);

            // 3. Satellite Size Slider
            create_control(&static_class, &format!("Satellite Size: {:.2}x", sat_sz), SS_LEFT, 20, 160, 360, 20, ID_LABEL_SAT_SIZE);
            let s3 = create_control(&trackbar_class, "", TBS_HORZ, 20, 180, 360, 30, ID_SLIDER_SAT_SIZE);
            SendMessageW(s3, TBM_SETRANGEMIN, 1, 10 as LPARAM);
            SendMessageW(s3, TBM_SETRANGEMAX, 1, 1000 as LPARAM);
            SendMessageW(s3, TBM_SETPOS, 1, (sat_sz * 100.0) as i32 as LPARAM);

            // 4. Visual Settings GroupBox (height 175)
            create_control(&button_class, "Visual Settings", BS_GROUPBOX, 20, 230, 360, 175, 0);

            // 4a. Color Preset Combo
            create_control(&static_class, "Color Preset:", SS_LEFT, 40, 255, 100, 20, 0);
            let c_preset = create_control(&combobox_class, "", CBS_DROPDOWNLIST | WS_VSCROLL, 160, 251, 200, 200, ID_COMBO_COLOR);
            SendMessageW(c_preset, CB_ADDSTRING, 0, to_wide("Atomic Starlink").as_ptr() as LPARAM);
            SendMessageW(c_preset, CB_ADDSTRING, 0, to_wide("Cyberpunk").as_ptr() as LPARAM);
            SendMessageW(c_preset, CB_ADDSTRING, 0, to_wide("Acid Green").as_ptr() as LPARAM);
            SendMessageW(c_preset, CB_ADDSTRING, 0, to_wide("Solar Flame").as_ptr() as LPARAM);
            SendMessageW(c_preset, CB_ADDSTRING, 0, to_wide("Deep Ocean").as_ptr() as LPARAM);
            
            let color_idx = match color_preset {
                crate::app::ColorPreset::AtomicStarlink => 0,
                crate::app::ColorPreset::Cyberpunk => 1,
                crate::app::ColorPreset::AcidGreen => 2,
                crate::app::ColorPreset::SolarFlame => 3,
                crate::app::ColorPreset::DeepOcean => 4,
            };
            SendMessageW(c_preset, CB_SETCURSEL, color_idx, 0);

            // 4b. Glow Intensity Combo
            create_control(&static_class, "Glow Intensity:", SS_LEFT, 40, 295, 100, 20, 0);
            let c_glow = create_control(&combobox_class, "", CBS_DROPDOWNLIST | WS_VSCROLL, 160, 291, 200, 200, ID_COMBO_GLOW);
            SendMessageW(c_glow, CB_ADDSTRING, 0, to_wide("Low").as_ptr() as LPARAM);
            SendMessageW(c_glow, CB_ADDSTRING, 0, to_wide("Medium").as_ptr() as LPARAM);
            SendMessageW(c_glow, CB_ADDSTRING, 0, to_wide("High").as_ptr() as LPARAM);
            SendMessageW(c_glow, CB_SETCURSEL, glow as WPARAM, 0);

            // 4c. FPS Limit Combo
            create_control(&static_class, "FPS Limit:", SS_LEFT, 40, 335, 100, 20, 0);
            let c_fps = create_control(&combobox_class, "", CBS_DROPDOWNLIST | WS_VSCROLL, 160, 331, 200, 200, ID_COMBO_FPS);
            SendMessageW(c_fps, CB_ADDSTRING, 0, to_wide("30 FPS").as_ptr() as LPARAM);
            SendMessageW(c_fps, CB_ADDSTRING, 0, to_wide("60 FPS").as_ptr() as LPARAM);
            SendMessageW(c_fps, CB_SETCURSEL, if fps == 30 { 0 } else { 1 }, 0);

            // 4d. Wallpaper Load Effect Checkbox
            let chk = create_control(&button_class, "Enable Wallpaper Load Effect (>60% CPU)", BS_AUTOCHECKBOX, 40, 375, 320, 25, ID_CHECKBOX_BG_EFFECT);
            SendMessageW(chk, BM_SETCHECK, if bg_effect_enabled { BST_CHECKED } else { BST_UNCHECKED } as WPARAM, 0);


            0
        }
        WM_HSCROLL => {
            let trackbar_hwnd = lparam as HWND;
            if trackbar_hwnd != 0 {
                let pos = SendMessageW(trackbar_hwnd, TBM_GETPOS, 0, 0) as i32;
                let id = GetDlgCtrlID(trackbar_hwnd);
                on_slider_scroll(hwnd, id, pos);
            }
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

    // Calculate window size to match client area of 400x420
    let mut rect = RECT { left: 0, top: 0, right: 400, bottom: 420 };
    let style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX;
    AdjustWindowRectEx(&mut rect, style, 0, 0);

    let hwnd = CreateWindowExW(
        0,
        class_name.as_ptr(),
        to_wide("Wallpaper Effects Settings").as_ptr(),
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
