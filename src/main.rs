/*
    Citrix StoreFront-based application launcher
*/

mod citrix;
mod crypto;
mod extract;
mod io;
mod maximize;
mod storage;
use citrix::{get_ica_file, ica_is_running};
use io::spit_and_log;
use maximize::maximize_window;
use std::{thread::sleep, time::Duration};
use storage::{launch_file, Settings};

/// Application state options
enum State {
    Initialization,
    ReadyToLogIn,
    ReadyToLaunch,
    Active,
}

/// Baseline application logical flow
/// - Check state
///   - If wfica32.exe is running, try to maximize the target window
///     - 1 second delay before running again
///   - If settings are not loaded or invalid, attempt to load or get them
///     - Success moves on immediately
///     - Errors result in 5 second delay
///   - If ICA file is not downloaded, attempt to get it
///     - Success moves on immediately
///     - Errors result in 5 second delay
///   - If ICA file is downloaded, attempt to launch it
///     - Success moves on immediately
///     - Errors result in 5 second delay
fn main() {
    let mut state: State;
    let mut settings = Settings::default();
    let mut file_name = String::new();
    loop {
        // Check and set state
        if ica_is_running() {
            state = State::Active;
        } else if settings.is_empty() || !settings.is_valid() {
            state = State::Initialization;
        } else if file_name.is_empty() {
            state = State::ReadyToLogIn;
        } else {
            state = State::ReadyToLaunch;
        }
        // Perform actions based on state
        match state {
            State::Initialization => {
                // Attempt to get settings from file or user input
                spit_and_log("Initializing...");
                match storage::get_settings() {
                    Ok(s) => {
                        settings = s;
                        spit_and_log("Settings loaded successfully.");
                    }
                    Err(e) => {
                        let msg = format!(
                            "Error: {}\r\n\r\nFailed to get settings. Retrying in 5 seconds.",
                            e
                        );
                        settings = Settings::default();
                        spit_and_log(&msg);
                        sleep(Duration::from_secs(5));
                    }
                };
            }
            State::ReadyToLogIn => {
                // Log into Citrix StoreFront and get ICA file
                spit_and_log("Logging in...");
                match get_ica_file(&settings) {
                    Ok(f) => {
                        file_name = f;
                        spit_and_log("ICA file downloaded successfully.");
                    }
                    Err(e) => {
                        let msg = format!(
                            "Error: {}\r\n\r\nFailed to get ICA file. Retrying in 5 seconds.",
                            e
                        );
                        spit_and_log(&msg);
                        settings = Settings::default();
                        file_name = String::new();
                        sleep(Duration::from_secs(5));
                    }
                };
            }
            State::ReadyToLaunch => {
                // Launch ICA file in default application
                spit_and_log("Launching file...");
                let target = file_name.clone();
                file_name = String::new();
                match launch_file(target.as_str()) {
                    Ok(_) => {
                        let msg = format!("File launched successfully: {}", target);
                        spit_and_log(&msg);
                        sleep(Duration::from_secs(5));
                    }
                    Err(e) => {
                        let msg = format!(
                            "Error: {}\r\n\r\nFailed to launch file. Retrying in 5 seconds.",
                            e
                        );
                        spit_and_log(&msg);
                        file_name = String::new();
                        sleep(Duration::from_secs(5));
                    }
                };
            }
            State::Active => {
                // Attempt to maximize target titled window (best effort only)
                if settings.maximization_active {
                    maximize_window(&settings.target);
                }
                sleep(Duration::from_secs(1));
            }
        }
    }
}
