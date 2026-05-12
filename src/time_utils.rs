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
            Ok(time::Date::from_calendar_date(now.year(), Month::January, 1)
                .unwrap()
                .with_time(Time::MIDNIGHT)
                .assume_offset(now.offset()))
        }
        _ => parse_rfc3339_weak(s).map(Into::into).map_err(Into::into),
    }
}

pub fn today() -> Result<(OffsetDateTime, OffsetDateTime)> {
    let now = OffsetDateTime::now_local()?;
    let start = now.replace_time(Time::MIDNIGHT);
    Ok((start, start + Duration::DAY))
}

pub fn yesterday() -> Result<(OffsetDateTime, OffsetDateTime)> {
    let now = OffsetDateTime::now_local()?;
    let today_start = now.replace_time(Time::MIDNIGHT);
    Ok((today_start - Duration::DAY, today_start))
}

pub fn this_week() -> Result<(OffsetDateTime, OffsetDateTime)> {
    let now = OffsetDateTime::now_local()?;
    let week_start = now.replace_time(Time::MIDNIGHT)
        - Duration::days(now.weekday().number_from_monday() as i64 - 1);
    Ok((week_start, week_start + Duration::days(7)))
}

pub fn this_month() -> Result<(OffsetDateTime, OffsetDateTime)> {
    let now = OffsetDateTime::now_local()?;
    let from = now.replace_day(1).unwrap().replace_time(Time::MIDNIGHT);
    let (next_month, next_year) = if now.month() == Month::December {
        (Month::January, now.year() + 1)
    } else {
        (now.month().next(), now.year())
    };
    let to = time::Date::from_calendar_date(next_year, next_month, 1)
        .unwrap()
        .with_time(Time::MIDNIGHT)
        .assume_offset(now.offset());
    Ok((from, to))
}

pub fn this_year() -> Result<(OffsetDateTime, OffsetDateTime)> {
    let now = OffsetDateTime::now_local()?;
    let from = time::Date::from_calendar_date(now.year(), Month::January, 1)
        .unwrap()
        .with_time(Time::MIDNIGHT)
        .assume_offset(now.offset());
    let to = time::Date::from_calendar_date(now.year() + 1, Month::January, 1)
        .unwrap()
        .with_time(Time::MIDNIGHT)
        .assume_offset(now.offset());
    Ok((from, to))
}
