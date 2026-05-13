use std::io::{Write, stdout};

use terminal_size::Width;

const SQUARE: char = '\u{25A0}';

pub mod debug;
pub mod durations;

pub struct Args {
    /// Day-by-day intensity values (0-10)
    pub buckets: Vec<u8>,
    /// Number of rows in the heatmap grid (e.g. 7 for days of the week)
    pub rows: usize,
    /// Number of columns (computed as ceil(buckets / rows) if not provided)
    pub cols: usize,
    /// Terminal width to use for centering (None = auto-detect)
    pub terminal_width: Option<u16>,
}

fn color_for_value(val: Option<u8>) -> anstyle::Style {
    let ansi = if let Some(val) = val {
        match val {
            0 => anstyle::AnsiColor::BrightBlack,
            1..=3 => anstyle::AnsiColor::Green,
            4..=6 => anstyle::AnsiColor::BrightGreen,
            7..=9 => anstyle::AnsiColor::Yellow,
            _ => anstyle::AnsiColor::Cyan,
        }
    } else {
        anstyle::AnsiColor::Black
    };
    anstyle::Style::new().fg_color(Some(ansi.into()))
}

pub fn show_heatmap(args: Args) {
    if args.buckets.is_empty() || args.rows == 0 {
        return;
    }

    let term_width = args.terminal_width.map(|w| w as usize).unwrap_or_else(|| {
        terminal_size::terminal_size()
            .map(|(Width(w), _)| w as usize)
            .unwrap_or(80)
    });

    let cols = args.cols;

    println!(); // leading newline

    for row in 0..args.rows {
        let mut line = String::with_capacity(cols);
        for col in 0..cols {
            let val = args.buckets.get(row * args.cols + col).copied();
            let style = color_for_value(val);
            line.push_str(&format!(
                "{}{}{}",
                style.render(),
                SQUARE,
                anstyle::Reset.render()
            ));
        }
        let pad = term_width.saturating_sub(cols) / 2;
        println!("{:pad$}{}", "", line, pad = pad);
    }

    println!(); // trailing newline
    stdout().flush().unwrap();
}
