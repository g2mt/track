use std::collections::BinaryHeap;
use std::fs::File;
use std::num::NonZeroU64;
use std::process::Command;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use anyhow::Result;
use time::{OffsetDateTime, Time};

use crate::database::{Database, Frequency};

fn command_exists(cmd: &str) -> bool {
    if cmd.contains('/') {
        std::path::Path::new(cmd).is_file()
    } else {
        std::env::var("PATH")
            .unwrap_or_default()
            .split(':')
            .any(|dir| std::path::Path::new(dir).join(cmd).is_file())
    }
}

struct ScheduleItem {
    category: Arc<str>,
    next_notification: OffsetDateTime,
    freq: Frequency,
}

impl ScheduleItem {
    fn into_next_notification(mut self, now: OffsetDateTime) -> Self {
        self.next_notification = match &self.freq {
            Frequency::Day => {
                let tomorrow = now.date().next_day().unwrap();
                tomorrow
                    .with_time(Time::MIDNIGHT)
                    .assume_offset(now.offset())
            }
            Frequency::Hour => {
                let this_hour = now.truncate_to_hour();
                this_hour.saturating_add(time::Duration::HOUR)
            }
            Frequency::DayOfWeek(weekday) => {
                let target_date = now.date();
                now.replace_time(time::Time::from_hms(0, 0, 0).unwrap())
                    .replace_date(target_date.next_occurrence(*weekday))
            }
            Frequency::DayOfMonth(day) => {
                let mut target_date = now.date().next_day().unwrap();
                while target_date.day() != *day {
                    target_date = target_date.next_day().unwrap();
                }
                now.replace_time(time::Time::from_hms(0, 0, 0).unwrap())
                    .replace_date(target_date)
            }
        };
        self
    }
}

impl Eq for ScheduleItem {}

impl PartialEq for ScheduleItem {
    fn eq(&self, other: &Self) -> bool {
        self.next_notification == other.next_notification
    }
}

impl PartialOrd for ScheduleItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        other.next_notification.partial_cmp(&self.next_notification)
    }
}

impl Ord for ScheduleItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.next_notification.cmp(&self.next_notification)
    }
}

pub struct Args<'a> {
    pub db: Database<File>,
    pub notifier: &'a str,
}

pub fn run_daemon(args: Args) -> Result<()> {
    if !command_exists(&args.notifier) {
        anyhow::bail!("Notifier binary not found: {}", args.notifier);
    }

    let pair = Arc::new((Mutex::new(false), Condvar::new()));
    let p = pair.clone();
    ctrlc::set_handler(move || {
        let (lock, cvar) = &*p;
        *lock.lock().unwrap() = true;
        cvar.notify_one();
    })?;

    let mut db = args.db;
    let mut info = db.read_info()?.unwrap_or_default();
    let mut heap = BinaryHeap::new();
    let now = OffsetDateTime::now_local()?;

    for (category, data) in info.iter() {
        if let Some(ref freq) = data.notify_every {
            let item = ScheduleItem {
                category: category.clone(),
                next_notification: OffsetDateTime::UNIX_EPOCH,
                freq: freq.clone(),
            };
            let item = if let Some(ts) = data.next_notification {
                match OffsetDateTime::from_unix_timestamp(ts.get() as i64) {
                    Ok(dt) => ScheduleItem {
                        next_notification: dt,
                        ..item
                    },
                    Err(_) => item.into_next_notification(now),
                }
            } else {
                item.into_next_notification(now)
            };
            heap.push(item);
        }
    }

    if heap.is_empty() {
        println!("No categories with set frequency.");
        return Ok(());
    }

    let (lock, cvar) = &*pair;
    let mut stop = lock.lock().unwrap();
    while !*stop {
        let Some(item) = heap.peek() else {
            break;
        };

        let now = OffsetDateTime::now_local()?;
        let sleep_until = item.next_notification;

        if sleep_until > now {
            let millis = (sleep_until - now).whole_milliseconds().max(0) as u64;
            println!("[{}] next {} on {}", now, item.category, sleep_until);
            if millis > 0 {
                stop = cvar
                    .wait_timeout(stop, Duration::from_millis(millis))
                    .unwrap()
                    .0;
            }
            if *stop {
                break;
            }
        }

        if let Some(item) = heap.pop() {
            let cat = item.category.clone();
            if let Err(e) = Command::new(&args.notifier)
                .arg(item.category.as_ref())
                .spawn()
            {
                println!("[{cat}] failed to spawn notifier: {e}");
            }
            let done_today = {
                let now = OffsetDateTime::now_local()?;
                let today_start = now.replace_time(Time::MIDNIGHT);
                let today_end = today_start.saturating_add(time::Duration::DAY);
                let start = today_start.unix_timestamp() as u64;
                let end = today_end.unix_timestamp() as u64;
                let mut iter = db.entries();
                iter.any(|r| match r {
                    Ok((_, entry)) => {
                        entry.category == cat && entry.start_time >= start && entry.start_time < end
                    }
                    Err(_) => false,
                })
            };
            let next_item = if !done_today {
                let mut item = item;
                item.next_notification = OffsetDateTime::now_local()?
                    .truncate_to_hour()
                    .saturating_add(time::Duration::hours(1));
                item
            } else {
                item.into_next_notification(OffsetDateTime::now_local()?)
            };
            if let Some(data) = info.data_mut(&cat) {
                data.next_notification =
                    NonZeroU64::new(next_item.next_notification.unix_timestamp() as u64);
                if let Err(e) = db.write_info(&info) {
                    println!("[{cat}] failed to save next_notification: {e}");
                }
            }
            heap.push(next_item);
        }
    }

    Ok(())
}
