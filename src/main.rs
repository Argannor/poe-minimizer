extern crate rev_lines;
#[cfg(windows)]
extern crate winapi;

use std::{thread, time};
use std::fs::File;
use std::io::BufReader;
use std::io::Error;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use std::ptr::null_mut;

use rev_lines::RevLines;

struct Settings {
    window_name: String,
    log_file_polling_interval_ms: u64,
    afk_marker: String,
    afk_marker_on: String,
    seconds_until_minimize: u64,
    seconds_to_check_for_poe: u64,
}

impl Settings {
    pub fn default() -> Self {
        Settings {
            window_name: "Path of Exile".to_owned(),
            log_file_polling_interval_ms: 500,
            afk_marker: "AFK mode is now".to_owned(),
            afk_marker_on: "ON".to_owned(),
            seconds_until_minimize: 5,
            seconds_to_check_for_poe: 30
        }
    }
}

fn main() {
    let settings: Settings = Settings::default();
    loop {
        let _ = match get_window_handle(&settings) {
            Ok(handle) => check_for_minimization(handle, &settings),
            Err(error) => {
                println!("couldn't find window {:?}", error);
                Err(error)
            }
        };
        thread::sleep(Duration::from_secs(settings.seconds_to_check_for_poe))
    }
}

fn find_log_path(window_handle: winapi::shared::windef::HWND) -> Option<String> {
    get_process_id(window_handle)
        .and_then(|process_id|  get_process_handle(process_id).map(|handle| (process_id, handle)))
        .and_then(|(process_id, handle)| get_process_module(handle).map(|module_handle| (process_id, handle, module_handle)))
        .and_then(|(_, process_handle, module_handle)| get_process_file_name(process_handle, module_handle))
        .map(|poe_path| poe_path.parent().map(|path| path.join("logs\\Client.txt")))
        .map(|path_buffer| path_buffer.and_then(|path| path.to_str().map(|x| x.to_string())))
        .unwrap_or(None)
}

fn check_for_minimization(handle: winapi::shared::windef::HWND, settings: &Settings) -> Result<(), Error> {
    let mut afk_status = false;
    let mut was_minimized = false;
    let mut last_time_maximized = SystemTime::now();
    let log_path = find_log_path(handle).expect("Failed to find Client.txt");
    loop {
        match get_last_afk_status_from_log(settings, &log_path) {
            Some(afk) => afk_status = afk,
            None => {}
        }
        let minimized = is_window_minimized(handle)?;
        if !minimized && minimized != was_minimized {
            last_time_maximized = SystemTime::now();
        }
        was_minimized = minimized;

        let elapsed = match last_time_maximized.elapsed() {
            Ok(elapsed) => {
                elapsed.as_secs()
            }
            Err(_) => 0
        };
        //println!("afk: {}, elapsed: {}", afk_status, elapsed);
        if afk_status && elapsed > settings.seconds_until_minimize {
            minimize_window(handle)?;
        }

        thread::sleep(time::Duration::from_millis(settings.log_file_polling_interval_ms));
    }
}

fn get_last_afk_status_from_log(settings: &Settings, log_path: &str) -> Option<bool> {
    let file = File::open(log_path).unwrap();
    let rev_lines = RevLines::new(BufReader::new(file)).unwrap();
    rev_lines
        .take(20)
        .skip_while(|x| !x.contains(&settings.afk_marker))
//        .inspect(|x| println!("log line: {}", x))
        .map(|x| x.contains(&settings.afk_marker_on))
        .next()
}

fn get_window_handle(settings: &Settings) -> Result<winapi::shared::windef::HWND, Error> {
    use std::ffi::CString;
    let window_handle = unsafe {
        let window_title = CString::new(settings.window_name.as_str()).expect("CString::new failed");
        winapi::um::winuser::FindWindowA(null_mut(), window_title.as_ptr())
    };
    if window_handle == null_mut() {
        Err(Error::last_os_error())
    } else {
        Ok(window_handle)
    }
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

fn minimize_window(window_handle: winapi::shared::windef::HWND) -> Result<(), Error> {
    let result = unsafe {
        // see SW_MINIMIZE:
        // https://docs.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-showwindow
        let minimize: i32 = 6;
        winapi::um::winuser::ShowWindow(window_handle, minimize)
    };
    if result != 0 {
        Ok(())
    } else {
        Err(Error::last_os_error())
    }
}

fn is_window_minimized(window_handle: winapi::shared::windef::HWND) -> Result<bool, Error> {
    // https://docs.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getwindowlonga
    let gwl_style = -16;
    let style = unsafe {
        winapi::um::winuser::GetWindowLongA(window_handle, gwl_style)
    };
    if style == 0 {
        return Err(Error::last_os_error());
    }
    let ws_minimize = 0x20000000i32;
    Ok(ws_minimize & style != 0)
}
