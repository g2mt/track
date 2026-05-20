use anyhow::Result;
use humantime::parse_rfc3339_weak;
use time::{Duration, Month, OffsetDateTime, Time};

pub fn parse_datetime(s: &str) -> Result<OffsetDateTime> {
    match s {
        "today" => Ok(OffsetDateTime::now_local()?.replace_time(Time::MIDNIGHT)),
        "yesterday" => Ok(OffsetDateTime::now_local()?
            .replace_time(Time::MIDNIGHT)
            .saturating_sub(Duration::DAY)),
        "this-week" => {
            let now = OffsetDateTime::now_local()?;
            let week_start = now.replace_time(Time::MIDNIGHT)
                - Duration::days(now.weekday().number_from_monday() as i64 - 1);
            Ok(week_start)
        }
        "this-month" => {
            let now = OffsetDateTime::now_local()?;
            Ok(now.replace_day(1).unwrap().replace_time(Time::MIDNIGHT))
        }
        "this-year" => {
            let now = OffsetDateTime::now_local()?;
            Ok(
                time::Date::from_calendar_date(now.year(), Month::January, 1)
                    .unwrap()
                    .with_time(Time::MIDNIGHT)
                    .assume_offset(now.offset()),
            )
        }
        _ => parse_rfc3339_weak(s).map(Into::into).map_err(Into::into),
    }
}

pub fn today() -> Result<OffsetDateTime> {
    let now = OffsetDateTime::now_local()?;
    let start = now.replace_time(Time::MIDNIGHT);
    Ok(start)
}

pub fn yesterday() -> Result<OffsetDateTime> {
    let now = OffsetDateTime::now_local()?;
    let today_start = now.replace_time(Time::MIDNIGHT);
    Ok(today_start - Duration::DAY)
}

pub fn this_week() -> Result<OffsetDateTime> {
    let now = OffsetDateTime::now_local()?;
    let week_start = now.replace_time(Time::MIDNIGHT)
        - Duration::days(now.weekday().number_from_monday() as i64 - 1);
    Ok(week_start)
}

pub fn this_month() -> Result<OffsetDateTime> {
    let now = OffsetDateTime::now_local()?;
    let from = now.replace_day(1).unwrap().replace_time(Time::MIDNIGHT);
    Ok(from)
}

pub fn this_year() -> Result<OffsetDateTime> {
    let now = OffsetDateTime::now_local()?;
    let from = time::Date::from_calendar_date(now.year(), Month::January, 1)
        .unwrap()
        .with_time(Time::MIDNIGHT)
        .assume_offset(now.offset());
    Ok(from)
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
