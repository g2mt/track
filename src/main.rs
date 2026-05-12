use anyhow::Result;
use clap::Parser;
use time::{Duration, Month, OffsetDateTime, Time};
mod cli;
mod logs;
mod track;
use cli::Cli;

fn main() -> Result<()> {
    let args = Cli::parse();

    if args.logs.today {
        let now = OffsetDateTime::now_local()?;
        logs::show_logs(
            Some(now.replace_time(Time::MIDNIGHT)),
            Some(now.replace_time(Time::MIDNIGHT) + Duration::DAY),
        )
    } else if args.logs.yesterday {
        let now = OffsetDateTime::now_local()?;
        let today_start = now.replace_time(Time::MIDNIGHT);
        logs::show_logs(Some(today_start - Duration::DAY), Some(today_start))
    } else if args.logs.this_week {
        let now = OffsetDateTime::now_local()?;
        let week_start = now.replace_time(Time::MIDNIGHT)
            - Duration::days(now.weekday().number_from_monday() as i64 - 1);
        logs::show_logs(Some(week_start), Some(week_start + Duration::days(7)))
    } else if args.logs.this_month {
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
        logs::show_logs(Some(from), Some(to))
    } else if args.logs.this_year {
        let now = OffsetDateTime::now_local()?;
        let from = time::Date::from_calendar_date(now.year(), Month::January, 1)
            .unwrap()
            .with_time(Time::MIDNIGHT)
            .assume_offset(now.offset());
        let to = time::Date::from_calendar_date(now.year() + 1, Month::January, 1)
            .unwrap()
            .with_time(Time::MIDNIGHT)
            .assume_offset(now.offset());
        logs::show_logs(Some(from), Some(to))
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
        )
    } else if let Some(daily) = args.daily {
        todo!("set daily goal to {}", daily);
    } else if let Some(category) = args.category {
        track::track(category)
    } else {
        unreachable!()
    }
}
