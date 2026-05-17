use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io::Cursor;
use std::num::NonZeroU64;
use std::sync::{Arc, LazyLock};

use super::super::{CategoryData, Database, Entry, Info};
use super::mock::MockFile;

static TEST_DATA_PADDING: LazyLock<Arc<str>> = LazyLock::new(|| Arc::from("X".repeat(256)));

#[test]
fn write_info_fits_in_place() {
    let mut db: Database<Cursor<Vec<u8>>> = Database::new(Cursor::new(vec![]));
    let info = Info {
        categories: BTreeMap::from([("test".into(), CategoryData::default())]),
        ..Default::default()
    };
    db.write_info(&info).unwrap();
    let content = db.backing.into_inner();
    assert!(content.starts_with(b"{\"categories\":{\"test\":{}}}"));
    // Line length must be a multiple of 128
    let newline_pos = content.iter().position(|&b| b == b'\n').unwrap();
    assert_eq!((newline_pos + 1) % 128, 0);
}

#[test]
fn read_info_roundtrip() {
    let mut db: Database<Cursor<Vec<u8>>> = Database::new(Cursor::new(vec![]));
    let info = Info {
        categories: BTreeMap::from([
            (
                "project1".into(),
                CategoryData {
                    goal: NonZeroU64::new(3600),
                    ..Default::default()
                },
            ),
            (
                "project2".into(),
                CategoryData {
                    goal: NonZeroU64::new(7200),
                    ..Default::default()
                },
            ),
        ]),
        ..Default::default()
    };
    db.write_info(&info).unwrap();
    let read_back = db.read_info().unwrap().expect("info present");
    assert_eq!(read_back.categories, info.categories);
}

#[test]
fn read_info_empty_file() {
    let mut db: Database<Cursor<Vec<u8>>> = Database::new(Cursor::new(vec![]));
    let result = db.read_info();
    assert!(result.unwrap().is_none());
}

#[test]
fn write_info_empty_file() {
    let mut db: Database<Cursor<Vec<u8>>> = Database::new(Cursor::new(vec![]));
    let info = Info::default();
    db.write_info(&info).unwrap();
    let content = db.backing.into_inner();
    assert!(!content.is_empty());
    let newline_pos = content.iter().position(|&b| b == b'\n').unwrap();
    assert_eq!((newline_pos + 1) % 128, 0);
}

#[test]
fn write_info_new_line_longer_shifts_rest() {
    let mut db: Database<MockFile> = Database::new(MockFile {
        data: RefCell::new(vec![]),
        pos: 0,
    });

    // Write initial (short) info
    let old_info = Info {
        categories: BTreeMap::from([("a".into(), CategoryData::default())]),
        ..Default::default()
    };
    db.write_info(&old_info).unwrap();

    // Append an entry
    let entry = Entry {
        category: "a".into(),
        start_time: 1,
        end_time: 2,
    };
    db.append_entry(&entry).unwrap();

    // Write new (longer) info that won't fit in place
    let new_info = Info {
        categories: BTreeMap::from([(
            "project".into(),
            CategoryData {
                goal: NonZeroU64::new(3600),
                ..Default::default()
            },
        )]),
        test_data: Some((*TEST_DATA_PADDING).clone()),
    };
    db.write_info(&new_info).unwrap();

    // Verify the entry is still readable via iteration
    let mut iter = db.entries();
    let (_, read_entry) = iter.next().unwrap().unwrap();
    assert_eq!(read_entry, entry);
    assert!(iter.next().is_none());

    // Verify first line is multiple of 128
    let result = db.backing.data.borrow();
    let newline_pos = result.iter().position(|&b| b == b'\n').unwrap();
    assert_eq!((newline_pos + 1) % 128, 0);
}

#[test]
fn write_info_new_line_shorter_shifts_rest() {
    let mut db: Database<MockFile> = Database::new(MockFile {
        data: RefCell::new(vec![]),
        pos: 0,
    });

    // Write initial (long) info
    let old_info = Info {
        categories: BTreeMap::from([(
            "project".into(),
            CategoryData {
                goal: NonZeroU64::new(3600),
                ..Default::default()
            },
        )]),
        test_data: Some((*TEST_DATA_PADDING).clone()),
    };
    db.write_info(&old_info).unwrap();

    // Append an entry
    let entry = Entry {
        category: "a".into(),
        start_time: 1,
        end_time: 2,
    };
    db.append_entry(&entry).unwrap();

    // Write new (shorter) info that fits in place
    let new_info = Info {
        categories: BTreeMap::from([("a".into(), CategoryData::default())]),
        ..Default::default()
    };
    db.write_info(&new_info).unwrap();
    eprintln!("{:?}", db.backing);

    // Verify the entry is still readable via iteration
    let mut iter = db.entries();
    let (_, read_entry) = iter.next().unwrap().unwrap();
    assert_eq!(read_entry, entry);
    assert!(iter.next().is_none());

    // Verify first line is multiple of 128
    let result = db.backing.data.borrow();
    let newline_pos = result.iter().position(|&b| b == b'\n').unwrap();
    assert_eq!((newline_pos + 1) % 128, 0);
}
