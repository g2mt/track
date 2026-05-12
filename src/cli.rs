use clap::{Args, Parser};

#[derive(Parser, Debug)]
#[command(name = "track", version, about = "Simple time-tracking CLI utility")]
pub struct Cli {
    /// Set the tracking file path
    #[arg(short = 'f', long = "file")]
    pub file: Option<String>,

    /// Set daily goal for a project (e.g. "1h")
    #[arg(long)]
    pub daily: Option<String>,

    #[command(flatten)]
    pub logs: Logs,

    /// Start of time range for logs
    #[arg(long = "from", conflicts_with = "dated_log")]
    pub from: Option<String>,

    /// End of time range for logs
    #[arg(long = "to", conflicts_with = "dated_log")]
    pub to: Option<String>,

    /// Project name
    pub category: Option<String>,
}

#[derive(Args, Debug)]
#[group(id = "dated_log", required = false, multiple = false)]
pub struct Logs {
    /// Show today's logs
    #[arg(long)]
    pub today: bool,

    /// Show yesterday's logs
    #[arg(long)]
    pub yesterday: bool,

    /// Show this week's logs
    #[arg(long)]
    pub this_week: bool,

    /// Show this month's logs
    #[arg(long)]
    pub this_month: bool,

    /// Show this year's logs
    #[arg(long)]
    pub this_year: bool,
}
