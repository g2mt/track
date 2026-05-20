use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use clap_complete::Shell;
use regex::Regex;

use crate::align::Align;
use crate::database::{CategoryType, Frequency, NormalDb};
use crate::utils::io::FileWithPath;

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

    /// Set daily goal for a project (e.g. "1h")
    #[arg(short = 'g', long)]
    pub goal: Option<String>,

    /// Treat category as a regex pattern
    #[arg(short = 'r', long = "regex")]
    pub regex: bool,

    /// Generate shell completion scripts
    #[arg(long, value_enum)]
    pub completion: Option<Shell>,

    /// Category name
    #[arg()]
    pub category: Option<Arc<str>>,

    /// Print all category names line by line then exit
    #[arg(long, help_heading = "Metadata")]
    pub categories: bool,

    /// List all goals with human-readable durations
    #[arg(long, help_heading = "Metadata")]
    pub goals: bool,

    /// List all notification frequencies
    #[arg(long, help_heading = "Metadata")]
    pub frequencies: bool,

    /// Align category lines in output
    #[arg(long, default_value = "center", help_heading = "Metadata")]
    pub align: Align,

    /// Clean mode (delete logs/records instead of showing)
    #[arg(long, help_heading = "Metadata")]
    pub clean: bool,

    /// Show a bar chart for category totals within a date range
    #[arg(long, help_heading = "Logs")]
    pub chart: bool,

    /// Set the category type (duration or oneshot)
    #[arg(long = "type", help_heading = "Metadata")]
    pub category_type: Option<CategoryType>,

    /// Remove category from info only (goals and category list)
    #[arg(long = "remove-category", help_heading = "Metadata")]
    pub remove_category: bool,

    #[command(flatten)]
    pub logs: Logs,

    /// Start of time range for logs
    #[arg(long = "from", conflicts_with = "dated_log", help_heading = "Logs")]
    pub from: Option<String>,

    /// End of time range for logs
    #[arg(long = "to", conflicts_with = "dated_log", help_heading = "Logs")]
    pub to: Option<String>,

    /// Run notification loop in the foreground
    #[arg(short = 'n', long = "notify", help_heading = "Notifications")]
    pub notify: bool,

    /// Binary to use for desktop notifications
    #[arg(long, default_value = "notify-send", help_heading = "Notifications")]
    pub notifier: String,

    /// Set notification frequency (day, hour, mon-sun, or 1-31)
    #[arg(long, help_heading = "Notifications")]
    pub frequency: Option<Frequency>,

    /// Reset the next notification time for a category
    #[arg(long = "reset-notification", help_heading = "Notifications")]
    pub reset_notification: bool,

    /// How long to wait before retrying a notification (day, hour, mon-sun, or 1-31)
    #[arg(
        long = "notify-again",
        default_value = "hour",
        help_heading = "Notifications"
    )]
    pub notify_again: Frequency,

    /// Show a debug heatmap (day: 24 columns, month: 7x5 grid)
    #[cfg(debug_assertions)]
    #[arg(long, value_enum)]
    pub debug_heatmap: Option<DebugHeatmap>,

    /// Show a debug chart with fake data
    #[cfg(debug_assertions)]
    #[arg(long)]
    pub debug_chart: bool,
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

    pub fn open_database(&self, write: bool) -> Result<NormalDb> {
        let path = self
            .file
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(Self::default_track_file);

        if write && std::env::var_os("TRACK_DISABLE_BACKUP").is_none() && path.try_exists()? {
            let bak = path.with_extension("bak");
            std::fs::copy(&path, &bak)?;
        }

        let mut options = std::fs::OpenOptions::new();
        options.read(true).write(write).create(write);
        let db_file = FileWithPath::open(path, options)?;
        Ok(NormalDb::new(db_file))
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
