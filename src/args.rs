use std::fs::File;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use clap_complete::Shell;
use regex::Regex;

use crate::align::Align;
use crate::database::{Database, Frequency};

#[derive(Debug)]
pub enum CategoryMatch {
    Exact(String),
    Regex(Regex),
}

impl CategoryMatch {
    pub fn matches(&self, category: &str) -> bool {
        match self {
            CategoryMatch::Exact(pat) => category == pat.as_str(),
            CategoryMatch::Regex(re) => re.is_match(category),
        }
    }
}

#[derive(clap::Parser, Debug)]
#[command(name = "track", version, about = "Simple time-tracking CLI utility")]
pub struct Args {
    /// Set the tracking file path
    #[arg(short = 'f', long = "file")]
    pub file: Option<String>,

    /// Print all category names line by line then exit
    #[arg(long)]
    pub categories: bool,

    /// List all goals with human-readable durations
    #[arg(short = 'g', long)]
    pub goals: bool,

    /// Set daily goal for a project (e.g. "1h")
    #[arg(long)]
    pub daily: Option<String>,

    #[command(flatten)]
    pub logs: Logs,

    /// Start of time range for logs
    #[arg(long = "from", conflicts_with = "dated_log")]
    pub from: Option<String>,

    /// End of time range for logs
    #[arg(long = "to", conflicts_with = "dated_log")]
    pub to: Option<String>,

    /// Category name
    pub category: Option<Arc<str>>,

    /// Treat category as a regex pattern
    #[arg(short = 'r', long = "regex")]
    pub regex: bool,

    /// Clean mode (delete logs/records instead of showing)
    #[arg(long)]
    pub clean: bool,

    /// Remove category from info only (goals and category list)
    #[arg(long = "remove-category")]
    pub remove_category: bool,

    /// Generate shell completion scripts
    #[arg(long, value_enum)]
    pub completion: Option<Shell>,

    /// Show a debug heatmap (day: 24 columns, month: 7x5 grid)
    #[cfg(debug_assertions)]
    #[arg(long, value_enum)]
    pub debug_heatmap: Option<DebugHeatmap>,

    /// Run notification loop in the foreground
    #[arg(short = 'n', long = "notify")]
    pub notify: bool,

    /// Binary to use for desktop notifications
    #[arg(long, default_value = "notify-send")]
    pub notifier: String,

    /// Set notification frequency (day, hour, mon-sun, or 1-31)
    #[arg(long, value_parser = parse_frequency)]
    pub frequency: Option<Frequency>,
}

#[derive(clap::ValueEnum, Clone, Debug)]
#[cfg(debug_assertions)]
pub enum DebugHeatmap {
    /// 1 row, 24 columns (hours in a day)
    Day,
    /// 7 rows, 5 columns (weeks in a month)
    Month,
}

impl Args {
    fn default_track_file() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        PathBuf::from(home).join("Documents/track.jsonl")
    }

    pub fn category_match(&self) -> Result<Option<CategoryMatch>> {
        self.category
            .as_ref()
            .map(|cat| {
                if self.regex {
                    Ok(CategoryMatch::Regex(Regex::new(cat)?))
                } else {
                    Ok(CategoryMatch::Exact(cat.to_string()))
                }
            })
            .transpose()
    }

    pub fn open_database(&self, write: bool) -> Result<Database<File>> {
        let path = self
            .file
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(Self::default_track_file);
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(write)
            .create(write)
            .open(&path)?;
        file.try_lock()?;
        Ok(Database::new(file))
    }
}

fn parse_frequency(s: &str) -> std::result::Result<Frequency, String> {
    match s.to_lowercase().as_str() {
        "day" => Ok(Frequency::Day),
        "hour" => Ok(Frequency::Hour),
        "mon" => Ok(Frequency::DayOfWeek(time::Weekday::Monday)),
        "tue" => Ok(Frequency::DayOfWeek(time::Weekday::Tuesday)),
        "wed" => Ok(Frequency::DayOfWeek(time::Weekday::Wednesday)),
        "thu" => Ok(Frequency::DayOfWeek(time::Weekday::Thursday)),
        "fri" => Ok(Frequency::DayOfWeek(time::Weekday::Friday)),
        "sat" => Ok(Frequency::DayOfWeek(time::Weekday::Saturday)),
        "sun" => Ok(Frequency::DayOfWeek(time::Weekday::Sunday)),
        _ => {
            if let Ok(day) = s.parse::<u8>() {
                if (1..=31).contains(&day) {
                    return Ok(Frequency::DayOfMonth(day));
                }
            }
            Err(format!(
                "invalid frequency: '{s}'. expected: day, hour, mon-sun, or 1-31"
            ))
        }
    }
}

#[derive(clap::Args, Debug)]
#[group(id = "dated_log", required = false, multiple = false)]
pub struct Logs {
    /// Show today's logs
    #[arg(long)]
    pub today: bool,

    /// Show yesterday's logs
    #[arg(long)]
    pub yesterday: bool,

    /// Show this week's logs
    #[arg(long)]
    pub this_week: bool,

    /// Show this month's logs
    #[arg(long)]
    pub this_month: bool,

    /// Show this year's logs
    #[arg(long)]
    pub this_year: bool,

    /// Align category lines in log output
    #[arg(long = "log-align", default_value = "center")]
    pub log_align: Align,
}
