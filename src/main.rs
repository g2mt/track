use anyhow::Result;
use clap::{CommandFactory, Parser};
use clap_complete::generate;

mod cli;
use cli::Cli;

mod database;
mod io_utils;
mod logs;
mod time_utils;
mod track;

fn main() -> Result<()> {
    let args = Cli::parse();

    if let Some(shell) = args.completion {
        let mut cmd = Cli::command();
        let bin_name = cmd.get_name().to_string();
        generate(shell, &mut cmd, &bin_name, &mut std::io::stdout());
        let style = anstyle::Style::new()
            .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow)));
        eprintln!(
            "{style}Hint: use '{bin} --completion {shell} > ~/.config/{shell}/completions/{bin}.{shell}' to generate completions for current user{reset}",
            style = style.render(),
            bin = bin_name,
            reset = anstyle::Reset.render()
        );
        return Ok(());
    }

    if args.categories {
        let mut db = args.open_database(false)?;
        let info = db.read_info()?.unwrap_or_default();
        for cat in info.categories() {
            println!("{}", cat);
        }
        return Ok(());
    }

    if args.logs.today {
        let (from, to) = time_utils::today()?;
        let mut db = args.open_database(false)?;
        return logs::show_logs(&mut db, Some(from), Some(to));
    } else if args.logs.yesterday {
        let (from, to) = time_utils::yesterday()?;
        let mut db = args.open_database(false)?;
        return logs::show_logs(&mut db, Some(from), Some(to));
    } else if args.logs.this_week {
        let (from, to) = time_utils::this_week()?;
        let mut db = args.open_database(false)?;
        return logs::show_logs(&mut db, Some(from), Some(to));
    } else if args.logs.this_month {
        let (from, to) = time_utils::this_month()?;
        let mut db = args.open_database(false)?;
        return logs::show_logs(&mut db, Some(from), Some(to));
    } else if args.logs.this_year {
        let (from, to) = time_utils::this_year()?;
        let mut db = args.open_database(false)?;
        return logs::show_logs(&mut db, Some(from), Some(to));
    } else if args.from.is_some() || args.to.is_some() {
        let mut db = args.open_database(false)?;
        return logs::show_logs(
            &mut db,
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
