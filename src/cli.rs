use clap::{Args, Parser};

#[derive(Parser, Debug)]
#[command(name = "track", version, about = "Simple time-tracking CLI utility")]
pub struct Cli {
    /// Set the tracking file path
    #[arg(short = 'f', long = "file")]
    pub file: Option<String>,

    /// Set daily goal for a project (e.g. "1h")
    #[arg(long = "daily")]
    pub daily: Option<String>,

    #[command(flatten)]
    pub logs: Logs,

    /// Project name
    pub project: Option<String>,
}

#[derive(Args, Debug)]
#[group(required = true, multiple = false)]
pub struct Logs {
    /// Show today's logs
    #[arg(long = "today")]
    pub today: bool,

    /// Show yesterday's logs
    #[arg(long = "yesterday")]
    pub yesterday: bool,

    /// Show this week's logs
    #[arg(long = "this-week")]
    pub this_week: bool,

    /// Show this month's logs
    #[arg(long = "this-month")]
    pub this_month: bool,

    /// Show this year's logs
    #[arg(long = "this-year")]
    pub this_year: bool,

    #[command(flatten)]
    pub range: LogsRange,
}

#[derive(Args, Debug)]
pub struct LogsRange {
    /// Start of time range for logs
    #[arg(long = "from")]
    pub from: Option<String>,

    /// End of time range for logs
    #[arg(long = "to")]
    pub to: Option<String>,
}
