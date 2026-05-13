use std::io::{stdout, Write};

use terminal_size::{Height, Width};

const SQUARE: char = '\u{25A0}';

pub mod debug;

pub struct Args {
    /// Day-by-day intensity values (0-10)
    pub buckets: Vec<u8>,
    /// Number of rows in the heatmap grid (e.g. 7 for days of the week)
    pub rows: usize,
    /// Number of columns (computed as ceil(buckets / rows) if not provided)
    pub cols: Option<usize>,
}

fn color_for_value(val: u8) -> anstyle::Style {
    let ansi = match val {
        0 => anstyle::AnsiColor::BrightBlack,
        1..=3 => anstyle::AnsiColor::Green,
        4..=6 => anstyle::AnsiColor::BrightGreen,
        7..=9 => anstyle::AnsiColor::Yellow,
        _ => anstyle::AnsiColor::Cyan,
    };
    anstyle::Style::new().fg_color(Some(ansi.into()))
}

pub fn show_heatmap(args: Args) {
    if args.buckets.is_empty() || args.rows == 0 {
        return;
    }

    let (Width(term_width), _) = terminal_size::terminal_size().unwrap_or((Width(80), Height(24)));
    let term_width = term_width as usize;

    let cols = args.cols.unwrap_or_else(|| (args.buckets.len() + args.rows - 1) / args.rows);

    // Pad buckets to fill the grid
    let padded_len = cols * args.rows;
    let mut padded = args.buckets;
    padded.resize(padded_len, 0);

    println!(); // leading newline

    for row in 0..args.rows {
        let mut line = String::with_capacity(cols);
        for col in 0..cols {
            let val = padded[col * args.rows + row];
            let style = color_for_value(val);
            line.push_str(&format!("{}{}{}", style.render(), SQUARE, anstyle::Reset.render()));
        }
        let pad = term_width.saturating_sub(cols) / 2;
        println!("{:pad$}{}", "", line, pad = pad);
    }

    println!(); // trailing newline
    stdout().flush().unwrap();
}
