use std::path::PathBuf;

use anyhow::Result;

use crate::database::{Database, NormalDb};
use crate::utils::io::FileWithPath;
use crate::utils::time::DATETIME_FMT;

pub struct Args {
    pub dest: NormalDb,
    pub source_path: PathBuf,
}

pub fn merge_from(args: Args) -> Result<()> {
    let Args {
        mut dest,
        source_path,
    } = args;

    let mut options = std::fs::OpenOptions::new();
    options.read(true).write(true);
    let source_file = FileWithPath::open(source_path, options)?;
    let mut source = Database::new(source_file);

    let source_end = source
        .entries()
        .next_back()
        .transpose()?
        .map(|(_, e)| e.start_time_local())
        .transpose()?;

    let result = dest.merge(&mut source)?;

    let count = result.new_source_entries;

    let yellow = anstyle::Style::new()
        .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow)));

    let from_s = result
        .common_span_entry
        .as_ref()
        .map(|(_, e)| e.start_time_local().ok())
        .flatten()
        .map(|dt| dt.format(&*DATETIME_FMT).unwrap())
        .unwrap_or_else(|| "beginning".to_string());

    let to_s = source_end
        .map(|dt| dt.format(&*DATETIME_FMT).unwrap())
        .unwrap_or_else(|| "now".to_string());

    println!(
        "Merged {yellow}{count}{reset} entries from {yellow}{from_s}{reset} to {yellow}{to_s}{reset}",
        yellow = yellow.render(),
        reset = anstyle::Reset.render(),
    );

    Ok(())
}
