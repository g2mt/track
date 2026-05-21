use std::sync::OnceLock;

use anyhow::Result;
use humantime::parse_rfc3339_weak;
use time::{Duration, Month, OffsetDateTime, Time, UtcOffset};

static LOCAL_OFFSET: OnceLock<UtcOffset> = OnceLock::new();

pub fn init_local_offset() {
    LOCAL_OFFSET
        .set(match UtcOffset::current_local_offset() {
            Ok(offset) => offset,
            Err(_) => {
                eprintln!("Cannot determine local timezone, falling back to UTC");
                UtcOffset::from_hms(0, 0, 0).unwrap()
            }
        })
        .unwrap();
}

pub fn to_local_offset(dt: OffsetDateTime) -> OffsetDateTime {
    dt.to_offset(*LOCAL_OFFSET.get().unwrap())
}

pub fn now_local() -> OffsetDateTime {
    OffsetDateTime::now_utc().to_offset(*LOCAL_OFFSET.get().unwrap())
}

pub fn parse_datetime(s: &str) -> Result<OffsetDateTime> {
    match s {
        "today" => Ok(now_local().replace_time(Time::MIDNIGHT)),
        "yesterday" => Ok(now_local()
            .replace_time(Time::MIDNIGHT)
            .saturating_sub(Duration::DAY)),
        "this-week" => Ok(now_local().replace_time(Time::MIDNIGHT)
            - Duration::days(now_local().weekday().number_from_monday() as i64 - 1)),
        "this-month" => Ok(now_local()
            .replace_day(1)
            .unwrap()
            .replace_time(Time::MIDNIGHT)),
        "this-year" => Ok(
            time::Date::from_calendar_date(now_local().year(), Month::January, 1)
                .unwrap()
                .with_time(Time::MIDNIGHT)
                .assume_offset(now_local().offset()),
        ),
        _ => parse_rfc3339_weak(s).map(Into::into).map_err(Into::into),
    }
}

pub fn today() -> OffsetDateTime {
    now_local().replace_time(Time::MIDNIGHT)
}

pub fn yesterday() -> OffsetDateTime {
    now_local().replace_time(Time::MIDNIGHT) - Duration::DAY
}

pub fn this_week() -> OffsetDateTime {
    let now = now_local();
    now.replace_time(Time::MIDNIGHT) - Duration::days(now.weekday().number_from_monday() as i64 - 1)
}

pub fn this_month() -> OffsetDateTime {
    now_local()
        .replace_day(1)
        .unwrap()
        .replace_time(Time::MIDNIGHT)
}

pub fn this_year() -> OffsetDateTime {
    let now = now_local();
    time::Date::from_calendar_date(now.year(), Month::January, 1)
        .unwrap()
        .with_time(Time::MIDNIGHT)
        .assume_offset(now.offset())
}

pub fn print_date_range_header(
    align: &crate::align::Align,
    from: Option<&OffsetDateTime>,
    to: Option<&OffsetDateTime>,
    terminal_width: u16,
) {
    let fmt = time::format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]")
        .expect("valid format description");
    let date_ansi =
        anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow)));
    let from_s = from
        .map(|dt| dt.format(&fmt).unwrap())
        .unwrap_or_else(|| "beginning".to_string());
    let to_s = to
        .map(|dt| dt.format(&fmt).unwrap())
        .unwrap_or_else(|| "now".to_string());
    align.print(
        &[
            crate::align::TextFragment::Ansi(&date_ansi),
            crate::align::TextFragment::Raw(&from_s),
            crate::align::TextFragment::Ansi(&anstyle::Reset),
            crate::align::TextFragment::Raw(" .. "),
            crate::align::TextFragment::Ansi(&date_ansi),
            crate::align::TextFragment::Raw(&to_s),
            crate::align::TextFragment::Ansi(&anstyle::Reset),
        ],
        terminal_width,
    );
    println!();
}
