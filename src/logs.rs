use std::collections::HashMap;
use std::fs::File;
use std::sync::Arc;

use anyhow::Result;
use time::OffsetDateTime;

use crate::args::CategoryMatch;
use crate::database::Database;
use crate::{cli, heatmap};

pub struct Args {
    pub db: Database<File>,
    pub from: Option<OffsetDateTime>,
    pub to: Option<OffsetDateTime>,
    pub category_match: Option<CategoryMatch>,
    pub clean: bool,
}

pub fn show_logs(args: Args) -> Result<()> {
    let Args {
        mut db,
        from,
        to,
        category_match,
        clean,
    } = args;
    let from_ts = from.as_ref().map(|dt| dt.unix_timestamp() as u64);
    let to_ts = to.as_ref().map(|dt| dt.unix_timestamp() as u64);

    // Hourly format only when `from` is set and range ≤ 32 hours
    let use_hourly = from_ts.map_or(false, |f| {
        let end = to_ts.unwrap_or_else(|| OffsetDateTime::now_utc().unix_timestamp() as u64);
        end.saturating_sub(f) <= 32 * 3600
    });

    let mut category_durations: HashMap<Arc<str>, u64> = HashMap::new();
    let mut durations: HashMap<u64, u64> = HashMap::new();

    let mut head_span = None;
    let mut tail_span = None;
    for res in db.entries().rev() {
        let (span, entry) = res?;

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
        if let Some(ref cm) = category_match {
            if !cm.matches(&entry.category) {
                continue;
            }
        }

        let duration = entry.end_time - entry.start_time;
        *category_durations.entry(entry.category).or_insert(0) += duration;

        let bucket = if use_hourly {
            entry.start_time - (entry.start_time % 3600)
        } else {
            entry.start_time - (entry.start_time % 86400)
        };
        *durations.entry(bucket).or_insert(0) += duration;

        tail_span = tail_span.or(Some(span));
        head_span = Some(span);
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

    // Heatmap
    if !durations.is_empty() {
        let min_key = *durations.keys().min().unwrap();
        let max_key = *durations.keys().max().unwrap();
        let interval = if use_hourly { 3600u64 } else { 86400u64 };
        let n = ((max_key - min_key) / interval + 1) as usize;

        let mut buckets = Vec::with_capacity(n);
        for i in 0..n {
            let key = min_key + (i as u64) * interval;
            let secs = durations.get(&key).copied().unwrap_or(0);
            let intensity = (secs / 1800).min(10) as u8;
            buckets.push(intensity);
        }

        if use_hourly {
            heatmap::show_heatmap(heatmap::Args {
                buckets,
                rows: 1,
                cols: Some(n),
            });
        } else {
            let cols = 14;
            let rows = (n + cols - 1) / cols;
            heatmap::show_heatmap(heatmap::Args {
                buckets,
                rows,
                cols: Some(cols),
            });
        }
    }

    // Cleaning prompt
    if clean && tail_span.is_some() {
        if cli::confirm("Delete these entries?") {
            db.remove_span(head_span, tail_span)?;
        }
    }

    Ok(())
}
