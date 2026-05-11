use clap::Parser;
mod cli;
use cli::Cli;

fn main() {
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
    } else if let Some(_daily) = args.daily {
        todo!("set daily goal");
    } else if let Some(_from) = args.from {
        todo!("print logs from time range");
    } else if let Some(_project) = args.project {
        todo!("start tracking project");
    }
}
