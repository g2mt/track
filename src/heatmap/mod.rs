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
    pub cols: usize,
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

    let (Width(term_width), _) = terminal_size::terminal_size().unwrap_or((Width(80), Height(24)));
    let term_width = term_width as usize;

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
