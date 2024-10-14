use std::{
    fmt::Display,
    io::{stdin, stdout, Write},
};

/// Get input from the console
/// - Accepts a prompt string
/// - Returns the trimmed input as a String
pub fn input(prompt: &str) -> String {
    print!("{}", prompt);
    // Flush STDOUT; ignore errors
    match stdout().flush() {
        Ok(_) => {}
        Err(_) => {}
    };
    let mut input = String::new();
    // Get user input; ignore errors
    match stdin().read_line(&mut input) {
        Ok(_) => {}
        Err(_) => {}
    };
    input.trim().to_string()
}

/// Print to console
/// - Accepts any type that implements Display
/// - Prints only this content and a newline
pub fn spit(content: impl Display) {
    println!("{}", content);
}
