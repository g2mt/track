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
    #[arg(long, help_heading = "Metadata")]
    pub categories: bool,

    /// List all goals with human-readable durations
    #[arg(long, help_heading = "Metadata")]
    pub goals: bool,

    /// List all notification frequencies
    #[arg(long, help_heading = "Metadata")]
    pub frequencies: bool,

    /// Set daily goal for a project (e.g. "1h")
    #[arg(short = 'g', long)]
    pub goal: Option<String>,

    /// Align category lines in output
    #[arg(long, default_value = "center", help_heading = "Metadata")]
    pub align: Align,

    #[command(flatten)]
    pub logs: Logs,

    /// Start of time range for logs
    #[arg(long = "from", conflicts_with = "dated_log", help_heading = "Logs")]
    pub from: Option<String>,

    /// End of time range for logs
    #[arg(long = "to", conflicts_with = "dated_log", help_heading = "Logs")]
    pub to: Option<String>,

    /// Category name
    #[arg()]
    pub category: Option<Arc<str>>,

    /// Treat category as a regex pattern
    #[arg(short = 'r', long = "regex")]
    pub regex: bool,

    /// Clean mode (delete logs/records instead of showing)
    #[arg(long, help_heading = "Metadata")]
    pub clean: bool,

    /// Remove category from info only (goals and category list)
    #[arg(long = "remove-category", help_heading = "Metadata")]
    pub remove_category: bool,

    /// Generate shell completion scripts
    #[arg(long, value_enum)]
    pub completion: Option<Shell>,

    /// Show a debug heatmap (day: 24 columns, month: 7x5 grid)
    #[cfg(debug_assertions)]
    #[arg(long, value_enum)]
    pub debug_heatmap: Option<DebugHeatmap>,

    /// Run notification loop in the foreground
    #[arg(short = 'n', long = "notify", help_heading = "Notifications")]
    pub notify: bool,

    /// Binary to use for desktop notifications
    #[arg(long, default_value = "notify-send", help_heading = "Notifications")]
    pub notifier: String,

    /// Set notification frequency (day, hour, mon-sun, or 1-31)
    #[arg(long, value_parser = parse_frequency, help_heading = "Notifications")]
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
    #[arg(long, help_heading = "Logs")]
    pub today: bool,

    /// Show yesterday's logs
    #[arg(long, help_heading = "Logs")]
    pub yesterday: bool,

    /// Show this week's logs
    #[arg(long, help_heading = "Logs")]
    pub this_week: bool,

    /// Show this month's logs
    #[arg(long, help_heading = "Logs")]
    pub this_month: bool,

    /// Show this year's logs
    #[arg(long, help_heading = "Logs")]
    pub this_year: bool,
}
