use std::collections::HashMap;
use std::fs::File;
use std::sync::Arc;

use anyhow::Result;
use time::OffsetDateTime;

use crate::database::Database;

pub fn show_logs(
    db: &mut Database<File>,
    from: Option<OffsetDateTime>,
    to: Option<OffsetDateTime>,
) -> Result<()> {
    let from_ts = from.as_ref().map(|dt| dt.unix_timestamp() as u64);
    let to_ts = to.as_ref().map(|dt| dt.unix_timestamp() as u64);

    let mut category_durations: HashMap<Arc<str>, u64> = HashMap::new();

    for entry in db.entries().rev() {
        let entry = entry?;

        if let Some(from) = from_ts {
            if entry.start_time < from {
                break;
            }
        }
        if let Some(to) = to_ts {
            if entry.start_time > to {
                continue;
            }
        }

        let duration = entry.end_time - entry.start_time;
        *category_durations.entry(entry.category).or_insert(0) += duration;
    }

    let mut categories: Vec<(Arc<str>, u64)> = category_durations.into_iter().collect();
    categories.sort_by(|a, b| b.1.cmp(&a.1));

    let total: u64 = categories.iter().map(|(_, d)| d).sum();

    // Header: yellow FROM .. TO
    let fmt = time::format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]")
        .expect("valid format description");
    let date_ansi =
        anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow)));
    let from_str = from
        .as_ref()
        .map(|dt| dt.format(&fmt).unwrap())
        .unwrap_or_else(|| "beginning".to_string());
    let to_str = to
        .as_ref()
        .map(|dt| dt.format(&fmt).unwrap())
        .unwrap_or_else(|| "now".to_string());
    println!(
        "{}{}{} .. {}{}{}\n",
        date_ansi,
        from_str,
        anstyle::Reset,
        date_ansi,
        to_str,
        anstyle::Reset,
    );

    // Category lines, sorted by duration descending
    for (category, duration) in &categories {
        let d = std::time::Duration::from_secs(*duration);
        println!(
            "  {}{}{} {}{}{}",
            anstyle::Style::new().bold(),
            category,
            anstyle::Reset,
            anstyle::Style::new()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::BrightBlack))),
            humantime::format_duration(d),
            anstyle::Reset,
        );
    }

    // Total
    println!();
    let total_d = std::time::Duration::from_secs(total);
    println!(
        "{}Total time:{} {}",
        anstyle::Style::new()
            .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Blue)))
            .bold(),
        anstyle::Reset,
        humantime::format_duration(total_d),
    );

    Ok(())
}
