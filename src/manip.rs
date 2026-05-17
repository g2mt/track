use std::fs::File;
use std::num::NonZeroU64;
use std::sync::Arc;

use anyhow::Result;

use crate::args::Args;
use crate::database::{Database, Frequency};

pub fn list_goals(mut db: Database<File>) -> Result<()> {
    let info = db.read_info()?.unwrap_or_default();
    for (category, data) in info.iter() {
        if let Some(goal) = data.goal {
            let d = std::time::Duration::from_secs(goal.get());
            println!("{} {}", category, humantime::format_duration(d));
        }
    }
    Ok(())
}

pub fn set_daily_goal(args: &Args, category: Arc<str>, daily: &str) -> Result<()> {
    let duration = daily.parse::<humantime::Duration>()?;
    let mut db = args.open_database(true)?;
    let mut info = db.read_info()?.unwrap_or_default();
    {
        let data = info.add_category(category.clone());
        data.goal = NonZeroU64::new(duration.as_secs());
        if let Some(ref freq) = args.frequency {
            data.notify_every = Some(freq.clone());
        }
    }
    let style =
        anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow)));
    println!(
        "Set daily goal for {}{}{} to {}{}{}",
        style,
        category,
        anstyle::Reset,
        style,
        duration,
        anstyle::Reset
    );
    db.write_info(&info)?;
    Ok(())
}

pub fn set_frequency(args: &Args, category: Arc<str>, freq: Frequency) -> Result<()> {
    let mut db = args.open_database(true)?;
    let mut info = db.read_info()?.unwrap_or_default();
    {
        let data = info.add_category(category.clone());
        data.notify_every = Some(freq);
    }
    let style =
        anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow)));
    println!(
        "Set notification frequency for {}{}{}",
        style,
        category,
        anstyle::Reset
    );
    db.write_info(&info)?;
    Ok(())
}
