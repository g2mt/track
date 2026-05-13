use std::fs::{File, TryLockError};
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use clap::{Args, Parser, ValueEnum};
use clap_complete::Shell;

use crate::database::Database;

#[derive(Parser, Debug)]
#[command(name = "track", version, about = "Simple time-tracking CLI utility")]
pub struct Cli {
    /// Set the tracking file path
    #[arg(short = 'f', long = "file")]
    pub file: Option<String>,

    /// Print all category names line by line then exit
    #[arg(long)]
    pub categories: bool,

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

    /// Generate shell completion scripts
    #[arg(long, value_enum)]
    pub completion: Option<Shell>,
}

impl Cli {
    fn default_track_file() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        PathBuf::from(home).join("Documents/track.jsonl")
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

#[derive(Args, Debug)]
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
}
