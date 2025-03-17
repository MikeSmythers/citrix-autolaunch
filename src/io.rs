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

/// Log out to file
/// - Accepts a string to log
/// - Logs to the LOG_FILE
/// - If the file does not exist, it is created
/// - If the file exists, the log is appended
/// - If the file is longer than 500 lines, only the last 500 lines are kept
/// - Condenses consecutive repeated lines with quantity marker
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
    if let Ok(_) = std::fs::write(LOG_FILE, content) {}
}

/// Combination of input and spit
/// - Accepts a prompt string
/// - Logs and spits
/// - Does not return anything
pub fn spit_and_log(input: &str) {
    log_to_file(&input);
    spit(&input);
}

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
