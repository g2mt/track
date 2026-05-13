use std::io::IsTerminal;

use anyhow::Result;
use clap::{CommandFactory, Parser};
use clap_complete::generate;

mod args;
use args::Args;
#[cfg(debug_assertions)]
use args::DebugHeatmap;
use time::OffsetDateTime;

mod cli;
mod database;
mod goals;
mod heatmap;
mod io_utils;
mod logs;
mod time_utils;
mod track;

fn main() -> Result<()> {
    let args = Args::parse();

    if let Some(shell) = args.completion {
        let mut cmd = Args::command();
        let bin_name = cmd.get_name().to_string();
        generate(shell, &mut cmd, &bin_name, &mut std::io::stdout());
        if std::io::stdout().is_terminal() {
            let style = anstyle::Style::new()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow)));
            eprintln!(
                "{style}Hint: use '{bin} --completion {shell} > ~/.config/{shell}/completions/{bin}.{shell}' to generate completions for current user{reset}",
                style = style.render(),
                bin = bin_name,
                reset = anstyle::Reset.render()
            );
        }
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

    if args.goals {
        return goals::list_goals(args.open_database(false)?);
    }

    // Debug heatmap

    #[cfg(debug_assertions)]
    if let Some(ref mode) = args.debug_heatmap {
        let (rows, cols) = match mode {
            DebugHeatmap::Day => (1, 24),
            DebugHeatmap::Month => (3, 14),
        };
        heatmap::debug::show_debug_heatmap(rows, cols);
        return Ok(());
    }

    // Logging

    let mut log_from: Option<OffsetDateTime> = None;
    let mut log_to: Option<OffsetDateTime> = None;
    if args.logs.today {
        log_from = Some(time_utils::today()?);
    } else if args.logs.yesterday {
        log_from = Some(time_utils::yesterday()?);
    } else if args.logs.this_week {
        log_from = Some(time_utils::this_week()?);
    } else if args.logs.this_month {
        log_from = Some(time_utils::this_month()?);
    } else if args.logs.this_year {
        log_from = Some(time_utils::this_year()?);
    } else if args.from.is_some() || args.to.is_some() {
        log_from = args
            .from
            .as_deref()
            .map(time_utils::parse_datetime)
            .transpose()?;
        log_to = args
            .to
            .as_deref()
            .map(time_utils::parse_datetime)
            .transpose()?;
    }
    if log_from.is_some() || log_to.is_some() {
        return logs::show_logs(logs::Args {
            db: args.open_database(args.clean)?, // clean requires write permissions
            from: log_from,
            to: log_to,
            category_match: args.category_match()?,
            clean: args.clean,
        });
    }

    // Category manipulation

    let category = args
        .category
        .clone()
        .ok_or_else(|| anyhow::anyhow!("missing category for tracking"))?;
    if let Some(daily) = &args.daily {
        return goals::set_daily_goal(&args, category, daily);
    } else if args.remove_category {
        let cm = args.category_match()?.unwrap();
        let mut db = args.open_database(true)?;
        let mut info = db.read_info()?.unwrap_or_default();
        let removed = info.remove_categories(&cm);
        if removed.is_empty() {
            return Err(anyhow::anyhow!("Category not found: {}", category));
        } else {
            if std::io::stdout().is_terminal() {
                println!(
                    "{}Removed categories from metadata:{}",
                    anstyle::Style::new()
                        .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Blue)))
                        .bold(),
                    anstyle::Reset,
                );
            }
            for cat in &removed {
                println!("{}", cat);
            }
        }
        db.write_info(&info)?;
        Ok(())
    } else {
        let db = args.open_database(true)?;
        track::track(db, category)
    }
}
