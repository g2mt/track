use std::io::IsTerminal;

use anyhow::Result;
use clap::{CommandFactory, Parser};
use clap_complete::generate;
use time::OffsetDateTime;

mod args;
use args::Args;
#[cfg(debug_assertions)]
use args::DebugHeatmap;

mod align;
mod chart;
mod database;
use database::CategoryData;
mod heatmap;
mod logs;
mod manip;
mod notify;
mod track;
mod utils;

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

    // Notify

    if args.notify {
        return notify::run_daemon(notify::Args {
            db: args.open_database(true)?.try_into()?,
            notifier: &args.notifier,
            notify_again: args.notify_again.clone(),
        });
    }

    // Info listing

    if args.categories {
        let mut db = args.open_database(false)?;
        let info = db.read_info()?.unwrap_or_default();
        for cat in info.keys() {
            println!("{}", cat);
        }
        return Ok(());
    }

    if args.goals {
        return manip::list(manip::Args {
            db: args.open_database(false)?,
            align: args.align.clone(),
            printer: |data: &CategoryData| {
                data.goal
                    .map(|g| {
                        let d = std::time::Duration::from_secs(g.get());
                        humantime::format_duration(d).to_string()
                    })
                    .unwrap_or_default()
            },
        });
    }

    if args.frequencies {
        return manip::list(manip::Args {
            db: args.open_database(false)?,
            align: args.align.clone(),
            printer: |data: &CategoryData| {
                data.notify_every
                    .as_ref()
                    .map(|f| match f {
                        database::Frequency::Day => "daily".to_string(),
                        database::Frequency::Hour => "hourly".to_string(),
                        database::Frequency::DayOfWeek(wd) => {
                            format!("on {}", wd)
                        }
                        database::Frequency::DayOfMonth(d) => format!("day {} of every month", d),
                    })
                    .unwrap_or_default()
            },
        });
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

    // Debug chart

    #[cfg(debug_assertions)]
    if args.debug_chart {
        chart::show_debug_chart(5);
        return Ok(());
    }

    // Chart

    if args.chart {
        let log_from = if args.logs.today {
            Some(utils::time::today()?)
        } else if args.logs.yesterday {
            Some(utils::time::yesterday()?)
        } else if args.logs.this_week {
            Some(utils::time::this_week()?)
        } else if args.logs.this_month {
            Some(utils::time::this_month()?)
        } else if args.logs.this_year {
            Some(utils::time::this_year()?)
        } else {
            args.from
                .as_deref()
                .map(utils::time::parse_datetime)
                .transpose()?
        };
        let log_to = args
            .to
            .as_deref()
            .map(utils::time::parse_datetime)
            .transpose()?;
        return chart::show_chart(chart::Args {
            db: args.open_database(false)?,
            from: log_from,
            to: log_to,
            category_match: args.category_match()?,
        });
    }

    // Logging

    let mut log_from: Option<OffsetDateTime> = None;
    let mut log_to: Option<OffsetDateTime> = None;
    let show_logs = if args.logs.today {
        log_from = Some(utils::time::today()?);
        true
    } else if args.logs.yesterday {
        log_from = Some(utils::time::yesterday()?);
        true
    } else if args.logs.this_week {
        log_from = Some(utils::time::this_week()?);
        true
    } else if args.logs.this_month {
        log_from = Some(utils::time::this_month()?);
        true
    } else if args.logs.this_year {
        log_from = Some(utils::time::this_year()?);
        true
    } else if args.from.is_some() || args.to.is_some() {
        log_from = args
            .from
            .as_deref()
            .map(utils::time::parse_datetime)
            .transpose()?;
        log_to = args
            .to
            .as_deref()
            .map(utils::time::parse_datetime)
            .transpose()?;
        true
    } else {
        args.clean
    };
    if show_logs {
        return logs::show_logs(logs::Args {
            db: args.open_database(args.clean)?, // clean requires write permissions
            from: log_from,
            to: log_to,
            category_match: args.category_match()?,
            clean: args.clean,
            align: args.align.clone(),
        });
    }

    // Category manipulation

    let category = args
        .category
        .clone()
        .ok_or_else(|| anyhow::anyhow!("missing category for tracking"))?;

    if let Some(daily) = &args.goal {
        manip::set_daily_goal(
            args.open_database(true)?,
            category,
            daily,
            args.frequency.as_ref(),
        )
    } else if let Some(ref freq) = args.frequency {
        manip::set_frequency(args.open_database(true)?, category, freq.clone())
    } else if let Some(ref ty) = args.category_type {
        manip::set_type(args.open_database(true)?, category, ty.clone())
    } else if args.reset_notification {
        let cm = args.category_match()?.unwrap();
        let mut db = args.open_database(true)?;
        let mut info = db.read_info()?.unwrap_or_default();
        let mut found = false;
        let today_midnight = OffsetDateTime::now_local()?
            .replace_time(time::Time::MIDNIGHT)
            .unix_timestamp() as u64;
        for (cat, data) in info.iter_mut() {
            if cm.matches(cat) {
                data.next_notification = std::num::NonZeroU64::new(today_midnight);
                found = true;
                if std::io::stdout().is_terminal() {
                    println!(
                        "Reset notification for {}{}{}",
                        anstyle::Style::new()
                            .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow))),
                        cat,
                        anstyle::Reset,
                    );
                }
            }
        }
        if !found {
            return Err(anyhow::anyhow!("Category not found: {}", category));
        }
        db.write_info(&info)?;
        Ok(())
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
        track::track(db.try_into()?, category)
    }
}
