use std::io::Write;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use anyhow::Result;
use humantime::format_duration;
use terminal_size::terminal_size;

use crate::database::{Database, Entry};

pub fn track(mut db: Database<std::fs::File>, category: &str) -> Result<()> {
    let pair = Arc::new((Mutex::new(false), Condvar::new()));
    let p = pair.clone();
    ctrlc::set_handler(move || {
        let (lock, cvar) = &*p;
        *lock.lock().unwrap() = true;
        cvar.notify_one();
    })?;

    let start = Instant::now();
    let mut elapsed = Duration::default();
    let max_secs = 5.0;

    let (lock, cvar) = &*pair;
    let mut stop = lock.lock().unwrap();

    while !*stop {
        let term_w = terminal_size().map(|(w, _)| w.0 as usize).unwrap_or(80);
        elapsed = elapsed.saturating_add(Duration::from_secs(1));
        let elapsed_secs = elapsed.as_secs_f64();
        let elapsed_str = format_duration(elapsed).to_string();

        // Build content: category, padding, elapsed_str
        let mut content = Vec::with_capacity(term_w);
        content.extend(category.chars());
        let padding = term_w.saturating_sub(category.len() + elapsed_str.len());
        for _ in 0..padding {
            content.push(' ');
        }
        content.extend(elapsed_str.chars());

        // Print progress bar
        let filled = ((elapsed_secs / max_secs) * term_w as f64).clamp(0.0, term_w as f64) as usize;
        let mut line = String::with_capacity(term_w + 64);
        line.push('\r');
        for (i, &ch) in content.iter().enumerate() {
            if i < filled {
                line.push_str("\x1b[47;30m");
            } else {
                line.push_str("\x1b[100m");
            }
            line.push(ch);
        }
        line.push_str("\x1b[0m");
        print!("{}", line);
        std::io::stdout().flush()?;

        stop = cvar.wait_timeout(stop, Duration::from_secs(1)).unwrap().0;
    }

    drop(stop);

    let elapsed = start.elapsed();
    let end_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let start_ts = end_ts - elapsed.as_secs();

    let entry = Entry {
        category: category.into(),
        start_time: start_ts,
        end_time: end_ts,
    };
    db.append_entry(&entry)?;

    // Move past the progress bar line
    print!("\n");
    std::io::stdout().flush()?;

    Ok(())
}
