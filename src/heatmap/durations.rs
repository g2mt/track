use std::collections::BTreeMap;

use time::OffsetDateTime;

use crate::heatmap::{Args, show_heatmap};

/// Number of days since the starting day in *from*
#[derive(Hash, Eq, PartialEq, Ord, PartialOrd, Clone, Copy)]
pub struct DayOffset(u64);

pub const HOURLY_CALENDAR_MAX: u64 = 32 * 3600;
pub const DAILY_COLS: usize = 28;

pub enum HeatmapDurations {
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

impl HeatmapDurations {
    pub fn new(from: Option<OffsetDateTime>, to: Option<OffsetDateTime>) -> Self {
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

    pub fn add_entry(&mut self, timestamp: OffsetDateTime, duration: u64) {
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

    pub fn show(&self, terminal_width: Option<u16>) {
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

                show_heatmap(Args {
                    buckets: intensity_buckets,
                    rows: 1,
                    cols,
                    terminal_width,
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

                show_heatmap(Args {
                    buckets: intensity_buckets,
                    rows,
                    cols: DAILY_COLS,
                    terminal_width,
                });
            }
        }
    }
}
