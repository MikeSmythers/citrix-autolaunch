use std::{
    fmt::Display,
    fs::OpenOptions,
    io::{stdin, stdout, BufRead, BufReader, Write},
};

const LOG_FILE: &str = "log.txt";

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

/// Log out to file
/// - Accepts a string to log
/// - Logs to the LOG_FILE
/// - If the file does not exist, it is created
/// - If the file exists, the log is appended
/// - If the file is longer than 500 lines, only the last 500 lines are kept
/// - Does not return anything
pub fn log_to_file(input: &str) {
    // Open file
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(LOG_FILE);
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
        let mut lines = lines;
        if lines.len() > 500 {
            lines = lines[lines.len() - 500..].to_vec();
        }
        content = lines.join("\n");
        if lines.len() > 0 {
            content.push('\n');
        }
    }

    // Append and write
    content.push_str(new_line);
    let result = std::fs::write(LOG_FILE, content);
    match result {
        Ok(_) => {}
        Err(_) => {}
    };
}

/// Combination of input and spit
/// - Accepts a prompt string
/// - Logs and spits
/// - Does not return anything
pub fn spit_and_log(input: &str) {
    log_to_file(&input);
    spit(&input);
}
