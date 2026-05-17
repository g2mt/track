use std::fs::File;
use std::num::NonZeroU64;
use std::sync::Arc;

use anyhow::Result;

use crate::align::{Align, TextFragment};
use crate::database::{CategoryData, Database, Frequency};

pub struct Args {
    pub db: Database<File>,
    pub align: Align,
    pub printer: fn(&CategoryData) -> String,
}

pub fn list(args: Args) -> Result<()> {
    let Args {
        mut db,
        align,
        printer,
    } = args;
    let info = db.read_info()?.unwrap_or_default();
    let terminal_width = terminal_size::terminal_size()
        .map(|(w, _)| w.0)
        .unwrap_or(80);

    let bold = anstyle::Style::new().bold();
    let reset = anstyle::Reset;
    let blue_bold = anstyle::Style::new()
        .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Blue)))
        .bold();

    // Header
    align.print(
        &[
            TextFragment::Ansi(&blue_bold),
            TextFragment::Raw(&"Total: "),
            TextFragment::Ansi(&reset),
            TextFragment::Raw(&info.len()),
        ],
        terminal_width,
    );
    println!();

    for (category, data) in info.iter() {
        let value = printer(data);
        if value.is_empty() {
            continue;
        }
        align.print(
            &[
                TextFragment::Raw(&"  "),
                TextFragment::Ansi(&bold),
                TextFragment::Raw(category),
                TextFragment::Ansi(&reset),
                TextFragment::HalfDivisor(" : "),
                TextFragment::Raw(&value),
            ],
            terminal_width,
        );
    }

    Ok(())
}

pub fn set_daily_goal(
    mut db: Database<File>,
    category: Arc<str>,
    daily: &str,
    frequency: Option<&Frequency>,
) -> Result<()> {
    let duration = daily.parse::<humantime::Duration>()?;
    let mut info = db.read_info()?.unwrap_or_default();
    {
        let data = info.add_category(category.clone());
        data.goal = NonZeroU64::new(duration.as_secs());
        if let Some(freq) = frequency {
            data.notify_every = Some(freq.clone());
        }
    }
    let style =
        anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow)));
    println!(
        "Set daily goal for {}{}{} to {}{}{}",
        style,
        category,
        anstyle::Reset,
        style,
        duration,
        anstyle::Reset
    );
    db.write_info(&info)?;
    Ok(())
}

pub fn set_frequency(mut db: Database<File>, category: Arc<str>, freq: Frequency) -> Result<()> {
    let mut info = db.read_info()?.unwrap_or_default();
    {
        let data = info.add_category(category.clone());
        data.notify_every = Some(freq);
    }
    let style =
        anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow)));
    println!(
        "Set notification frequency for {}{}{}",
        style,
        category,
        anstyle::Reset
    );
    db.write_info(&info)?;
    Ok(())
}
