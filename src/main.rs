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
use io::spit;
use maximize::maximize_window;
use std::thread::sleep;
use storage::launch_file;

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
    let mut settings = storage::Settings::default();
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
                spit("Initializing...");
                match storage::get_settings() {
                    Ok(s) => {
                        settings = s;
                        spit("Settings loaded successfully.");
                    }
                    Err(e) => {
                        let msg = format!(
                            "Error: {}\r\n\r\nFailed to get settings. Retrying in 5 seconds.",
                            e
                        );
                        settings = storage::Settings::default();
                        spit(msg);
                        sleep(std::time::Duration::from_secs(5));
                    }
                };
            }
            State::ReadyToLogIn => {
                // Log into Citrix StoreFront and get ICA file
                spit("Logging in...");
                match get_ica_file(&settings) {
                    Ok(f) => {
                        file_name = f;
                        spit("ICA file downloaded successfully.");
                    }
                    Err(e) => {
                        let msg = format!(
                            "Error: {}\r\n\r\nFailed to get ICA file. Retrying in 5 seconds.",
                            e
                        );
                        spit(msg);
                        settings = storage::Settings::default();
                        file_name = String::new();
                        sleep(std::time::Duration::from_secs(5));
                    }
                };
            }
            State::ReadyToLaunch => {
                // Launch ICA file in default application
                spit("Launching file...");
                let target = file_name.clone();
                file_name = String::new();
                match launch_file(target.as_str()) {
                    Ok(_) => {
                        let msg = format!("File launched successfully: {}", target);
                        spit(msg);
                        sleep(std::time::Duration::from_secs(5));
                    }
                    Err(e) => {
                        let msg = format!(
                            "Error: {}\r\n\r\nFailed to launch file. Retrying in 5 seconds.",
                            e
                        );
                        spit(msg);
                        file_name = String::new();
                        sleep(std::time::Duration::from_secs(5));
                    }
                };
            }
            State::Active => {
                // Attempt to maximize target titled window (best effort only)
                if settings.maximization_active {
                    maximize_window(&settings.target);
                }
                sleep(std::time::Duration::from_secs(1));
            }
        }
    }
}
