extern crate config;
extern crate rev_lines;
extern crate serde;
#[cfg(windows)]
extern crate winapi;

use std::{thread, time};
use std::fs::File;
use std::io::BufReader;
use std::io::Error;
use std::time::{Duration, SystemTime};

use config::{ConfigError};
use rev_lines::RevLines;
use serde::{Deserialize};

const WINDOW_NAME: &str = "window_name";
const LOG_FILE_LOCATION: &str = "log_file_location";
const LOG_FILE_POLLING_INTERVAL_MS: &str = "log_file_polling_interval_ms";
const AFK_MARKER: &str = "afk_marker";
const AFK_MARKER_ON: &str = "afk_marker_on";
const SECONDS_UNTIL_MINIMIZE: &str = "seconds_until_minimize";
const SECONDS_TO_CHECK_FOR_POE: &str = "seconds_to_check_for_poe";

#[derive(Debug, Deserialize)]
struct Settings {
    window_name: String,
    log_file_location: String,
    log_file_polling_interval_ms: u64,
    afk_marker: String,
    afk_marker_on: String,
    seconds_until_minimize: u64,
    seconds_to_check_for_poe: u64,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let mut config = config::Config::default();
        config.set_default(WINDOW_NAME, "Path of Exile")?;
        config.set_default(LOG_FILE_LOCATION, "C:\\Program Files (x86)\\Grinding Gear Games\\Path of Exile\\logs\\Client.txt")?;
        config.set_default(LOG_FILE_POLLING_INTERVAL_MS, 500)?;
        config.set_default(AFK_MARKER, "AFK mode is now")?;
        config.set_default(AFK_MARKER_ON, "ON")?;
        config.set_default(SECONDS_UNTIL_MINIMIZE, 5)?;
        config.set_default(SECONDS_TO_CHECK_FOR_POE, 30)?;
        config.merge(config::File::with_name("Settings"))?;
        config.try_into()
    }
}

fn main() {
    let settings: Settings = Settings::new().expect("Failed to load config");
    loop {
        let _ = match get_window_handle(&settings) {
            Ok(handle) => check_for_minimization(handle, &settings),
            Err(error) => Err(error)
        };
        thread::sleep(Duration::from_secs(settings.seconds_to_check_for_poe))
    }
}


fn check_for_minimization(handle: winapi::shared::windef::HWND, settings: &Settings) -> Result<(), Error> {
    let mut afk_status = false;
    let mut was_minimized = false;
    let mut last_time_maximized = SystemTime::now();
    loop {
        match get_last_afk_status_from_log(settings) {
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

fn get_last_afk_status_from_log(settings: &Settings) -> Option<bool> {
    let file = File::open(&settings.log_file_location).unwrap();
    let rev_lines = RevLines::new(BufReader::new(file)).unwrap();
    rev_lines
        .take(20)
        .skip_while(|x| !x.contains(&settings.afk_marker))
        //.inspect(|x| println!("{}", x))
        .map(|x| x.contains(&settings.afk_marker_on))
        .next()
}

fn get_window_handle(settings: &Settings) -> Result<winapi::shared::windef::HWND, Error> {
    use std::ptr::null_mut;
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
