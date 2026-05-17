use std::io::{self, BufRead, Write};

pub fn confirm(prompt: &str) -> bool {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    loop {
        print!("{} [y/N] ", prompt);
        let _ = stdout.flush();
        let mut line = String::new();
        stdin.lock().read_line(&mut line).expect("stdin read");
        match line.trim().to_lowercase().as_str() {
            "y" | "yes" => return true,
            "n" | "no" | "" => return false,
            _ => continue,
        }
    }
}
