use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

mod cli;
use cli::Cli;

use crate::database::Database;

mod database;
mod logs;
mod time_utils;
mod track;

fn default_track_file() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join("Documents/track.jsonl")
}

fn main() -> Result<()> {
    let args = Cli::parse();
    let path = args
        .file
        .map(PathBuf::from)
        .unwrap_or_else(default_track_file);
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&path)?;
    let _database = Database::new(file);

    if args.logs.today {
        let (from, to) = time_utils::today()?;
        logs::show_logs(Some(from), Some(to))
    } else if args.logs.yesterday {
        let (from, to) = time_utils::yesterday()?;
        logs::show_logs(Some(from), Some(to))
    } else if args.logs.this_week {
        let (from, to) = time_utils::this_week()?;
        logs::show_logs(Some(from), Some(to))
    } else if args.logs.this_month {
        let (from, to) = time_utils::this_month()?;
        logs::show_logs(Some(from), Some(to))
    } else if args.logs.this_year {
        let (from, to) = time_utils::this_year()?;
        logs::show_logs(Some(from), Some(to))
    } else if args.from.is_some() || args.to.is_some() {
        logs::show_logs(
            args.from
                .as_deref()
                .map(time_utils::parse_datetime)
                .transpose()?,
            args.to
                .as_deref()
                .map(time_utils::parse_datetime)
                .transpose()?,
        )
    } else if let Some(daily) = args.daily {
        todo!("set daily goal to {}", daily);
    } else if let Some(category) = args.category {
        track::track(category)
    } else {
        unreachable!()
    }
}
