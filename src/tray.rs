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

use crate::tray::Event::{Exit, Nothing};

const SYSICON_ID: u32 = 0x10;
const SYSTEM_TRAY_POPUP_EXIT: u32 = 0x111;
const SYSTEM_TRAY_MESSAGE: u32 = 0x11;

pub enum Event {
    Exit,
    Nothing
}

pub fn create_message_window() -> Result<HWND, Error> {
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
        ShowWindow(handle, SW_SHOW);
        Ok(handle)
    }
}

pub fn destroy_message_window(window_handle: HWND) -> Result<(), Error> {
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
    let icon_path: U16CString = U16CString::from_str(".\\icon.ico").unwrap();
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

pub fn handle_windows_messages(window_handle: HWND) -> Result<Event, Error> {
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
        let response_code = PeekMessageW(&mut msg, window_handle, 0, 0, PM_REMOVE);
        if response_code == 0 {
            return Ok(Nothing);
        }
        println!("{:?}", msg.message);

        if msg.message == SYSTEM_TRAY_POPUP_EXIT {
            println!("exit");
            return Ok(Exit);
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
        let event = lparam & 0xff;
        if event == 0x04 {
            println!("right clicked");
            create_tray_menu(hwnd);
            return 0;
        }
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

fn create_tray_menu(window_handle: HWND) {
    unsafe {
        let mut position = POINT { x: 0, y: 0 };
        GetCursorPos(&mut position);
        let popup = CreatePopupMenu();
        InsertMenuA(popup, 0xFFFFFFFF, MF_BYPOSITION | MF_STRING, SYSTEM_TRAY_POPUP_EXIT as usize, CString::new("Exit").unwrap().as_ptr());
        TrackPopupMenu(popup, TPM_LEFTALIGN | TPM_LEFTBUTTON | TPM_BOTTOMALIGN, position.x, position.y, 0, window_handle, null_mut());
    }
}
