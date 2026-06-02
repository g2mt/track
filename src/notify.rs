use std::collections::BinaryHeap;
use std::process::Command;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use anyhow::Result;
use time::{OffsetDateTime, Time};

use crate::database::{Frequency, Info, ReloadableDb};
use crate::track;

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
        self.next_notification = self.freq.next_date(now);
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

fn build_heap(info: &Info, now: OffsetDateTime) -> BinaryHeap<ScheduleItem> {
    let mut heap = BinaryHeap::new();
    for (category, data) in info.iter() {
        if let Some(ref freq) = data.notify_every {
            let item = ScheduleItem {
                category: category.clone(),
                next_notification: OffsetDateTime::UNIX_EPOCH,
                freq: freq.clone(),
            };
            let item = if let Some(dt) = data.next_notification_local() {
                ScheduleItem {
                    next_notification: dt,
                    ..item
                }
            } else {
                item.into_next_notification(now)
            };
            heap.push(item);
        }
    }
    heap
}

pub struct Args<'a> {
    pub db: ReloadableDb,
    pub notifier: &'a str,
    pub notify_again: Frequency,
    pub off_from: i8,
    pub off_to: i8,
}

fn is_off_hours(off_from: i8, off_to: i8, now: OffsetDateTime) -> bool {
    if off_from < 0 || off_to < 0 {
        return false;
    }
    let hour = now.hour() as i8;
    hour >= off_from && hour <= off_to
}

pub fn run_daemon(args: Args) -> Result<()> {
    if !command_exists(&args.notifier) {
        anyhow::bail!("Notifier binary not found: {}", args.notifier);
    }

    let exited = Arc::new((Mutex::new(false), Condvar::new()));

    // Ctrl-C handler
    {
        let exited = exited.clone();
        ctrlc::set_handler(move || {
            let (mtx, cvar) = &*exited;
            *mtx.lock().unwrap() = true;
            cvar.notify_all();
        })?;
    }

    let mut db = args.db;
    let mut info = db.try_lock(|db| Ok(db.read_info()?.unwrap_or_default()))?;
    let now = crate::utils::time::now_local();
    let mut heap = build_heap(&info, now);

    if heap.is_empty() {
        println!("No categories with set frequency.");
        return Ok(());
    }

    if let Some(item) = heap.peek() {
        println!(
            "[{}] next {} on {}",
            now, item.category, item.next_notification
        );
    }

    let (mtx, cvar) = &*exited;
    loop {
        // Wait for exit signal or reload timeout
        let mut state = mtx.lock().unwrap();
        if *state {
            break;
        }
        let wait_result = cvar.wait_timeout(state, Duration::from_secs(1)).unwrap();
        state = wait_result.0;
        if *state {
            break;
        }
        if is_off_hours(args.off_from, args.off_to, now) {
            continue;
        }

        let reloaded;
        (db, reloaded) = db.reload()?;
        if reloaded {
            println!("[{}] reloaded", crate::utils::time::now_local());
            info = db.try_lock(|db| Ok(db.read_info()?.unwrap_or_default()))?;
        }
        let now = crate::utils::time::now_local();
        heap = build_heap(&info, now);

        // Nothing due yet, keep polling
        let Some(item) = heap.peek() else {
            continue;
        };
        if item.next_notification > now {
            continue;
        }
        // Spawn notifier for due item, unless the category is currently being tracked
        let Some(item) = heap.pop() else {
            continue;
        };
        let being_tracked = db.try_lock(|db| {
            Ok(db
                .entries()
                .rev()
                .take(track::LATEST_ENTRIES_TRACKED)
                .filter_map(|r| r.ok())
                .any(|(_, entry)| entry.category == item.category && entry.is_being_tracked))
        })?;
        if !being_tracked {
            if let Err(e) = Command::new(&args.notifier)
                .arg(item.category.as_ref())
                .spawn()
            {
                println!("{}: failed to spawn notifier: {e}", item.category);
            }
        }

        db.try_lock(|db| {
            // Write to database and update notification times
            // done_today is true when the total tracked duration for this category
            // today meets or exceeds its goal. If no goal is set, the notification
            // always advances to the next scheduled time without re-notifying.
            let done_today = {
                let now = crate::utils::time::now_local();
                let today_start = now.replace_time(Time::MIDNIGHT);
                let today_end = today_start.saturating_add(time::Duration::DAY);
                let goal: Option<u64> = info
                    .data(&item.category)
                    .and_then(|d| d.goal.map(|g| g.get()));
                let total: u64 = db
                    .entries()
                    .filter_map(|r| match r {
                        Ok((_, entry)) if entry.category == item.category => entry
                            .start_time_local()
                            .ok()
                            .filter(|dt| *dt >= today_start && *dt < today_end)
                            .map(|_| entry.elapsed_seconds()),
                        _ => None,
                    })
                    .sum();
                goal.map_or(true, |g| total >= g)
            };
            let next_item = if !done_today {
                let mut item = item;
                item.next_notification =
                    args.notify_again.next_date(crate::utils::time::now_local());
                item
            } else {
                item.into_next_notification(crate::utils::time::now_local())
            };

            // Record next notification time
            if let Some(data) = info.data_mut(&next_item.category) {
                data.set_next_notification_local(Some(next_item.next_notification));
                if let Err(e) = db.write_info(&info) {
                    println!(
                        "{}: failed to save next_notification: {e}",
                        next_item.category
                    );
                }
            }
            println!(
                "[{}] next {} on {}",
                crate::utils::time::now_local(),
                next_item.category,
                next_item.next_notification
            );
            heap.push(next_item);
            Ok(())
        })?;
    }

    Ok(())
}
