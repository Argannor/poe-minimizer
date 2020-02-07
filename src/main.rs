extern crate rev_lines;
#[cfg(windows)]
extern crate winapi;

use std::{thread, time};
use std::fs::File;
use std::io::BufReader;
use std::io::Error;
use std::time::{Duration, SystemTime};

use rev_lines::RevLines;

mod winutils;

struct Settings {
    window_name: String,
    log_file_polling_interval_ms: u64,
    seconds_until_minimize: u64,
    seconds_to_check_for_poe: u64,
}

impl Settings {
    pub fn default() -> Self {
        Settings {
            window_name: "Path of Exile".to_owned(),
            log_file_polling_interval_ms: 500,
            seconds_until_minimize: 5,
            seconds_to_check_for_poe: 30
        }
    }
}

fn main() {
    let settings: Settings = Settings::default();
    loop {
        let _ = match winutils::get_window_handle(&settings.window_name) {
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
    use winutils::*;
    get_process_path_by_window_handle(window_handle)
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
        let minimized = winutils::is_window_minimized(handle)?;
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
            winutils::minimize_window(handle)?;
        }

        thread::sleep(time::Duration::from_millis(settings.log_file_polling_interval_ms));
    }
}

fn get_last_afk_status_from_log(settings: &Settings, log_path: &str) -> Option<bool> {
    let file = File::open(log_path).unwrap();
    let rev_lines = RevLines::new(BufReader::new(file)).unwrap();
    rev_lines
        .take(20)
        .skip_while(|x| !x.contains("ac9")) // magic constant in log for chat related stuff
        .skip_while(|x| x.contains("] @")) // otherwise ppl could send forged messages
        //        .inspect(|x| println!("log line: {}", x))
        .map(|x| log_line_as_afk_status(x.as_str()))
        .next()
        .flatten()
}

fn log_line_as_afk_status(log_line: &str) -> Option<bool> {
    if is_afk_activated_message(log_line) {
        Some(true)
    } else if is_afk_deactivated_message(log_line) {
        Some(false)
    } else {
        None
    }
}

fn is_afk_activated_message(log_line: &str) -> bool {
    log_line.contains(": AFK mode is now ON.")
    || log_line.contains(": Le mode Absent (AFK) est désormais activé.")
    || log_line.contains(": AFK-Modus ist nun AN.")
    || log_line.contains(": Modo LDT Ativado.")
    || log_line.contains(": Режим \"отошёл\" включён.")
    || log_line.contains(": เปิดโหมด AFK แล้ว ตอบกลับอัตโนมัติ")
    || log_line.contains(": El modo Ausente está habilitado.")
    || log_line.contains(": 자리 비움 모드를 설정했습니다.")
}

fn is_afk_deactivated_message(log_line: &str) -> bool {
    log_line.contains(": AFK mode is now OFF.")
    || log_line.contains(": Le mode Absent (AFK) est désactivé.")
    || log_line.contains(": AFK-Modus ist nun AUS.")
    || log_line.contains(": Modo LDT Desativado.")
    || log_line.contains(": Режим \"отошёл\" выключен.")
    || log_line.contains(": ปิดโหมด AFK แล้ว")
    || log_line.contains(": El modo Ausente está deshabilitado.")
    || log_line.contains(": 자리 비움 모드를 해제했습니다.")
}

