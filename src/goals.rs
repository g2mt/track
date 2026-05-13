use std::fs::File;
use std::sync::Arc;

use anyhow::Result;

use crate::args::Args;
use crate::database::Database;

pub fn list_goals(mut db: Database<File>) -> Result<()> {
    let info = db.read_info()?.unwrap_or_default();
    for (category, secs) in info.goals() {
        let d = std::time::Duration::from_secs(*secs);
        println!("{} {}", category, humantime::format_duration(d));
    }
    Ok(())
}

pub fn set_daily_goal(args: &Args, category: Arc<str>, daily: &str) -> Result<()> {
    let duration = daily.parse::<humantime::Duration>()?;
    let mut db = args.open_database(true)?;
    let mut info = db.read_info()?.unwrap_or_default();
    info.goals_mut()
        .insert(category.clone(), duration.as_secs());
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
