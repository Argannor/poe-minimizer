use std::io::Error;
use std::ptr::null_mut;
use std::path::PathBuf;
use winapi::um::winreg::{HKEY_CURRENT_USER, RegQueryValueExA};
use winapi::shared::winerror::{ERROR_SUCCESS, ERROR_FILE_NOT_FOUND};
use std::ffi::CString;

pub fn get_window_handle(title: &str) -> Result<winapi::shared::windef::HWND, Error> {
    let window_handle = unsafe {
        let window_title = CString::new(title).expect("CString::new failed");
        winapi::um::winuser::FindWindowA(null_mut(), window_title.as_ptr())
    };
    if window_handle == null_mut() {
        Err(Error::last_os_error())
    } else {
        Ok(window_handle)
    }
}

pub fn get_process_path_by_window_handle(window_handle: winapi::shared::windef::HWND) -> Result<PathBuf, Error> {
    get_process_id(window_handle)
        .and_then(|process_id|  get_process_handle(process_id).map(|handle| (process_id, handle)))
        .and_then(|(process_id, handle)| get_process_module(handle).map(|module_handle| (process_id, handle, module_handle)))
        .and_then(|(_, process_handle, module_handle)| get_process_file_name(process_handle, module_handle))
}

pub fn minimize_window(window_handle: winapi::shared::windef::HWND) -> Result<(), Error> {
    let result = unsafe {
        winapi::um::winuser::ShowWindow(window_handle, winapi::um::winuser::SW_MINIMIZE)
    };
    if result != 0 {
        Ok(())
    } else {
        Err(Error::last_os_error())
    }
}

pub fn is_window_minimized(window_handle: winapi::shared::windef::HWND) -> Result<bool, Error> {
    let style = unsafe {
        winapi::um::winuser::GetWindowLongA(window_handle, winapi::um::winuser::GWL_STYLE)
    };
    if style == 0 {
        return Err(Error::last_os_error());
    }
    Ok(winapi::um::winuser::WS_MINIMIZE as i32 & style != 0)
}

fn get_process_id(window_handle: winapi::shared::windef::HWND) -> Result<u32, Error> {
    let mut process_id: u32 = 0;
    unsafe {
        winapi::um::winuser::GetWindowThreadProcessId(window_handle, &mut process_id);
    }
    if process_id != 0 {
        Ok(process_id)
    } else {
        Err(Error::last_os_error())
    }
}

fn get_process_handle(process_id: u32) -> Result<winapi::um::winnt::HANDLE, Error> {
    let handle: winapi::um::winnt::HANDLE = unsafe {
        winapi::um::processthreadsapi::OpenProcess(
            winapi::um::winnt::PROCESS_VM_READ | winapi::um::winnt::PROCESS_QUERY_INFORMATION,
            0,
            process_id
        )
    };
    if handle == null_mut() {
        Err(Error::last_os_error())
    } else {
        Ok(handle)
    }
}

fn get_process_module(process_handle: winapi::um::winnt::HANDLE) -> Result<winapi::shared::minwindef::HMODULE, Error> {
    let h_mod = ::std::ptr::null_mut();
    if unsafe {
        let mut cb_needed = 0;
        winapi::um::psapi::EnumProcessModulesEx(
            process_handle,
            h_mod as *mut *mut winapi::ctypes::c_void as _,
            ::std::mem::size_of::<u32>() as u32,
            &mut cb_needed,
            winapi::um::psapi::LIST_MODULES_ALL
        )
    } != 0 {
        Ok(h_mod)
    } else {
        Err(Error::last_os_error())
    }
}

fn get_process_file_name(process_handle: winapi::um::winnt::HANDLE, module_handle: winapi::shared::minwindef::HMODULE) -> Result<PathBuf, Error> {
    const BUFFER_LENGTH: usize = winapi::shared::minwindef::MAX_PATH + 1;
    let mut exe_buf = [0u16; BUFFER_LENGTH];
    if unsafe {
        winapi::um::psapi::GetModuleFileNameExW(process_handle,
                                                module_handle,
                                                exe_buf.as_mut_ptr(),
                                                BUFFER_LENGTH as u32
        )
    } == 0 {
        return Err(Error::last_os_error());
    }
    let mut pos = 0;
    for x in exe_buf.iter() {
        if *x == 0 {
            break;
        }
        pos += 1;
    }

    Ok(PathBuf::from(String::from_utf16_lossy(&exe_buf[..pos])))
}

pub fn is_in_autostart() -> Result<bool, Error> {
    unsafe {
        let key = get_autostart_hkey()?;
        let result = RegQueryValueExA(key, CString::new("poe-minimizer").unwrap().as_ptr(), null_mut(), &mut winapi::um::winnt::REG_SZ, null_mut(), null_mut());
        match result as u32 {
            ERROR_SUCCESS => Ok(true),
            ERROR_FILE_NOT_FOUND => Ok(false),
            _ => Err(Error::last_os_error())
        }
    }
}

pub fn add_to_autostart() -> Result<(), Error> {
    unsafe {
        let key = get_autostart_hkey()?;
        let current_path = std::env::current_exe()?;
        let current_path = current_path.to_str().unwrap().clone();

        let value = CString::new(format!("\"{}\"", current_path)).unwrap();
        let value = value.as_bytes_with_nul();
        winapi::um::winreg::RegSetValueExA(key, CString::new("poe-minimizer").unwrap().as_ptr(), 0, winapi::um::winnt::REG_SZ, value.as_ptr(), value.len() as u32);
    }
    Ok(())
}

pub fn remove_from_autostart() -> Result<(), Error> {
    unsafe {
        let key = get_autostart_hkey()?;
        winapi::um::winreg::RegDeleteValueA(key, CString::new("poe-minimizer").unwrap().as_ptr());
    }
    Ok(())
}

fn get_autostart_hkey() -> Result<winapi::shared::minwindef::HKEY, Error> {
    let mut key: winapi::shared::minwindef::HKEY = null_mut();
    unsafe {
        let result = winapi::um::winreg::RegCreateKeyA(HKEY_CURRENT_USER, CString::new("Software\\Microsoft\\Windows\\CurrentVersion\\Run").unwrap().as_ptr(), &mut key);
        if result != ERROR_SUCCESS as i32 {
            Err(Error::last_os_error())
        } else {
            Ok(key)
        }
    }
}
