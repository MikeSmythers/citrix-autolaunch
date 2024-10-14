use crate::{
    crypto::{decrypt_string, encrypt_string},
    io::{input, spit},
};
use reqwest::{blocking, Url};
use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string};
use std::{fs, path::Path};

// TODO: Allow user to modify location and name of settings file
const SETTINGS_FILE: &str = "settings.txt";

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

/// Create and save Settings from user input
/// - Validates URL prior to saving
fn create_settings(reason: &str) -> Result<Settings, String> {
    spit(reason);
    let base_uri = input("Base URI (https): ");
    spit("Verifying gateway...");
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
                spit("Gateway verified.");
            } else {
                return Err("Gateway not recognized.".to_string());
            }
        }
        Err(e) => return Err(format!("Failed to connect to gateway: {:?}", e)),
    };
    let application_name: String = input("Application to launch: ");
    let login = input("Login: ");
    let passwd = input("Password: ");
    let maximization_active = input("Maximize window on launch? (y/n): ") == "y";
    let target = match maximization_active {
        true => input("Title of window to maximize: "),
        false => "".to_string(),
    };
    let settings = Settings {
        base_uri,
        application_name,
        login,
        passwd,
        target,
        maximization_active,
    };
    if !settings.is_valid() {
        return create_settings("Invalid settings. Please try again.\r\n\r\n");
    }
    match save_settings(&settings) {
        Ok(_) => Ok(settings),
        Err(e) => Err(e),
    }
}

/// Load Settings from file
/// - If no file is found, creates new Settings via create_settings
pub fn get_settings() -> Result<Settings, String> {
    // Check if settings file exists
    // - If not, create new settings
    match Path::new(&SETTINGS_FILE).exists() {
        true => (),
        false => {
            return create_settings(
                "No settings file was found in the current directory.\r\nLet's create one!\r\n\r\n",
            );
        }
    };
    // Attempt to read existing settings file
    // - If file is empty, corrupted, or unreadable, create new settings
    // TODO: Is this creating a race condition when file disappears or no perm to read?
    let f =
        match fs::read_to_string(&SETTINGS_FILE) {
            Ok(f) => f,
            Err(_) => return create_settings(
                "A settings file was found, but it's unreadable.\r\nLet's make a new one!\r\n\r\n",
            ),
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
pub fn save_settings(settings: &Settings) -> Result<(), String> {
    let serialized = match to_string(&settings) {
        Ok(s) => s,
        Err(e) => return Err(format!("Failed to serialize settings: {:?}", e)),
    };
    let encrypted = match encrypt_string(&serialized) {
        Ok(e) => e,
        Err(e) => return Err(format!("Failed to encrypt settings: {:?}", e)),
    };
    match fs::write(SETTINGS_FILE, encrypted) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Failed to write settings file: {:?}", e)),
    }
}

/// Launch file in default application
/// - Returns Result<(), String>
pub fn launch_file(file_name: &str) -> Result<(), String> {
    match open::that(file_name) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Failed to launch file: {:?}", e)),
    }
}
