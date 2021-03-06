use std::ffi::CString;
use std::io::Error;
use std::ptr::null_mut;

use widestring::U16CString;
use winapi::shared::minwindef::*;
use winapi::shared::windef::*;
use winapi::um::shellapi::*;
//use winapi::um::d2d1::*;
use winapi::um::wingdi::*;
use winapi::um::winuser::*;

use crate::tray::Event::{Exit, Nothing, ToggleAutoStart};
use crate::utils::{LoggableResult, join_executable_path};

const SYSICON_ID: u32 = 0x10;
const SYSTEM_TRAY_POPUP_EXIT: usize = 0x111;
const SYSTEM_TRAY_POPUP_STARTUP: usize = 0x112;
const SYSTEM_TRAY_POPUP_VERSION: usize = 0x113;
const SYSTEM_TRAY_MESSAGE: u32 = 0x11;
const MESSAGE_SHOW_TRAY_POPUP: u32 = WM_APP + 1;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub enum Event {
    Exit,
    ToggleAutoStart,
    Nothing
}

pub struct SystemTray {
    window_handle: HWND,
    menu_handle: HMENU,
    run_on_startup: bool
}

impl SystemTray {
    pub fn new(run_on_startup: bool) -> Result<SystemTray, Error> {
        let window_handle = create_message_window()?;
        let menu_handle = create_tray_menu(run_on_startup);
        Ok(SystemTray {
            window_handle,
            menu_handle,
            run_on_startup
        })
    }

    pub fn handle_windows_messages(&self) -> Result<Event, Error> {
        handle_windows_messages(self.window_handle, self.menu_handle)
    }

    pub fn set_run_on_startup(&mut self, new_value: bool) {
        self.run_on_startup = new_value;
        let mask = bool_as_menu_flag(new_value);
        update_tray_menu_item_state(self.window_handle, self.menu_handle, SYSTEM_TRAY_POPUP_STARTUP, mask);
    }
}

impl Drop for SystemTray {
    fn drop(&mut self) {
        destroy_message_window(self.window_handle)
            .log_error_and_ignore("failed to clean up SystemTray, exiting regardless.");
    }
}

fn create_message_window() -> Result<HWND, Error> {
    unsafe {
        let class_name = U16CString::from_str("poe-minimizer").unwrap();
        let icon = LoadIconW(0 as HINSTANCE, IDI_APPLICATION);
        let cursor = LoadCursorW(0 as HINSTANCE, IDC_ARROW);
        let brush = CreateSolidBrush(0xffffffff);
        let window_class = WNDCLASSW {
            style: 0,
            lpfnWndProc: Some(win_proc_dispatch),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: 0 as HINSTANCE,
            hIcon: icon,
            hCursor: cursor,
            hbrBackground: brush,
            lpszMenuName: null_mut(),
            lpszClassName: class_name.as_ptr(),
        };
        let class_atom = RegisterClassW(&window_class);
        if class_atom == 0 {
            return Err(Error::last_os_error());
        }

        let window_name = CString::new("poe-minimizer").unwrap();

        let style = WS_EX_NOACTIVATE | WS_EX_NOINHERITLAYOUT;
        let handle = CreateWindowExA(style,
                                     CString::new("poe-minimizer").unwrap().as_ptr(),
                                     window_name.as_ptr(),
                                     0, 0, 0, 0, 0,
                                     HWND_MESSAGE, null_mut(), 0 as HINSTANCE,
                                     null_mut(),
        );
        if handle == null_mut() {
            return Err(Error::last_os_error());
        }
        create_system_tray(handle).expect("yo");
        Ok(handle)
    }
}

fn destroy_message_window(window_handle: HWND) -> Result<(), Error> {
    remove_system_tray(window_handle).expect("failed to remove application from system tray");
    Ok(())
}

fn create_system_tray(window_handle: HWND) -> Result<(), Error> {
    let mut icon_data = init_notify_icon_data();
    icon_data.hWnd = window_handle;
    let title = "poe-minimizer".as_bytes();
    for i in 0..title.len() {
        icon_data.szTip[i] = title[i] as i8;
    }
    let icon_path = join_executable_path("icon.ico").unwrap_or(".\\icon.ico".to_owned());
    let icon_path: U16CString = U16CString::from_str(icon_path).unwrap();
    let icon = unsafe {
        let icon = LoadImageW(null_mut(), icon_path.as_ptr(), IMAGE_ICON, 0, 0, LR_LOADFROMFILE) as HICON;
        if icon == null_mut() {
            Err(Error::last_os_error())
        } else {
            Ok(icon)
        }
    }?;
    icon_data.hIcon = icon;
    unsafe {
        if 0 == Shell_NotifyIconA(NIM_ADD, &mut icon_data) {
            Err(Error::last_os_error())
        } else {
            Ok(())
        }
    }
}

fn remove_system_tray(window_handle: HWND) -> Result<(), Error> {
    let mut icon_data = init_notify_icon_data();
    icon_data.hWnd = window_handle;
    unsafe {
        if 0 == Shell_NotifyIconA(NIM_DELETE, &mut icon_data) {
            Err(Error::last_os_error())
        } else {
            Ok(())
        }
    }
}

fn init_notify_icon_data() -> NOTIFYICONDATAA {
    let wtf_is_this: NOTIFYICONDATAA_u = unsafe {
        std::mem::transmute::<u32, NOTIFYICONDATAA_u>(0 as u32)
    };
    NOTIFYICONDATAA {
        cbSize: 0,
        hWnd: null_mut(),
        uID: SYSICON_ID,
        uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
        uCallbackMessage: SYSTEM_TRAY_MESSAGE,
        hIcon: null_mut(),
        szTip: [0; 128],
        dwState: NIS_SHAREDICON,
        dwStateMask: 0,
        szInfo: [0; 256],
        u: wtf_is_this, // FIXME: If someone could explain to me how this could be done without this absurdity, I'd be happy.
        szInfoTitle: [0; 64],
        dwInfoFlags: 0,
        guidItem: winapi::shared::guiddef::GUID {
            Data1: 0,
            Data2: 0,
            Data3: 0,
            Data4: [0; 8],
        },
        hBalloonIcon: null_mut(),
    }
}

fn handle_windows_messages(window_handle: HWND, menu_handle: HMENU) -> Result<Event, Error> {
    unsafe {
        let mut msg = MSG {
            hwnd: null_mut(),
            message: 0,
            wParam: 0,
            lParam: 0,
            time: 0,
            pt: POINT {
                x: 0,
                y: 0,
            },
        };
        let response_code = GetMessageW(&mut msg, window_handle, 0, 0);
        if response_code == 0 {
            return Ok(Nothing);
        }
        trace!("[WM] received windows message: {}", msg.message);

        if msg.message == WM_COMMAND {
            trace!("[WM] received command: lp: {}, wp: {}", msg.lParam, msg.wParam);
            return match msg.wParam {
                SYSTEM_TRAY_POPUP_EXIT => Ok(Exit),
                SYSTEM_TRAY_POPUP_STARTUP => {
                    update_tray_menu_item_state(window_handle, menu_handle, SYSTEM_TRAY_POPUP_STARTUP, MFS_UNCHECKED);
                    Ok(ToggleAutoStart)
                },
                _ => Ok(Nothing)
            };
        }

        if msg.message == MESSAGE_SHOW_TRAY_POPUP {
            trace!("[WM] received show tray popup");
            show_tray_menu(window_handle, menu_handle);
        }

        TranslateMessage(&mut msg);
        DispatchMessageW(&mut msg);
    }
    Ok(Nothing)
}


unsafe extern "system" fn win_proc_dispatch(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM)
                                            -> LRESULT
{
    if msg == SYSTEM_TRAY_MESSAGE {
        trace!("received tray message: {}, w: {}, l: {}", msg, wparam, lparam);
        let event = lparam & 0xff;
        if event == 0x04 {
            PostMessageA(hwnd, MESSAGE_SHOW_TRAY_POPUP, 0, 0);
            //create_tray_menu(hwnd);
            //return 0;
        }
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

fn create_tray_menu(run_on_startup: bool) -> HMENU {
    unsafe {
        let popup = CreatePopupMenu();
        trace!("creating popup: {:?}", popup);
        let version = format!("poe-minimizer v{}", VERSION);
        InsertMenuA(popup, 0xFFFFFFFE, MF_BYPOSITION | MF_STRING | MF_DISABLED, SYSTEM_TRAY_POPUP_VERSION, CString::new(version).unwrap().as_ptr());
        InsertMenuA(popup, 0xFFFFFFFE, MF_SEPARATOR, 0, CString::new("test").unwrap().as_ptr());
        InsertMenuA(popup, 0xFFFFFFFE, MF_BYPOSITION | MF_STRING | bool_as_menu_flag(run_on_startup), SYSTEM_TRAY_POPUP_STARTUP, CString::new("Run on startup").unwrap().as_ptr());
        InsertMenuA(popup, 0xFFFFFFFE, MF_SEPARATOR, 0, CString::new("test").unwrap().as_ptr());
        InsertMenuA(popup, 0xFFFFFFFE, MF_BYPOSITION | MF_STRING, SYSTEM_TRAY_POPUP_EXIT, CString::new("Exit").unwrap().as_ptr());
        popup
    }
}

fn show_tray_menu(window_handle: HWND, menu_handle: HMENU) {
    unsafe {
        let mut position = POINT { x: 0, y: 0 };
        GetCursorPos(&mut position);
        SetForegroundWindow(window_handle);
        TrackPopupMenu(menu_handle, TPM_LEFTALIGN | TPM_LEFTBUTTON | TPM_BOTTOMALIGN, position.x, position.y, 0, window_handle, null_mut());
        PostMessageA(window_handle, WM_NULL, 0, 0);
    }
}

fn update_tray_menu_item_state(window_handle: HWND, menu_handle: HMENU, item_id: usize, flags: u32) {
    unsafe {
        let mut item_info = MENUITEMINFOA {
            cbSize: std::mem::size_of::<MENUITEMINFOA>() as u32,
            fMask: MIIM_STATE,
            fType: 0,
            fState: flags,
            wID: 0,
            hSubMenu: null_mut(),
            hbmpChecked: null_mut(),
            hbmpUnchecked: null_mut(),
            dwItemData: 0,
            dwTypeData: null_mut(),
            cch: 0,
            hbmpItem: null_mut()
        };

        if 0 == SetMenuItemInfoA(menu_handle, item_id as u32, 0, &mut item_info) {
            Err::<(), Error>(Error::last_os_error()).log_error_and_ignore("failed to update menu item");
        }
        DrawMenuBar(window_handle);
    }
}

fn bool_as_menu_flag(value: bool) -> u32 {
    if value {
        MFS_CHECKED
    } else {
        MFS_UNCHECKED
    }
}
