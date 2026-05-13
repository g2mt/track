use std::collections::HashMap;
use std::fs::File;
use std::sync::Arc;

use anyhow::Result;
use time::OffsetDateTime;

use crate::args::{Align, CategoryMatch};
use crate::cli;
use crate::database::Database;
use crate::heatmap::durations::HeatmapDurations;

pub struct Args {
    pub db: Database<File>,
    pub from: Option<OffsetDateTime>,
    pub to: Option<OffsetDateTime>,
    pub category_match: Option<CategoryMatch>,
    pub clean: bool,
    pub align: Align,
}

pub fn show_logs(args: Args) -> Result<()> {
    let Args {
        mut db,
        from,
        to,
        category_match,
        clean,
        align,
    } = args;
    let from_ts = from.as_ref().map(|dt| dt.unix_timestamp() as u64);
    let to_ts = to.as_ref().map(|dt| dt.unix_timestamp() as u64);

    let mut category_durations: HashMap<Arc<str>, u64> = HashMap::new();
    let mut heatmap_durations = HeatmapDurations::new(from.clone(), to.clone());

    let mut head_span = None;
    let mut tail_span = None;
    for res in db.entries().rev() {
        let (span, entry) = res?;

        if let Some(from) = from_ts {
            if entry.start_time < from {
                break;
            }
        }
        if let Some(to) = to_ts {
            if entry.start_time > to {
                continue;
            }
        }
        if let Some(ref cm) = category_match {
            if !cm.matches(&entry.category) {
                continue;
            }
        }

        let duration = entry.end_time - entry.start_time;
        *category_durations.entry(entry.category).or_insert(0) += duration;

        let ts = OffsetDateTime::from_unix_timestamp(entry.start_time as i64)?;
        heatmap_durations.add_entry(ts, duration);

        tail_span = tail_span.or(Some(span));
        head_span = Some(span);
    }

    let mut categories: Vec<(Arc<str>, u64)> = category_durations.into_iter().collect();
    categories.sort_by(|a, b| b.1.cmp(&a.1));

    let total: u64 = categories.iter().map(|(_, d)| d).sum();

    let terminal_width = terminal_size::terminal_size().map(|(w, _)| w.0).unwrap_or(80);

    // Header: yellow FROM .. TO
    let fmt = time::format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]")
        .expect("valid format description");
    let date_ansi =
        anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow)));
    let from_str = from
        .as_ref()
        .map(|dt| dt.format(&fmt).unwrap())
        .unwrap_or_else(|| "beginning".to_string());
    let to_str = to
        .as_ref()
        .map(|dt| dt.format(&fmt).unwrap())
        .unwrap_or_else(|| "now".to_string());
    let header = format!("{} .. {}", from_str, to_str);
    match align {
        Align::Left => {
            println!(
                "{date_ansi}{}{reset} .. {date_ansi}{}{reset}\n",
                from_str,
                to_str,
                date_ansi = date_ansi,
                reset = anstyle::Reset,
            );
        }
        Align::Center => {
            let padding = (terminal_width as usize).saturating_sub(header.len()) / 2;
            print!("{:padding$}", "", padding = padding);
            println!(
                "{date_ansi}{}{reset} .. {date_ansi}{}{reset}\n",
                from_str,
                to_str,
                date_ansi = date_ansi,
                reset = anstyle::Reset,
            );
        }
    }

    // Category lines, sorted by duration descending
    let dim = anstyle::Style::new()
        .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::BrightBlack)));
    let bold = anstyle::Style::new().bold();
    let reset = anstyle::Reset;
    for (category, duration) in &categories {
        let d = std::time::Duration::from_secs(*duration);
        let dur_str = humantime::format_duration(d).to_string();
        match align {
            Align::Left => {
                println!(
                    "  {bold}{category}{reset} {dim}{dur_str}{reset}",
                    bold = bold,
                    reset = reset,
                    dim = dim,
                );
            }
            Align::Center => {
                let colon_pos = category.len() + 1;
                let padding = (terminal_width as usize / 2).saturating_sub(colon_pos);
                print!("{:padding$}", "", padding = padding);
                println!(
                    "{bold}{category}{reset} : {dim}{dur_str}{reset}",
                    bold = bold,
                    reset = reset,
                    dim = dim,
                );
            }
        }
    }

    // Total
    println!();
    let total_d = std::time::Duration::from_secs(total);
    let total_str = humantime::format_duration(total_d).to_string();
    let total_line = format!("Total time: {}", total_str);
    let blue_bold = anstyle::Style::new()
        .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Blue)))
        .bold();
    match align {
        Align::Left => {
            println!(
                "{blue_bold}Total time:{reset} {total_str}",
                blue_bold = blue_bold,
                reset = anstyle::Reset,
            );
        }
        Align::Center => {
            let padding = (terminal_width as usize).saturating_sub(total_line.len()) / 2;
            print!("{:padding$}", "", padding = padding);
            println!(
                "{blue_bold}Total time:{reset} {total_str}",
                blue_bold = blue_bold,
                reset = anstyle::Reset,
            );
        }
    }

    // Heatmap
    heatmap_durations.show(Some(terminal_width));

    // Cleaning prompt
    if clean && tail_span.is_some() {
        if cli::confirm("Delete these entries?") {
            db.remove_span(head_span, tail_span)?;
        }
    }

    Ok(())
}
