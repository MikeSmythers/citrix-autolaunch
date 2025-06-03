use std::{
    fmt::Display,
    io::{stdin, stdout, Write},
};

use crate::storage::log_to_file;

/// Get input from the console
/// - Accepts a prompt string
/// - Returns the trimmed input as a String
pub fn input(prompt: &str) -> String {
    print!("{}", prompt);
    // Flush STDOUT; ignore errors
    if let Ok(_) = stdout().flush() {}
    let mut input = String::new();
    // Get user input; ignore errors
    if let Ok(_) = stdin().read_line(&mut input) {}
    input.trim().to_string()
}

/// Get password from the console
/// - Accepts a prompt string
/// - Returns the trimmed input as a String
pub fn pw_input(prompt: &str) -> String {
    let mut success = false;
    let mut password = String::new();
    while !success {
        let result = rpassword::prompt_password(prompt);
        match result {
            Ok(pw) => {
                success = true;
                password = pw;
            }
            Err(_) => (),
        }
    }
    password
}

/// Print to console
/// - Accepts any type that implements Display
/// - Prints only this content and a newline
pub fn spit(content: impl Display) {
    println!("{}", content);
}

/// Combination of input and spit
/// - Accepts a prompt string
/// - Logs and spits
/// - Does not return anything
pub fn spit_and_log(input: &str) {
    log_to_file(&input);
    spit(&input);
}
