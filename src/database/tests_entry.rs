use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::Arc;

use super::*;
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
        let new_pos = match pos {
            SeekFrom::Start(p) => p,
            SeekFrom::End(p) => (data_len as i64 + p) as u64,
            SeekFrom::Current(p) => (self.pos as i64 + p) as u64,
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

    let mut iter = db.entries().unwrap();
    assert_eq!(iter.next().unwrap().unwrap(), entry1);
    assert_eq!(iter.next().unwrap().unwrap(), entry2_updated);
    assert!(iter.next().is_none());
}
