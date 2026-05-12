use anyhow::Result;
use clap::Parser;

mod cli;
use cli::Cli;

mod database;
mod io_utils;
mod logs;
mod time_utils;
mod track;

fn main() -> Result<()> {
    let args = Cli::parse();

    if args.logs.today {
        let (from, to) = time_utils::today()?;
        return logs::show_logs(Some(from), Some(to));
    } else if args.logs.yesterday {
        let (from, to) = time_utils::yesterday()?;
        return logs::show_logs(Some(from), Some(to));
    } else if args.logs.this_week {
        let (from, to) = time_utils::this_week()?;
        return logs::show_logs(Some(from), Some(to));
    } else if args.logs.this_month {
        let (from, to) = time_utils::this_month()?;
        return logs::show_logs(Some(from), Some(to));
    } else if args.logs.this_year {
        let (from, to) = time_utils::this_year()?;
        return logs::show_logs(Some(from), Some(to));
    } else if args.from.is_some() || args.to.is_some() {
        return logs::show_logs(
            args.from
                .as_deref()
                .map(time_utils::parse_datetime)
                .transpose()?,
            args.to
                .as_deref()
                .map(time_utils::parse_datetime)
                .transpose()?,
        );
    }

    let category = args
        .category
        .clone()
        .ok_or_else(|| anyhow::anyhow!("missing category for tracking"))?;
    if let Some(daily) = &args.daily {
        let daily = daily.parse::<humantime::Duration>()?;
        let mut db = args.open_database(true)?;
        let mut info = db.read_info()?.unwrap_or_default();
        info.goals_mut().insert(category.clone(), daily.as_secs());
        println!("Set daily goal for {} to {}", category, daily);
        db.write_info(&info)?;
        Ok(())
    } else {
        let db = args.open_database(true)?;
        track::track(db, category)
    }
}
