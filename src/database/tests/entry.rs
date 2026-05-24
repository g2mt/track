use std::cell::RefCell;
use std::collections::BTreeMap;

use super::super::{CategoryData, Database, Entry, Info, Span};
use super::mock::MockFile;

#[test]
fn append_and_replace_entry() {
    let mut db: Database<MockFile> = Database::new(MockFile {
        data: RefCell::new(vec![]),
        pos: 0,
    });

    db.write_info(&Info::default()).unwrap();

    let entry1 = Entry {
        is_being_tracked: false,
        category: "c1".into(),
        start_time: 10,
        end_time: 20,
    };
    let entry2 = Entry {
        is_being_tracked: false,
        category: "c2".into(),
        start_time: 30,
        end_time: 40,
    };

    db.append_entry(&entry1).unwrap();
    let entry2_span = db.append_entry(&entry2).unwrap();

    let entry2_updated = Entry {
        is_being_tracked: false,
        category: "a".repeat(200).into(),
        start_time: 30,
        end_time: 50,
    };
    db.replace_entry(entry2_span, &entry2_updated).unwrap();

    let mut iter = db.entries();
    assert_eq!(iter.next().unwrap().unwrap().1, entry1);
    assert_eq!(iter.next().unwrap().unwrap().1, entry2_updated);
    assert!(iter.next().is_none());
}

#[test]
fn append_and_replace_middle_entry() {
    let mut db: Database<MockFile> = Database::new(MockFile {
        data: RefCell::new(vec![]),
        pos: 0,
    });

    db.write_info(&Info::default()).unwrap();

    let entries = [
        Entry {
            is_being_tracked: false,
            category: "c1".into(),
            start_time: 10,
            end_time: 20,
        },
        Entry {
            is_being_tracked: false,
            category: "c2".into(),
            start_time: 30,
            end_time: 40,
        },
        Entry {
            is_being_tracked: false,
            category: "c3".into(),
            start_time: 50,
            end_time: 60,
        },
        Entry {
            is_being_tracked: false,
            category: "c4".into(),
            start_time: 70,
            end_time: 80,
        },
        Entry {
            is_being_tracked: false,
            category: "c5".into(),
            start_time: 90,
            end_time: 100,
        },
    ];

    let mut spans = Vec::new();
    for entry in &entries {
        spans.push(db.append_entry(entry).unwrap());
    }

    let entry2_updated = Entry {
        is_being_tracked: false,
        category: "a".repeat(200).into(),
        start_time: 30,
        end_time: 45,
    };
    db.replace_entry(spans[1], &entry2_updated).unwrap();

    let expected = [
        &entries[0],
        &entry2_updated,
        &entries[2],
        &entries[3],
        &entries[4],
    ];

    let mut iter = db.entries();
    for expected_entry in expected {
        assert_eq!(iter.next().unwrap().unwrap().1, *expected_entry);
    }
    assert!(iter.next().is_none());
}

#[test]
fn iterate_backwards() {
    let mut db: Database<MockFile> = Database::new(MockFile {
        data: RefCell::new(vec![]),
        pos: 0,
    });

    db.write_info(&Info::default()).unwrap();

    let entries = [
        Entry {
            is_being_tracked: false,
            category: "c1".into(),
            start_time: 10,
            end_time: 20,
        },
        Entry {
            is_being_tracked: false,
            category: "c2".into(),
            start_time: 30,
            end_time: 40,
        },
        Entry {
            is_being_tracked: false,
            category: "c3".into(),
            start_time: 50,
            end_time: 60,
        },
    ];

    for entry in &entries {
        db.append_entry(entry).unwrap();
    }

    let mut iter = db.entries();
    assert_eq!(iter.next_back().unwrap().unwrap().1, entries[2]);
    assert_eq!(iter.next_back().unwrap().unwrap().1, entries[1]);
    assert_eq!(iter.next_back().unwrap().unwrap().1, entries[0]);
    assert!(iter.next_back().is_none());
}

#[test]
fn iterate_backwards_and_forwards() {
    let mut db: Database<MockFile> = Database::new(MockFile {
        data: RefCell::new(vec![]),
        pos: 0,
    });

    db.write_info(&Info::default()).unwrap();

    let entries = [
        Entry {
            is_being_tracked: false,
            category: "c1".into(),
            start_time: 10,
            end_time: 20,
        },
        Entry {
            is_being_tracked: false,
            category: "c2".into(),
            start_time: 30,
            end_time: 40,
        },
        Entry {
            is_being_tracked: false,
            category: "c3".into(),
            start_time: 50,
            end_time: 60,
        },
        Entry {
            is_being_tracked: false,
            category: "c4".into(),
            start_time: 70,
            end_time: 80,
        },
    ];

    for entry in &entries {
        db.append_entry(entry).unwrap();
    }

    let mut iter = db.entries();
    assert_eq!(iter.next_back().unwrap().unwrap().1, entries[3]);
    assert_eq!(iter.next().unwrap().unwrap().1, entries[0]);
    assert_eq!(iter.next_back().unwrap().unwrap().1, entries[2]);
    assert_eq!(iter.next().unwrap().unwrap().1, entries[1]);
    assert!(iter.next_back().is_none());
    assert!(iter.next().is_none());
}

#[test]
fn remove_span_removes_middle_entries() {
    let mut db: Database<MockFile> = Database::new(MockFile {
        data: RefCell::new(vec![]),
        pos: 0,
    });

    let info = Info {
        categories: BTreeMap::from([
            ("a".into(), CategoryData::default()),
            ("b".into(), CategoryData::default()),
        ]),
        ..Default::default()
    };
    db.write_info(&info).unwrap();

    let entries = [
        Entry {
            is_being_tracked: false,
            category: "a".into(),
            start_time: 0,
            end_time: 1,
        },
        Entry {
            is_being_tracked: false,
            category: "b".into(),
            start_time: 2,
            end_time: 3,
        },
        Entry {
            is_being_tracked: false,
            category: "c".into(),
            start_time: 4,
            end_time: 5,
        },
        Entry {
            is_being_tracked: false,
            category: "d".into(),
            start_time: 6,
            end_time: 7,
        },
        Entry {
            is_being_tracked: false,
            category: "e".into(),
            start_time: 8,
            end_time: 9,
        },
    ];

    for entry in &entries {
        db.append_entry(entry).unwrap();
    }

    // Collect entry spans via iteration
    let spans: Vec<Span> = db
        .entries()
        .filter_map(|r| r.ok().map(|(s, _)| s))
        .collect();
    assert_eq!(spans.len(), 5);

    // Remove entries at indices 1, 2, 3 (a contiguous byte range)
    let start = Span::new(spans[1].start(), spans[1].end());
    let end = Span::new(spans[3].start(), spans[3].end());
    let removed = db.remove_span(Some(start), Some(end)).unwrap();
    assert!(removed > 0);

    // Re-read info and verify it's unchanged
    let info_after = db.read_info().unwrap().unwrap();
    assert_eq!(info, info_after);

    // Only entries[0] and entries[4] should remain
    let remaining: Vec<Entry> = db
        .entries()
        .filter_map(|r| r.ok().map(|(_, e)| e))
        .collect();
    assert_eq!(remaining.len(), 2);
    assert_eq!(remaining[0], entries[0]);
    assert_eq!(remaining[1], entries[4]);
}

#[test]
fn remove_span_removes_head_entries() {
    let mut db: Database<MockFile> = Database::new(MockFile {
        data: RefCell::new(vec![]),
        pos: 0,
    });

    let info = Info {
        categories: BTreeMap::from([
            ("a".into(), CategoryData::default()),
            ("b".into(), CategoryData::default()),
        ]),
        ..Default::default()
    };
    db.write_info(&info).unwrap();

    let entries = [
        Entry {
            is_being_tracked: false,
            category: "a".into(),
            start_time: 0,
            end_time: 1,
        },
        Entry {
            is_being_tracked: false,
            category: "b".into(),
            start_time: 2,
            end_time: 3,
        },
        Entry {
            is_being_tracked: false,
            category: "c".into(),
            start_time: 4,
            end_time: 5,
        },
        Entry {
            is_being_tracked: false,
            category: "d".into(),
            start_time: 6,
            end_time: 7,
        },
        Entry {
            is_being_tracked: false,
            category: "e".into(),
            start_time: 8,
            end_time: 9,
        },
    ];

    for entry in &entries {
        db.append_entry(entry).unwrap();
    }

    // Collect entry spans via iteration
    let spans: Vec<Span> = db
        .entries()
        .filter_map(|r| r.ok().map(|(s, _)| s))
        .collect();
    assert_eq!(spans.len(), 5);

    // Remove entries at indices 0, 1 (head)
    let removed = db.remove_span(None, Some(spans[1])).unwrap();
    assert!(removed > 0);

    // Re-read info and verify it's unchanged
    let info_after = db.read_info().unwrap().unwrap();
    assert_eq!(info, info_after);

    // Only entries[2], entries[3], entries[4] should remain
    let remaining: Vec<Entry> = db
        .entries()
        .filter_map(|r| r.ok().map(|(_, e)| e))
        .collect();
    assert_eq!(remaining.len(), 3);
    assert_eq!(remaining[0], entries[2]);
    assert_eq!(remaining[1], entries[3]);
    assert_eq!(remaining[2], entries[4]);
}

#[test]
fn remove_span_removes_tail_entries() {
    let mut db: Database<MockFile> = Database::new(MockFile {
        data: RefCell::new(vec![]),
        pos: 0,
    });

    let info = Info {
        categories: BTreeMap::from([
            ("a".into(), CategoryData::default()),
            ("b".into(), CategoryData::default()),
        ]),
        ..Default::default()
    };
    db.write_info(&info).unwrap();

    let entries = [
        Entry {
            is_being_tracked: false,
            category: "a".into(),
            start_time: 0,
            end_time: 1,
        },
        Entry {
            is_being_tracked: false,
            category: "b".into(),
            start_time: 2,
            end_time: 3,
        },
        Entry {
            is_being_tracked: false,
            category: "c".into(),
            start_time: 4,
            end_time: 5,
        },
        Entry {
            is_being_tracked: false,
            category: "d".into(),
            start_time: 6,
            end_time: 7,
        },
        Entry {
            is_being_tracked: false,
            category: "e".into(),
            start_time: 8,
            end_time: 9,
        },
    ];

    for entry in &entries {
        db.append_entry(entry).unwrap();
    }

    // Collect entry spans via iteration
    let spans: Vec<Span> = db
        .entries()
        .filter_map(|r| r.ok().map(|(s, _)| s))
        .collect();
    assert_eq!(spans.len(), 5);

    // Remove entries at indices 3, 4 (tail)
    let removed = db.remove_span(Some(spans[3]), None).unwrap();
    assert!(removed > 0);

    // Re-read info and verify it's unchanged
    let info_after = db.read_info().unwrap().unwrap();
    assert_eq!(info, info_after);

    // Only entries[0], entries[1], entries[2] should remain
    let remaining: Vec<Entry> = db
        .entries()
        .filter_map(|r| r.ok().map(|(_, e)| e))
        .collect();
    assert_eq!(remaining.len(), 3);
    assert_eq!(remaining[0], entries[0]);
    assert_eq!(remaining[1], entries[1]);
    assert_eq!(remaining[2], entries[2]);
}
