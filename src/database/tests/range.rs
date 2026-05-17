use std::cell::RefCell;

use time::OffsetDateTime;

use super::super::{Database, Entry, Info};
use super::mock::MockFile;

fn dt(ts: u64) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(ts as i64).unwrap()
}

#[test]
fn range_all() {
    let mut db: Database<MockFile> = Database::new(MockFile {
        data: RefCell::new(vec![]),
        pos: 0,
    });

    db.write_info(&Info::default()).unwrap();

    let entries = [
        Entry {
            category: "a".into(),
            start_time: 10,
            end_time: 20,
        },
        Entry {
            category: "b".into(),
            start_time: 30,
            end_time: 40,
        },
        Entry {
            category: "c".into(),
            start_time: 50,
            end_time: 60,
        },
    ];
    for entry in &entries {
        db.append_entry(entry).unwrap();
    }

    let results: Vec<Entry> = db
        .latest_entries_range(..)
        .filter_map(|r| r.ok().map(|(_, e)| e))
        .collect();

    assert_eq!(results.len(), 3);
    assert_eq!(results[0], entries[2]);
    assert_eq!(results[1], entries[1]);
    assert_eq!(results[2], entries[0]);
}

#[test]
fn range_from_bound() {
    let mut db: Database<MockFile> = Database::new(MockFile {
        data: RefCell::new(vec![]),
        pos: 0,
    });

    db.write_info(&Info::default()).unwrap();

    let entries = [
        Entry {
            category: "a".into(),
            start_time: 10,
            end_time: 20,
        },
        Entry {
            category: "b".into(),
            start_time: 30,
            end_time: 40,
        },
        Entry {
            category: "c".into(),
            start_time: 50,
            end_time: 60,
        },
    ];
    for entry in &entries {
        db.append_entry(entry).unwrap();
    }

    let results: Vec<Entry> = db
        .latest_entries_range(dt(30)..)
        .filter_map(|r| r.ok().map(|(_, e)| e))
        .collect();

    assert_eq!(results.len(), 2);
    assert_eq!(results[0], entries[2]);
    assert_eq!(results[1], entries[1]);
}

#[test]
fn range_to_bound() {
    let mut db: Database<MockFile> = Database::new(MockFile {
        data: RefCell::new(vec![]),
        pos: 0,
    });

    db.write_info(&Info::default()).unwrap();

    let entries = [
        Entry {
            category: "a".into(),
            start_time: 10,
            end_time: 20,
        },
        Entry {
            category: "b".into(),
            start_time: 30,
            end_time: 40,
        },
        Entry {
            category: "c".into(),
            start_time: 50,
            end_time: 60,
        },
    ];
    for entry in &entries {
        db.append_entry(entry).unwrap();
    }

    let results: Vec<Entry> = db
        .latest_entries_range(..dt(40))
        .filter_map(|r| r.ok().map(|(_, e)| e))
        .collect();

    assert_eq!(results.len(), 2);
    assert_eq!(results[0], entries[1]);
    assert_eq!(results[1], entries[0]);
}

#[test]
fn range_both_bounds() {
    let mut db: Database<MockFile> = Database::new(MockFile {
        data: RefCell::new(vec![]),
        pos: 0,
    });

    db.write_info(&Info::default()).unwrap();

    let entries = [
        Entry {
            category: "a".into(),
            start_time: 10,
            end_time: 20,
        },
        Entry {
            category: "b".into(),
            start_time: 30,
            end_time: 40,
        },
        Entry {
            category: "c".into(),
            start_time: 50,
            end_time: 60,
        },
    ];
    for entry in &entries {
        db.append_entry(entry).unwrap();
    }

    let results: Vec<Entry> = db
        .latest_entries_range(dt(20)..dt(55))
        .filter_map(|r| r.ok().map(|(_, e)| e))
        .collect();

    assert_eq!(results.len(), 2);
    assert_eq!(results[0], entries[2]);
    assert_eq!(results[1], entries[1]);
}

#[test]
fn range_empty() {
    let mut db: Database<MockFile> = Database::new(MockFile {
        data: RefCell::new(vec![]),
        pos: 0,
    });

    db.write_info(&Info::default()).unwrap();

    let entries = [
        Entry {
            category: "a".into(),
            start_time: 10,
            end_time: 20,
        },
        Entry {
            category: "b".into(),
            start_time: 30,
            end_time: 40,
        },
    ];
    for entry in &entries {
        db.append_entry(entry).unwrap();
    }

    let results: Vec<Entry> = db
        .latest_entries_range(dt(100)..dt(200))
        .filter_map(|r| r.ok().map(|(_, e)| e))
        .collect();

    assert!(results.is_empty());
}

#[test]
fn range_returns_spans() {
    let mut db: Database<MockFile> = Database::new(MockFile {
        data: RefCell::new(vec![]),
        pos: 0,
    });

    db.write_info(&Info::default()).unwrap();

    db.append_entry(&Entry {
        category: "x".into(),
        start_time: 100,
        end_time: 200,
    })
    .unwrap();

    let (span, entry) = db
        .latest_entries_range(dt(50)..dt(150))
        .next()
        .unwrap()
        .unwrap();

    assert_eq!(entry.start_time, 100);
    assert!(span.start() > 0);
    assert!(span.end() > span.start());
}
