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
    if let (Some(from_dt), Some(to_dt)) = (from, to) {
        let fmt = time::format_description::parse(
            "[year]-[month]-[day] [hour]:[minute]:[second]",
        )
        .expect("valid format description");
        println!(
            "\x1b[33m{} .. {}\x1b[0m\n",
            from_dt.format(&fmt).unwrap(),
            to_dt.format(&fmt).unwrap(),
        );
    }

    // Category lines, sorted by duration descending
    for (category, duration) in &categories {
        let d = std::time::Duration::from_secs(*duration);
        println!(
            "  \x1b[1m{}\x1b[0m \x1b[90m({})\x1b[0m",
            category,
            humantime::format_duration(d),
        );
    }

    // Total
    println!();
    let total_d = std::time::Duration::from_secs(total);
    println!(
        "\x1b[34mTotal time:\x1b[0m {}",
        humantime::format_duration(total_d),
    );

    Ok(())
}
