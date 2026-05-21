use anyhow::Result;
use time::OffsetDateTime;

use crate::align::Align;
use crate::args::CategoryMatch;
use crate::database::range::TimeRange;
use crate::database::{CategoryType, NormalDb};

pub struct Args {
    pub db: NormalDb,
    pub from: Option<OffsetDateTime>,
    pub to: Option<OffsetDateTime>,
    pub category_match: Option<CategoryMatch>,
}

const SQUARE: char = '\u{2588}';
const HORIZ: char = '\u{2500}';
const TOP_LEFT: char = '\u{250C}';
const TOP_RIGHT: char = '\u{2510}';
const BOT_LEFT: char = '\u{2514}';
const BOT_RIGHT: char = '\u{2518}';
const VERT: char = '\u{2502}';

pub fn show_chart(args: Args) -> Result<()> {
    let Args {
        mut db,
        from,
        to,
        category_match,
    } = args;

    let info = db.read_info()?.unwrap_or_default();

    let concrete_to = to.unwrap_or_else(|| crate::utils::time::now_local());
    let concrete_from = from.unwrap_or_else(|| concrete_to - time::Duration::days(14));

    let (term_width, term_height) = terminal_size::terminal_size()
        .map(|(w, h)| (w.0 as usize, h.0 as usize))
        .unwrap_or((80, 24));

    let total_days = ((concrete_to - concrete_from).whole_days() + 1).max(1) as usize;
    let inner_width = if term_width >= 2 { term_width - 2 } else { 1 };
    let days_per_col = ((total_days + inner_width - 1) / inner_width).max(1);
    // Number of columns needed to cover all days; used to center the chart.
    let used_cols = (total_days + days_per_col - 1) / days_per_col;
    // Offset to center the bars horizontally when fewer columns are used.
    let col_offset = inner_width.saturating_sub(used_cols);

    let mut columns = vec![0u64; inner_width];
    for res in db.latest_entries_range(TimeRange {
        from: Some(concrete_from),
        to: Some(concrete_to),
    }) {
        let (_span, entry) = res?;

        if let Some(ref cm) = category_match {
            if !cm.matches(&entry.category) {
                continue;
            }
        }

        let is_duration = info
            .data(&entry.category)
            .map(|d| matches!(d.r#type, CategoryType::Duration))
            .unwrap_or(true);

        if !is_duration {
            continue;
        }

        let ts = entry.start_time_local()?;
        let day = ts.replace_time(time::Time::MIDNIGHT);
        let offset = (day - concrete_from).whole_days() as usize;
        // Shift column right by the centering offset so bars appear centered.
        let col = col_offset + offset / days_per_col;
        if col < inner_width {
            columns[col] += entry.elapsed_seconds();
        }
    }

    if columns.iter().all(|&c| c == 0) {
        return Ok(());
    }

    let chart_height = (term_height / 3).max(1);

    let max_col = *columns.iter().max().unwrap_or(&1);

    let align = Align::Center;
    crate::utils::time::print_date_range_header(
        &align,
        from.as_ref(),
        to.as_ref(),
        term_width as u16,
    );

    let top_border: String = std::iter::once(TOP_LEFT)
        .chain(std::iter::repeat(HORIZ).take(inner_width))
        .chain(std::iter::once(TOP_RIGHT))
        .collect();
    println!("{}", top_border);

    for row in (0..chart_height).rev() {
        let mut line = String::with_capacity(term_width);
        line.push(VERT);
        for secs in &columns {
            let bar_fill = if max_col > 0 {
                (*secs as usize * chart_height) / max_col as usize
            } else {
                0
            };
            line.push(if row < bar_fill { SQUARE } else { ' ' });
        }
        line.push(VERT);
        println!("{}", line);
    }

    let bot_border: String = std::iter::once(BOT_LEFT)
        .chain(std::iter::repeat(HORIZ).take(inner_width))
        .chain(std::iter::once(BOT_RIGHT))
        .collect();
    println!("{}", bot_border);

    Ok(())
}

#[cfg(debug_assertions)]
pub fn show_debug_chart(num_days: usize) {
    let (term_width, term_height) = terminal_size::terminal_size()
        .map(|(w, h)| (w.0 as usize, h.0 as usize))
        .unwrap_or((80, 24));

    let chart_height = (term_height / 3).max(1);
    let inner_width = term_width - 2;
    let days_per_col = (num_days + inner_width - 1) / inner_width;
    let mut columns = vec![0u64; inner_width];
    let mut rng = crate::utils::rand::Lcg::new(1337);
    for col in columns.iter_mut() {
        for _ in 0..days_per_col {
            *col += (rng.next_u64() % 8) + 1;
        }
    }

    let max_col = *columns.iter().max().unwrap_or(&1);

    let debug_header = "debug chart";
    let align = Align::Center;
    align.print(
        &[crate::align::TextFragment::Raw(debug_header)],
        term_width as u16,
    );
    println!();

    let top_border: String = std::iter::once(TOP_LEFT)
        .chain(std::iter::repeat(HORIZ).take(inner_width))
        .chain(std::iter::once(TOP_RIGHT))
        .collect();
    println!("{}", top_border);

    for row in (0..chart_height).rev() {
        let mut line = String::with_capacity(term_width);
        line.push(VERT);
        for secs in &columns {
            let bar_fill = if max_col > 0 {
                (*secs as usize * chart_height) / max_col as usize
            } else {
                0
            };
            line.push(if row < bar_fill { SQUARE } else { ' ' });
        }
        line.push(VERT);
        println!("{}", line);
    }

    let bot_border: String = std::iter::once(BOT_LEFT)
        .chain(std::iter::repeat(HORIZ).take(inner_width))
        .chain(std::iter::once(BOT_RIGHT))
        .collect();
    println!("{}", bot_border);
}
