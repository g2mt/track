use std::collections::BTreeMap;
use std::io::Cursor;
use std::num::NonZeroU64;

use super::super::{CategoryData, Database, Info};

#[test]
fn write_info_fits_in_place() {
    let mut db: Database<Cursor<Vec<u8>>> = Database::new(Cursor::new(vec![]));
    let info = Info {
        categories: BTreeMap::from([("test".into(), CategoryData { goal: None })]),
    };
    db.write_info(&info).unwrap();
    let content = db.backing.into_inner();
    assert!(content.starts_with(b"{\"categories\":{\"test\":{}}}"));
    // Line length must be a multiple of 128
    let newline_pos = content.iter().position(|&b| b == b'\n').unwrap();
    assert_eq!((newline_pos + 1) % 128, 0);
}

#[test]
fn write_info_new_line_longer_shifts_rest() {
    let old_info = Info {
        categories: BTreeMap::from([("a".into(), CategoryData { goal: None })]),
    };
    let mut content = serde_json::to_string(&old_info).unwrap().into_bytes();
    let line_len = content.len() + 1;
    let padded = line_len.next_multiple_of(128);
    let padding = padded - line_len;
    for _ in 0..padding {
        content.push(b' ');
    }
    content.push(b'\n');
    content.extend_from_slice(b"[\"entry\",1,2]\n");

    let mut db: Database<Cursor<Vec<u8>>> = Database::new(Cursor::new(content));

    let new_info = Info {
        categories: BTreeMap::from([(
            "project".into(),
            CategoryData {
                goal: NonZeroU64::new(3600),
            },
        )]),
    };
    db.write_info(&new_info).unwrap();

    let result = db.backing.into_inner();
    // The rest (entries) must still be present
    assert!(result.ends_with(b"[\"entry\",1,2]\n"));
    // First line must be multiple of 128
    let newline_pos = result.iter().position(|&b| b == b'\n').unwrap();
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
                },
            ),
            (
                "project2".into(),
                CategoryData {
                    goal: NonZeroU64::new(7200),
                },
            ),
        ]),
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
