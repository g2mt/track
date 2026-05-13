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
        let style =
            anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow)));
        eprintln!(
            "{style}Hint: use '{bin} --completion {shell} > ~/.config/{shell}/completions/{bin}.{shell}' to generate completions for current user{reset}",
            style = style.render(),
            bin = bin_name,
            reset = anstyle::Reset.render()
        );
        return Ok(());
    }

    // Info listing

    if args.categories {
        let mut db = args.open_database(false)?;
        let info = db.read_info()?.unwrap_or_default();
        for cat in info.categories() {
            println!("{}", cat);
        }
        return Ok(());
    }

    // Logging

    let mut log_args: Option<logs::LogArgs> = None;
    if args.logs.today {
        let from = time_utils::today()?;
        log_args = Some(logs::LogArgs {
            db: args.open_database(false)?,
            from: Some(from),
            to: None,
            clean: false,
        });
    } else if args.logs.yesterday {
        let from = time_utils::yesterday()?;
        log_args = Some(logs::LogArgs {
            db: args.open_database(false)?,
            from: Some(from),
            to: None,
            clean: false,
        });
    } else if args.logs.this_week {
        let from = time_utils::this_week()?;
        log_args = Some(logs::LogArgs {
            db: args.open_database(false)?,
            from: Some(from),
            to: None,
            clean: false,
        });
    } else if args.logs.this_month {
        let from = time_utils::this_month()?;
        log_args = Some(logs::LogArgs {
            db: args.open_database(false)?,
            from: Some(from),
            to: None,
            clean: false,
        });
    } else if args.logs.this_year {
        let from = time_utils::this_year()?;
        log_args = Some(logs::LogArgs {
            db: args.open_database(false)?,
            from: Some(from),
            to: None,
            clean: false,
        });
    } else if args.from.is_some() || args.to.is_some() {
        log_args = Some(logs::LogArgs {
            db: args.open_database(false)?,
            from: args
                .from
                .as_deref()
                .map(time_utils::parse_datetime)
                .transpose()?,
            to: args
                .to
                .as_deref()
                .map(time_utils::parse_datetime)
                .transpose()?,
            clean: false,
        });
    }
    if let Some(args) = log_args {
        return logs::show_logs(args);
    }

    // Category manipulation

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
