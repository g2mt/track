use std::collections::{BTreeMap, HashMap};
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
        from: OffsetDateTime, // 0th minute of the starting time
    },
    /// Daily durations mapping the day offset to how much work is done on that day
    Daily {
        buckets: BTreeMap<DayOffset, u64>,
        from: OffsetDateTime, // midnight of the starting time, acts as reference day as well
    },
}

const HOURLY_CALENDAR_MAX: u64 = 32 * 3600;
const DAILY_COLS: usize = 14;

impl HeatmapDurations {
    fn new(from: Option<OffsetDateTime>, to: Option<OffsetDateTime>) -> Self {
        let to = to.unwrap_or_else(OffsetDateTime::now_utc);
        let use_hourly = from.as_ref().map_or(false, |f| {
            let diff = to - *f;
            diff.whole_seconds() as u64 <= HOURLY_CALENDAR_MAX
        });

        if use_hourly {
            let to = to.replace_time(time::Time::from_hms(to.hour(), 0, 0).unwrap());
            let from = if let Some(from) = from {
                from.replace_time(time::Time::from_hms(from.hour(), 0, 0).unwrap())
            } else {
                to - time::Duration::hours(24)
            };
            let n = (to - from).whole_hours();
            Self::Hourly {
                buckets: vec![0; n.try_into().unwrap()],
                from,
            }
        } else {
            let from = from.unwrap_or_else(|| to - time::Duration::days(14));
            Self::Daily {
                buckets: BTreeMap::new(),
                from,
            }
        }
    }

    fn add_entry(&mut self, timestamp: OffsetDateTime, duration: u64) {
        match self {
            Self::Hourly { buckets, from, .. } => {
                let idx: usize = (timestamp - *from).whole_hours().try_into().unwrap();
                buckets[idx] += duration;
            }
            Self::Daily { buckets, from, .. } => {
                let day = timestamp.replace_time(time::Time::MIDNIGHT);
                let offset = (day - *from).whole_days();
                *buckets.entry(DayOffset(offset as u64)).or_insert(0) += duration;
            }
        }
    }

    fn show(&self) {
        match self {
            Self::Hourly { buckets, from } => {
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

                let cols = if from.hour() == 0 && from.minute() == 0 && from.second() == 0 {
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
            Self::Daily { buckets, .. } => {
                if buckets.is_empty() {
                    return;
                }

                let n_days = (buckets.last_key_value().unwrap().0).0 + 1;
                let max_duration: u64 = buckets.values().sum();

                let rows = (n_days as usize).div_ceil(DAILY_COLS);
                let mut intensity_buckets = vec![0; n_days as usize];
                for (offset, duration) in buckets.iter() {
                    let intensity =
                        (((*duration as f64) / (max_duration as f64)) * 10.0).round() as u8;
                    intensity_buckets[offset.0 as usize] = intensity;
                }

                heatmap::show_heatmap(heatmap::Args {
                    buckets: intensity_buckets,
                    rows,
                    cols: DAILY_COLS,
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
    let mut heatmap_durations = HeatmapDurations::new(from.clone(), to.clone());

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

        let ts = OffsetDateTime::from_unix_timestamp(entry.start_time as i64)?;
        heatmap_durations.add_entry(ts, duration);

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
