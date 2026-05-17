use std::collections::HashMap;
use std::fs::File;
use std::sync::Arc;

use anyhow::Result;
use time::OffsetDateTime;

use crate::align::{Align, TextFragment};
use crate::args::CategoryMatch;
use crate::database::Database;
use crate::heatmap::durations::HeatmapDurations;
use crate::utils::cli;

pub struct Args {
    pub db: Database<File>,
    pub from: Option<OffsetDateTime>,
    pub to: Option<OffsetDateTime>,
    pub category_match: Option<CategoryMatch>,
    pub clean: bool,
    pub align: Align,
}

struct TimeRange {
    from: Option<OffsetDateTime>,
    to: Option<OffsetDateTime>,
}

impl std::ops::RangeBounds<OffsetDateTime> for TimeRange {
    fn start_bound(&self) -> std::ops::Bound<&OffsetDateTime> {
        match &self.from {
            Some(dt) => std::ops::Bound::Included(dt),
            None => std::ops::Bound::Unbounded,
        }
    }
    fn end_bound(&self) -> std::ops::Bound<&OffsetDateTime> {
        match &self.to {
            Some(dt) => std::ops::Bound::Included(dt),
            None => std::ops::Bound::Unbounded,
        }
    }
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

    let mut category_durations: HashMap<Arc<str>, u64> = HashMap::new();
    let mut heatmap_durations = HeatmapDurations::new(from.clone(), to.clone());

    let mut head_span = None;
    let mut tail_span = None;
    for res in db.latest_entries_range(TimeRange {
        from: from.clone(),
        to: to.clone(),
    }) {
        let (span, entry) = res?;

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

    let terminal_width = terminal_size::terminal_size()
        .map(|(w, _)| w.0)
        .unwrap_or(80);

    // Header: yellow FROM .. TO
    let fmt = time::format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]")
        .expect("valid format description");
    let date_ansi =
        anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow)));
    let from_s = from
        .as_ref()
        .map(|dt| dt.format(&fmt).unwrap())
        .unwrap_or_else(|| "beginning".to_string());
    let to_s = to
        .as_ref()
        .map(|dt| dt.format(&fmt).unwrap())
        .unwrap_or_else(|| "now".to_string());
    align.print(
        &[
            TextFragment::Ansi(&date_ansi),
            TextFragment::Raw(&from_s),
            TextFragment::Ansi(&anstyle::Reset),
            TextFragment::Raw(&" .. "),
            TextFragment::Ansi(&date_ansi),
            TextFragment::Raw(&to_s),
            TextFragment::Ansi(&anstyle::Reset),
        ],
        terminal_width,
    );
    println!();

    // Category lines, sorted by duration descending
    let dim =
        anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::BrightBlack)));
    let bold = anstyle::Style::new().bold();
    let reset = anstyle::Reset;
    for (category, duration) in &categories {
        let d = std::time::Duration::from_secs(*duration);
        let dur_str = humantime::format_duration(d).to_string();
        align.print(
            &[
                TextFragment::Raw(&"  "),
                TextFragment::Ansi(&bold),
                TextFragment::Raw(category),
                TextFragment::Ansi(&reset),
                TextFragment::HalfDivisor(" : "),
                TextFragment::Ansi(&dim),
                TextFragment::Raw(&dur_str),
                TextFragment::Ansi(&reset),
            ],
            terminal_width,
        );
    }

    // Total
    let total_d = std::time::Duration::from_secs(total);
    let total_str = humantime::format_duration(total_d).to_string();
    let blue_bold = anstyle::Style::new()
        .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Blue)))
        .bold();
    println!();
    align.print(
        &[
            TextFragment::Ansi(&blue_bold),
            TextFragment::Raw(&"Total time:"),
            TextFragment::Ansi(&anstyle::Reset),
            TextFragment::Raw(&format!(" {}", total_str)),
        ],
        terminal_width,
    );

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
