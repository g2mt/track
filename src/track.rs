use std::io::Write;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, SystemTime};

use anyhow::Result;
use humantime::format_duration;
use terminal_size::terminal_size;

use crate::database::{Database, Entry};
use crate::utils;

pub fn track(mut db: Database<std::fs::File>, category: Arc<str>) -> Result<()> {
    let mut info = db.read_info()?.unwrap_or_default();
    info.add_category(category.clone());
    db.write_info(&info)?;

    let pair = Arc::new((Mutex::new(false), Condvar::new()));
    let p = pair.clone();
    ctrlc::set_handler(move || {
        let (lock, cvar) = &*p;
        *lock.lock().unwrap() = true;
        cvar.notify_one();
    })?;

    let start = SystemTime::now();
    let mut elapsed = Duration::default();
    // Load initial elapsed from today's entries for this category
    let today_start = utils::time::today()?;
    for res in db.latest_entries_range(today_start..) {
        let (_, entry) = res?;
        if entry.category.as_ref() == category.as_ref() {
            elapsed += Duration::from_secs(entry.end_time - entry.start_time);
        }
    }
    let max_secs = info
        .data(&category)
        .and_then(|d| d.goal)
        .map(|n| n.get())
        .unwrap_or(3600) as f64;

    let start_time = start.duration_since(std::time::UNIX_EPOCH)?.as_secs();
    let mut db_entry = Entry {
        category: category.clone(),
        start_time,
        end_time: start_time,
    };
    db.append_entry(&db_entry)?;

    let (lock, cvar) = &*pair;
    let mut stop = lock.lock().unwrap();
    while !*stop {
        let term_w = terminal_size().map(|(w, _)| w.0 as usize).unwrap_or(80);
        elapsed = elapsed.saturating_add(Duration::from_secs(1));
        let elapsed_secs = elapsed.as_secs_f64();
        let elapsed_str = if elapsed_secs < max_secs {
            format!(
                "{} ({} remaining)",
                format_duration(elapsed),
                format_duration(Duration::from_secs((max_secs - elapsed_secs) as _))
            )
        } else {
            format!("{}", format_duration(elapsed))
        };

        // Build content: category, padding, elapsed_str
        let mut content = Vec::with_capacity(term_w);
        content.extend(category.chars());
        let padding = term_w.saturating_sub(category.len() + elapsed_str.len());
        for _ in 0..padding {
            content.push(' ');
        }
        content.extend(elapsed_str.chars());

        // Print progress bar
        let filled =
            (((elapsed_secs / max_secs).fract() * term_w as f64) as usize).clamp(0, term_w);
        let mut line = String::with_capacity(term_w + 64);
        line.push('\r');
        let filled_style = anstyle::Style::new()
            .bg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::White)))
            .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Black)));
        let empty_style = anstyle::Style::new()
            .bg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::BrightBlack)));

        for (i, &ch) in content.iter().enumerate() {
            if i < filled {
                line.push_str(&format!("{}", filled_style.render()));
            } else {
                line.push_str(&format!("{}", empty_style.render()));
            }
            line.push(ch);
        }
        line.push_str(&format!("{}", anstyle::Reset.render()));
        print!("{}", line);
        std::io::stdout().flush()?;

        stop = cvar.wait_timeout(stop, Duration::from_secs(1)).unwrap().0;

        if elapsed.as_secs() % 300 == 0 {
            let end_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs();
            db_entry.end_time = end_time;
            db.update_last_entry(&db_entry)?;
        }
    }
    drop(stop);

    // One last save
    let end_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    db_entry.end_time = end_time;
    db.update_last_entry(&db_entry)?;

    // Move past the progress bar line
    print!("\n");
    std::io::stdout().flush()?;

    Ok(())
}
