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

/// Number of days since the starting day in *from*
#[derive(Hash, Eq, PartialEq, Ord, PartialOrd, Clone, Copy)]
struct DayOffset(u64);

enum HeatmapDurations {
    /// Hourly durations with how much work is done per hour
    Hourly {
        buckets: Vec<u64>,
        range_start: u64,
        from: Option<OffsetDateTime>,
    },
    /// Daily durations mapping the day offset to how much work is done on that day
    Monthly {
        buckets: HashMap<DayOffset, u64>,
        from_ts: Option<u64>,
        from: Option<OffsetDateTime>,
        to: OffsetDateTime,
    },
}

const HOURLY_INTERVAL: u64 = 3600;
const HOURLY_CALENDAR_MAX: u64 = 32*3600;
const DAILY_INTERVAL: u64 = 86400;

impl HeatmapDurations {
    fn new(from: Option<OffsetDateTime>, to: OffsetDateTime) -> Self {
        let use_hourly = from.as_ref().map_or(false, |f| {
            let diff = to - *f;
            diff.whole_seconds() as u64 <= HOURLY_CALENDAR_MAX
        });

        if use_hourly {
            let from_ts = from.as_ref().map(|dt| dt.unix_timestamp() as u64);
            let to_ts = to.unix_timestamp() as u64;
            let range_start = from_ts.map(|f| f - (f % HOURLY_INTERVAL)).unwrap_or(0);
            let range_end = to_ts - (to_ts % HOURLY_INTERVAL);
            let n = ((range_end - range_start) / HOURLY_INTERVAL+ 1) as usize;
            Self::Hourly {
                buckets: vec![0; n],
                range_start,
                from,
            }
        } else {
            let from_ts = from
                .as_ref()
                .map(|dt| dt.unix_timestamp() as u64 - (dt.unix_timestamp() as u64 % DAILY_INTERVAL));
            Self::Monthly {
                buckets: HashMap::new(),
                from_ts,
                from,
                to,
            }
        }
    }

    fn add_entry(&mut self, timestamp: u64, duration: u64) {
        match self {
            Self::Hourly { buckets, range_start, .. } => {
                let idx = ((timestamp.saturating_sub(*range_start)) / HOURLY_INTERVAL) as usize;
                    buckets[idx] += duration;
            }
            Self::Monthly { buckets, from_ts, .. } => {
                let day = timestamp - (timestamp % DAILY_INTERVAL);
                let ref_day = from_ts.get_or_insert(day);
                let offset = (day - *ref_day) / DAILY_INTERVAL;
                *buckets.entry(DayOffset(offset as u64)).or_insert(0) += duration;
            }
        }
    }

    fn show(&self) {
        match self {
            Self::Hourly {
                buckets,
                from,
                ..
            } => {
                if buckets.is_empty() {
                    return;
                }
                let n = buckets.len();
                let max_secs = *buckets.iter().max().unwrap_or(&1);

                let intensity_buckets: Vec<u8> = buckets
                    .iter()
                    .map(|&secs| {
                        if max_secs > 0 {
                            ((secs as f64 / max_secs as f64) * 10.0).round() as u8
                        } else {
                            0
                        }
                    })
                    .collect();

                let cols = if let Some(from) = from
                    && from.hour() == 0
                    && from.minute() == 0
                    && from.second() == 0
                {
                    n.max(24)
                } else {
                    n
                };

                heatmap::show_heatmap(heatmap::Args {
                    buckets: intensity_buckets,
                    rows: 1,
                    cols,
                });
            }
            Self::Monthly {
                buckets,
                from_ts,
                from,
                to,
            } => {
                if buckets.is_empty() {
                    return;
                }

                let ref_day = from_ts.unwrap_or(0);
                let min_offset = buckets.keys().min().copied().unwrap_or(DayOffset(0));
                let max_offset = buckets.keys().max().copied().unwrap_or(DayOffset(0));
                let min_day = ref_day + min_offset.0 * DAILY_INTERVAL;
                let max_day = ref_day + max_offset.0 * DAILY_INTERVAL;

                let range_start = from
                    .as_ref()
                    .map(|dt| dt.unix_timestamp() as u64 - (dt.unix_timestamp() as u64 % DAILY_INTERVAL))
                    .unwrap_or(min_day);
                let range_end = to.unix_timestamp() as u64 - (to.unix_timestamp() as u64 % DAILY_INTERVAL);
                let n = ((range_end - range_start) / DAILY_INTERVAL + 1) as usize;

                let max_secs = *buckets.values().max().unwrap_or(&1);

                let intensity_buckets: Vec<u8> = (0..n)
                    .map(|i| {
                        let day = range_start + (i as u64) * DAILY_INTERVAL;
                        let offset = (day - ref_day) / DAILY_INTERVAL;
                        let secs = buckets
                            .get(&DayOffset(offset as u64))
                            .copied()
                            .unwrap_or(0);
                        if max_secs > 0 {
                            ((secs as f64 / max_secs as f64) * 10.0).round() as u8
                        } else {
                            0
                        }
                    })
                    .collect();

                let cols = 14;
                let rows = (n + cols - 1) / cols;

                heatmap::show_heatmap(heatmap::Args {
                    buckets: intensity_buckets,
                    rows,
                    cols,
                });
            }
        }
    }
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

    let mut category_durations: HashMap<Arc<str>, u64> = HashMap::new();
    let mut heatmap_durations = HeatmapDurations::new(from.clone(), to.unwrap_or_else(OffsetDateTime::now_utc));

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

        heatmap_durations.add_entry(entry.start_time, duration);

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
    heatmap_durations.show();

    // Cleaning prompt
    if clean && tail_span.is_some() {
        if cli::confirm("Delete these entries?") {
            db.remove_span(head_span, tail_span)?;
        }
    }

    Ok(())
}
