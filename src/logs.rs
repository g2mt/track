use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use time::OffsetDateTime;

use crate::align::{Align, TextFragment};
use crate::args::CategoryMatch;
use crate::database::range::TimeRange;
use crate::database::{CategoryType, NormalDb};
use crate::heatmap::durations::HeatmapDurations;
use crate::utils::cli;

pub struct Args {
    pub db: NormalDb,
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

    let info = db.read_info()?.unwrap_or_default();

    let mut category_durations: HashMap<Arc<str>, u64> = HashMap::new();
    let mut category_counts: HashMap<Arc<str>, u64> = HashMap::new();
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

        let is_duration = info
            .data(&entry.category)
            .map(|d| matches!(d.r#type, CategoryType::Duration))
            .unwrap_or(true);

        let duration = entry.end_time - entry.start_time;
        if is_duration {
            *category_durations.entry(entry.category).or_insert(0) += duration;
        } else {
            *category_counts.entry(entry.category).or_insert(0) += 1;
        }

        let ts = OffsetDateTime::from_unix_timestamp(entry.start_time as i64)?;
        heatmap_durations.add_entry(ts, duration);

        tail_span = tail_span.or(Some(span));
        head_span = Some(span);
    }

    let mut duration_categories: Vec<(Arc<str>, u64)> = category_durations.into_iter().collect();
    duration_categories.sort_by(|a, b| b.1.cmp(&a.1));

    let mut oneshot_categories: Vec<(Arc<str>, u64)> = category_counts.into_iter().collect();
    oneshot_categories.sort_by(|a, b| b.1.cmp(&a.1));

    let total: u64 = duration_categories.iter().map(|(_, d)| d).sum();

    let terminal_width = terminal_size::terminal_size()
        .map(|(w, _)| w.0)
        .unwrap_or(80);

    // Header: yellow FROM .. TO
    crate::utils::time::print_date_range_header(&align, from.as_ref(), to.as_ref(), terminal_width);

    // Duration category lines, sorted by duration descending
    let date_ansi =
        anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow)));
    let bold = anstyle::Style::new().bold();
    let reset = anstyle::Reset;
    for (category, duration) in &duration_categories {
        let d = std::time::Duration::from_secs(*duration);
        let dur_str = humantime::format_duration(d).to_string();
        align.print(
            &[
                TextFragment::Raw(&"  "),
                TextFragment::Ansi(&bold),
                TextFragment::Raw(category),
                TextFragment::Ansi(&reset),
                TextFragment::HalfDivisor(" : "),
                TextFragment::Ansi(&date_ansi),
                TextFragment::Raw(&dur_str),
                TextFragment::Ansi(&reset),
            ],
            terminal_width,
        );
    }

    // Oneshot category lines, sorted by count descending
    for (category, count) in &oneshot_categories {
        align.print(
            &[
                TextFragment::Raw(&"  "),
                TextFragment::Ansi(&bold),
                TextFragment::Raw(category),
                TextFragment::Ansi(&reset),
                TextFragment::HalfDivisor(" : "),
                TextFragment::Ansi(&date_ansi),
                TextFragment::Raw(&count.to_string()),
                TextFragment::Ansi(&reset),
                TextFragment::Raw(" times"),
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
