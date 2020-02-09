#![windows_subsystem = "windows"]
#[macro_use]
extern crate log;
extern crate rev_lines;
extern crate simplelog;
extern crate widestring;
#[cfg(windows)]
extern crate winapi;

use std::{thread, time};
use std::fs::File;
use std::io::{BufReader, ErrorKind};
use std::io::Error;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use log::LevelFilter;
use rev_lines::RevLines;
use simplelog::WriteLogger;
use winapi::_core::sync::atomic::{AtomicBool, Ordering};

use utils::*;

use crate::tray::Event;

mod winutils;
mod tray;
mod utils;

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
            seconds_to_check_for_poe: 30,
        }
    }
}

static RUNNING: AtomicBool = AtomicBool::new(true);

fn main() {
    init_logger();
    let handle = thread::spawn(move || {
        main_service();
    });
    main_window().log_error_and_ignore("error in message event loop");
    let _ = handle.join();
}

fn init_logger() {
    let args: Vec<String> = std::env::args().collect();
    let level = if args.len() < 2 {
        LevelFilter::Warn
    } else {
        match args[1].as_str() {
            "trace" => LevelFilter::Trace,
            "debug" => LevelFilter::Debug,
            "info" => LevelFilter::Info,
            _ => LevelFilter::Warn
        }
    };

    simplelog::CombinedLogger::init(
        vec![
            WriteLogger::new(level, simplelog::Config::default(), File::create("log.txt").unwrap())
        ]
    ).unwrap();
}

fn main_window() -> Result<(), Error> {
    let message_window_handle = tray::create_message_window()?;
    while RUNNING.load(Ordering::Relaxed) {
        let event = tray::handle_windows_messages(message_window_handle)?;
        match event {
            Event::Exit => {
                tray::destroy_message_window(message_window_handle)?;
                RUNNING.store(false, Ordering::Relaxed);
            }
            Event::Nothing => {}
        }
    }
    Ok(())
}

fn main_service() {
    let settings: Settings = Settings::default();

    loop {
        if let Ok(handle) = winutils::get_window_handle(&settings.window_name)
            .log_info("failed to get window handle for Path Of Exile") {
            check_for_minimization(handle, &settings)
                .log_error_and_ignore("failed to minimize window");
        };
        if !RUNNING.load(Ordering::Relaxed) {
            break;
        }
        thread::sleep(Duration::from_secs(settings.seconds_to_check_for_poe))
    }
}

fn find_log_path(window_handle: winapi::shared::windef::HWND) -> Result<String, Error> {
    use winutils::*;
    get_process_path_by_window_handle(window_handle)
        .map(|poe_path| construct_log_path(poe_path))
        .and_then(|path_option| path_option.as_result(Error::new(ErrorKind::Other, "failed to construct client.txt path.")))
}

fn construct_log_path(poe_executable_path: PathBuf) -> Option<String> {
    poe_executable_path.parent()
        .map(|path| path.join("logs\\Client.txt"))
        .and_then(|path_buffer| path_buffer.to_str().map(|x| x.to_string()))
}

fn check_for_minimization(handle: winapi::shared::windef::HWND, settings: &Settings) -> Result<(), Error> {
    let mut afk_status = false;
    let mut was_minimized = false;
    let mut last_time_maximized = SystemTime::now();
    let log_path = find_log_path(handle)?;
    while RUNNING.load(Ordering::Relaxed) {
        match get_last_afk_status_from_log(&log_path)? {
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

        if afk_status && elapsed > settings.seconds_until_minimize {
            winutils::minimize_window(handle)?;
        }

        thread::sleep(time::Duration::from_millis(settings.log_file_polling_interval_ms));
    }
    Ok(())
}

fn get_last_afk_status_from_log(log_path: &str) -> Result<Option<bool>, Error> {
    let file = File::open(log_path)?;
    let rev_lines = RevLines::new(BufReader::new(file))?;
    let afk_status = rev_lines
        .take(20)
        .skip_while(|x| !x.contains("ac9")) // magic constant in log for chat related stuff
        .skip_while(|x| x.contains("] @")) // otherwise ppl could send forged messages
        .inspect(|x| trace!("log line: {}", x))
        .map(|x| log_line_as_afk_status(x.as_str()))
        .next()
        .flatten();
    Ok(afk_status)
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

