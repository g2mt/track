use std::cell::RefCell;
use std::io::{Read, Seek, SeekFrom, Write};

use super::*;
use crate::database::Span;
use crate::io_utils::Truncate;

// Need to interior mutability for Truncate to work with &self
#[derive(Debug)]
struct MockFile {
    data: RefCell<Vec<u8>>,
    pos: u64,
}

impl Read for MockFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let data = self.data.borrow();
        let slice = &data[self.pos as usize..];
        let n = std::cmp::min(slice.len(), buf.len());
        buf[..n].copy_from_slice(&slice[..n]);
        self.pos += n as u64;
        Ok(n)
    }
}

impl Write for MockFile {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let pos = self.pos as usize;
        let end = pos + buf.len();
        {
            let mut data = self.data.borrow_mut();
            if end > data.len() {
                data.resize(end, 0);
            }
            data[pos..end].copy_from_slice(buf);
        }
        self.pos = end as u64;
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Seek for MockFile {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let data_len = self.data.borrow().len();
        let new_pos: u64 = match pos {
            SeekFrom::Start(p) => p.try_into().unwrap(),
            SeekFrom::End(p) => p.strict_add_unsigned(data_len as u64).try_into().unwrap(),
            SeekFrom::Current(p) => p
                .strict_add_unsigned(self.pos as u64)
                .clamp(0, data_len as i64)
                .try_into()
                .unwrap(),
        };
        self.pos = new_pos;
        Ok(self.pos)
    }
}

impl Truncate for MockFile {
    fn set_len(&self, len: u64) -> std::io::Result<()> {
        self.data.borrow_mut().truncate(len as usize);
        Ok(())
    }
}

#[test]
fn append_and_update_entry() {
    let mut db: Database<MockFile> = Database::new(MockFile {
        data: RefCell::new(vec![]),
        pos: 0,
    });

    db.write_info(&Info::default()).unwrap();

    let entry1 = Entry {
        category: "c1".into(),
        start_time: 10,
        end_time: 20,
    };
    let entry2 = Entry {
        category: "c2".into(),
        start_time: 30,
        end_time: 40,
    };

    db.append_entry(&entry1).unwrap();
    db.append_entry(&entry2).unwrap();

    let entry2_updated = Entry {
        category: "c2".into(),
        start_time: 30,
        end_time: 50,
    };
    db.update_last_entry(&entry2_updated).unwrap();

    let mut iter = db.entries();
    assert_eq!(iter.next().unwrap().unwrap().1, entry1);
    assert_eq!(iter.next().unwrap().unwrap().1, entry2_updated);
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
            category: "c1".into(),
            start_time: 10,
            end_time: 20,
        },
        Entry {
            category: "c2".into(),
            start_time: 30,
            end_time: 40,
        },
        Entry {
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
            category: "c1".into(),
            start_time: 10,
            end_time: 20,
        },
        Entry {
            category: "c2".into(),
            start_time: 30,
            end_time: 40,
        },
        Entry {
            category: "c3".into(),
            start_time: 50,
            end_time: 60,
        },
        Entry {
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
        categories: vec!["a".into(), "b".into()],
        ..Default::default()
    };
    db.write_info(&info).unwrap();

    let entries = [
        Entry {
            category: "a".into(),
            start_time: 0,
            end_time: 1,
        },
        Entry {
            category: "b".into(),
            start_time: 2,
            end_time: 3,
        },
        Entry {
            category: "c".into(),
            start_time: 4,
            end_time: 5,
        },
        Entry {
            category: "d".into(),
            start_time: 6,
            end_time: 7,
        },
        Entry {
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
        categories: vec!["a".into(), "b".into()],
        ..Default::default()
    };
    db.write_info(&info).unwrap();

    let entries = [
        Entry {
            category: "a".into(),
            start_time: 0,
            end_time: 1,
        },
        Entry {
            category: "b".into(),
            start_time: 2,
            end_time: 3,
        },
        Entry {
            category: "c".into(),
            start_time: 4,
            end_time: 5,
        },
        Entry {
            category: "d".into(),
            start_time: 6,
            end_time: 7,
        },
        Entry {
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
        categories: vec!["a".into(), "b".into()],
        ..Default::default()
    };
    db.write_info(&info).unwrap();

    let entries = [
        Entry {
            category: "a".into(),
            start_time: 0,
            end_time: 1,
        },
        Entry {
            category: "b".into(),
            start_time: 2,
            end_time: 3,
        },
        Entry {
            category: "c".into(),
            start_time: 4,
            end_time: 5,
        },
        Entry {
            category: "d".into(),
            start_time: 6,
            end_time: 7,
        },
        Entry {
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
