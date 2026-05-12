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

fn main() -> Result<()> {
    let args = Cli::parse();

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
    } else if let Some(daily) = &args.daily {
        let daily = daily.parse::<humantime::Duration>()?;
        let mut db = args.open_database(true)?;
        let mut info = db.read_info()?;
        println!("Set daily goal for to {}", daily);
        Ok(())
    } else if let Some(category) = &args.category {
        track::track(category)
    } else {
        Err(anyhow::anyhow!("missing category for tracking"))
    }
}
