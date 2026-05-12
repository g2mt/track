use anyhow::Result;
use clap::Parser;
mod cli;
mod logs;
mod track;
use cli::Cli;

fn main() -> Result<()> {
    let args = Cli::parse();

    if args.logs.today {
        todo!("print today's logs");
    } else if args.logs.yesterday {
        todo!("print yesterday's logs");
    } else if args.logs.this_week {
        todo!("print this week's logs");
    } else if args.logs.this_month {
        todo!("print this month's logs");
    } else if args.logs.this_year {
        todo!("print this year's logs");
    } else if args.from.is_some() || args.to.is_some() {
        logs::show_logs(
            args.from
                .map(|s| humantime::parse_rfc3339_weak(&s))
                .transpose()?
                .map(Into::into),
            args.to
                .map(|s| humantime::parse_rfc3339_weak(&s))
                .transpose()?
                .map(Into::into),
        );
    } else if let Some(daily) = args.daily {
        todo!("set daily goal to {}", daily);
    } else if let Some(category) = args.category {
        track::track(category);
    }

    Ok(())
}
