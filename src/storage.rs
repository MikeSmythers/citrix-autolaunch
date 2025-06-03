use crate::{
    crypto::{decrypt_string, encrypt_string},
    io::{input, pw_input, spit_and_log},
};
use reqwest::{blocking, Url};
use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string};
use std::{
    fs::{self, OpenOptions},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

/*****************
 *   Constants   *
 ****************/

const ICA_FILE_NAME: &str = "AutoLaunch.ica";
const LOG_FILE_NAME: &str = "log.txt";
const SETTINGS_FILE_NAME: &str = "settings.txt";

/***************
 *   Structs   *
 **************/

/// User entered settings for the application
#[derive(Deserialize, Serialize)]
pub struct Settings {
    pub base_uri: String,
    pub application_name: String,
    pub login: String,
    pub passwd: String,
    pub target: String,
    pub maximization_active: bool,
}

/// Create blank copy of Settings struct
impl Default for Settings {
    fn default() -> Self {
        let empty = "".to_string();
        Self {
            base_uri: empty.clone(),
            application_name: empty.clone(),
            login: empty.clone(),
            passwd: empty.clone(),
            target: empty.clone(),
            maximization_active: false,
        }
    }
}

/// Destructor for Settings struct
impl Drop for Settings {
    fn drop(&mut self) {
        // Clear all fields of Settings struct
        self.base_uri.clear();
        self.application_name.clear();
        self.login.clear();
        self.passwd.clear();
        self.target.clear();
        self.maximization_active = false;
    }
}

impl Settings {
    /// Check if Settings are valid - returns bool
    pub fn is_valid(&self) -> bool {
        match Url::parse(&self.base_uri) {
            Ok(_) => (),
            Err(_) => return false,
        };
        !self.login.is_empty() || !self.passwd.is_empty()
    }
    /// Check if all fields of Settings are blank - returns bool
    pub fn is_empty(&self) -> bool {
        self.base_uri.is_empty() && self.login.is_empty() && self.passwd.is_empty()
    }
}

/****************
 *   File Ops   *
 ***************/

/// Get core data directory as PathBuf
/// - Checks if LOCALAPPDATA environment variable is accessible
/// - Checks if Citrix_AutoLaunch directory exists
/// - Creates Citrix_AutoLaunch directory if it does not exist
/// - Checks if Citrix_AutoLaunch directory is a valid path
/// - Returns Result<PathBuf, String>
fn get_data_directory() -> Result<PathBuf, String> {
    // Check if localappdata variable is accessible
    let localappdata = match std::env::var("LOCALAPPDATA") {
        Ok(path) => path,
        Err(_) => return Err("Failed to get LOCALAPPDATA environment variable.".to_string()),
    };
    // Check if localappdata is a valid path
    let path = Path::new(&localappdata);
    if !path.exists() {
        return Err("LOCALAPPDATA path does not exist.".to_string());
    }
    // Check if Citrix_AutoLaunch directory exists, and create if not
    let citrix_autolaunch_path = path.join("Citrix_AutoLaunch");
    if !citrix_autolaunch_path.exists() {
        match fs::create_dir(&citrix_autolaunch_path) {
            Ok(_) => (),
            Err(e) => {
                return Err(format!(
                    "Failed to create Citrix_AutoLaunch directory: {:?}",
                    e
                ))
            }
        };
    }
    // Check if Citrix_AutoLaunch directory is a valid path
    if !citrix_autolaunch_path.is_dir() {
        return Err("Citrix_AutoLaunch path is not a directory.".to_string());
    }
    Ok(citrix_autolaunch_path)
}

/// Verify core data directory is writeable
/// - Gets path from get_data_directory()
/// - Attempts to write a test file to Citrix_AutoLaunch directory
/// - If successful, deletes test file
/// - Returns Result<(), String>
fn verify_file_location() -> Result<(), String> {
    // Get data directory
    let citrix_autolaunch_path = match get_data_directory() {
        Ok(path) => path,
        Err(e) => return Err(format!("Failed to get data directory: {:?}", e)),
    };
    // Write test file to Citrix_AutoLaunch directory
    let test_file_path = citrix_autolaunch_path.join("test.txt");
    match fs::write(&test_file_path, "Test") {
        Ok(_) => (),
        Err(e) => return Err(format!("Failed to write test file: {:?}", e)),
    };
    // Delete test file
    match fs::remove_file(&test_file_path) {
        Ok(_) => (),
        Err(e) => return Err(format!("Failed to delete test file: {:?}", e)),
    };
    // Return Result<(), String>
    Ok(())
}

/// Get file as string from Citrix_AutoLaunch directory
/// - If file does not exist, returns error
/// - If file exists, attempts to read file and return contents as String
/// - Returns Result<String, String>
fn get_file_as_string(file_name: &str) -> Result<String, String> {
    // Get data directory
    let citrix_autolaunch_path = match get_data_directory() {
        Ok(path) => path,
        Err(e) => return Err(format!("Failed to get data directory: {:?}", e)),
    };
    // Append file name to path
    let file_path = citrix_autolaunch_path.join(file_name);
    // Check if file exists
    if !file_path.exists() {
        return Err(format!("File does not exist: {}", file_name));
    }
    // Attempt to read file
    match fs::read_to_string(file_path) {
        Ok(f) => Ok(f),
        Err(e) => Err(format!("Failed to read file: {:?}", e)),
    }
}

/// Set file in Citrix_AutoLaunch directory from string
/// - If file does not exist, creates new file
/// - If file exists, deletes existing file and creates new file
/// - Returns Result<(), String>
fn set_file_from_string(file_name: &str, file_string: &str) -> Result<(), String> {
    // Get data directory
    let citrix_autolaunch_path = match get_data_directory() {
        Ok(path) => path,
        Err(e) => return Err(format!("Failed to get data directory: {:?}", e)),
    };
    // Append file name to path
    let file_path = citrix_autolaunch_path.join(file_name);
    // Delete file if it exists
    if file_path.exists() {
        match fs::remove_file(&file_path) {
            Ok(_) => (),
            Err(e) => return Err(format!("Failed to delete existing file: {:?}", e)),
        };
    }
    // Attempt to write to file
    match fs::write(file_path, file_string) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Failed to write file: {:?}", e)),
    }
}

/// Load Settings from file
/// - If no file is found, creates new Settings via create_settings
pub fn get_settings() -> Result<Settings, String> {
    // Get file
    let f = match get_file_as_string(SETTINGS_FILE_NAME) {
        Ok(f) => f,
        Err(_) => {
            return create_settings("No settings file found.\r\nLet's make a new one!\r\n\r\n")
        }
    };
    // Check if file is empty (prevents unnecessary decryption and deserialization)
    // - If so, create new settings
    if f.is_empty() {
        return create_settings(
            "A settings file was found, but it's empty.\r\nLet's make a new one!\r\n\r\n",
        );
    }
    // Attempt to decrypt settings file
    // - If encryption key doesn't match (or other errors occur), create new settings
    let d =
        match decrypt_string(f) {
            Ok(d) => d,
            Err(_) => return create_settings(
                "A settings file was found, but the encryption key doesn't match.\r\nLet's make a new one!\r\n\r\n",
            ),
        };
    // Attempt to deserialize settings file
    // - If file is corrupted or data format doesn't match Settings struct, create new settings
    match from_str(&d) {
        Ok(s) => Ok(s),
        Err(_) => create_settings(
            "A settings file was found, but it's corrupted.\r\nLet's make a new one!\r\n\r\n",
        ),
    }
}

/// Save Settings to file
/// - Encrypts Settings before saving
/// - Returns Result<(), String>
fn save_settings(settings: &Settings) -> Result<(), String> {
    // Serialize settings to string
    let serialized = match to_string(&settings) {
        Ok(s) => s,
        Err(e) => return Err(format!("Failed to serialize settings: {:?}", e)),
    };

    // Encrypt settings string
    let encrypted = match encrypt_string(&serialized) {
        Ok(e) => e,
        Err(e) => return Err(format!("Failed to encrypt settings: {:?}", e)),
    };

    // Save to file
    match set_file_from_string(SETTINGS_FILE_NAME, encrypted.as_str()) {
        Ok(_) => Ok(()),
        Err(e) => return Err(format!("Failed to save settings: {:?}", e)),
    }
}

pub fn save_ica(data: &str) -> Result<(), String> {
    // Get data directory
    let citrix_autolaunch_path = match get_data_directory() {
        Ok(path) => path,
        Err(e) => return Err(format!("Failed to get data directory: {:?}", e)),
    };
    // Append file name to path
    let file_path = citrix_autolaunch_path.join(ICA_FILE_NAME);
    // Attempt to write to file
    match fs::write(file_path, data) {
        Ok(_) => Ok(()),
        Err(e) => return Err(format!("Failed to write ICA file: {:?}", e)),
    }
}

/// Launch file in default application
/// - Returns Result<(), String>
pub fn launch_ica(file_name: &str) -> Result<(), String> {
    // Get data directory
    let citrix_autolaunch_path = match get_data_directory() {
        Ok(path) => path,
        Err(e) => return Err(format!("Failed to get data directory: {:?}", e)),
    };
    // Append file name to path
    let file_path = citrix_autolaunch_path.join(ICA_FILE_NAME);
    // Check if file exists
    if !file_path.exists() {
        return Err(format!("File does not exist: {}", file_name));
    }
    // Attempt to launch file using open crate
    match open::that(file_path) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Failed to launch file: {:?}", e)),
    }
}

/******************
 *   User Input   *
 *****************/

/// Create and save Settings from user input
/// - Validates URL prior to saving
fn create_settings(reason: &str) -> Result<Settings, String> {
    // Check that core data directory is writeable
    match verify_file_location() {
        Ok(_) => (),
        Err(e) => println!("File system access failed: {:?}", e),
    };

    // Inform user why settings are being created
    spit_and_log(reason);

    // Get base URI from user input
    let base_uri = input("Base URI (https): ");

    // Check that base URI is valid, uses SSL, and is reachable
    spit_and_log("Verifying gateway...");
    let input_uri = match Url::parse(&base_uri) {
        Ok(u) => u,
        Err(e) => return Err(format!("Invalid URI: {:?}", e)),
    };
    if input_uri.scheme() != "https" {
        return Err("URI must use HTTPS.".to_string());
    }
    match blocking::get(input_uri) {
        Ok(r) => {
            let response = match r.text() {
                Ok(t) => t,
                Err(e) => return Err(format!("Failed to read response: {:?}", e)),
            };
            if response.contains("Citrix") {
                spit_and_log("Gateway verified.");
            } else {
                return Err("Gateway not recognized.".to_string());
            }
        }
        Err(e) => return Err(format!("Failed to connect to gateway: {:?}", e)),
    };

    // Get application name, login, password, and maximization settings from user input
    let application_name: String = input("Application to launch: ");
    let login = input("Login: ");
    let passwd = pw_input("Password: ");
    let maximization_active = input("Maximize window on launch? (y/n): ") == "y";
    let target = match maximization_active {
        true => input("Title of window to maximize: "),
        false => "".to_string(),
    };

    // Create settings struct
    let settings = Settings {
        base_uri,
        application_name,
        login,
        passwd,
        target,
        maximization_active,
    };

    // Check if settings are valid
    if !settings.is_valid() {
        return create_settings("Invalid settings. Please try again.\r\n\r\n");
    }

    // Save settings to file
    match save_settings(&settings) {
        Ok(_) => Ok(settings),
        Err(e) => Err(e),
    }
}

/************************
*   System Operations   *
************************/

/// Log out to file
/// - Accepts a string to log
/// - Logs to the LOG_FILE
/// - If the file does not exist, it is created
/// - If the file exists, the log is appended
/// - If the file is longer than 500 lines, only the last 500 lines are kept
/// - Condenses consecutive repeated lines with quantity marker
/// - Does not return anything
pub fn log_to_file(input: &str) {
    // Get data directory
    let citrix_autolaunch_path = match get_data_directory() {
        Ok(path) => path,
        Err(_) => return,
    };
    // Append file name to path
    let log_file_path = citrix_autolaunch_path.join(LOG_FILE_NAME);
    // Open file
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(&log_file_path);
    let file = match file {
        Ok(file) => file,
        Err(_) => return,
    };

    // Create new content
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let new_line = if input.is_empty() {
        ""
    } else {
        &format!("[{}] {}", timestamp, input)
    };
    let mut content = String::new();

    // Read existing file
    let reader = BufReader::new(file);
    if let Ok(lines) = reader.lines().collect::<Result<Vec<String>, _>>() {
        // Split lines
        let mut lines = lines;

        // Condense or append
        if let Some(last_line) = lines.last() {
            let (condensed, condensed_line) = condense_repetition(last_line, new_line);
            if condensed {
                lines.pop();
                lines.push(condensed_line);
            } else {
                lines.push(new_line.to_string());
            }
        } else {
            lines.push(new_line.to_string());
        }

        // Trim to 500 lines
        if lines.len() > 500 {
            lines = lines[lines.len() - 500..].to_vec();
        }

        // Create output string
        content = lines.join("\n");
        if lines.len() > 0 {
            content.push('\n');
        }
    } else {
        // Create new content
        content.push_str(new_line);
    }

    // Write to file
    if let Ok(_) = std::fs::write(log_file_path, content) {}
}

/* ---------------- UTILITIES ---------------- */

fn condense_repetition(last_line: &str, new_line: &str) -> (bool, String) {
    if !last_line.contains(']') {
        return (false, new_line.to_string());
    }
    let last_line_text = last_line.split(']').collect::<Vec<&str>>()[1].trim();
    let new_line_text = new_line.split(']').collect::<Vec<&str>>()[1].trim();
    if last_line_text == new_line_text {
        return (true, format!("{} (2)", new_line));
    }
    if last_line_text.starts_with(format!("{} (", new_line_text).as_str()) {
        let count = last_line_text
            .split(' ')
            .collect::<Vec<&str>>()
            .last()
            .unwrap()
            .replace("(", "")
            .replace(")", "")
            .parse::<u32>()
            .unwrap();
        return (true, format!("{} ({})", new_line, count + 1));
    }
    return (false, new_line.to_string());
}
