use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};

use anyhow::Result;

use super::Entry;
use crate::database::BUFFER_SIZE;

#[derive(Debug, PartialEq)]
enum Direction {
    Forward,
    Backward,
}

pub struct Iter<'a, Backing: Seek + Read> {
    backing: BufReader<&'a mut Backing>,
    head_offset: Option<u64>, // points to the next, unread element
    tail_offset: Option<u64>, // points to the previous, unread element
    seek_dir: Direction,
    had_error: bool,
}

impl<'a, Backing: Seek + Read> Iter<'a, Backing> {
    pub(super) fn new(backing: &'a mut Backing) -> Self {
        Self {
            backing: BufReader::new(backing),
            head_offset: None,
            tail_offset: None,
            seek_dir: Direction::Forward,
            had_error: false,
        }
    }

    fn calc_first_entry_offset(&mut self) -> Result<()> {
        if self.head_offset.is_some() {
            return Ok(());
        }
        self.backing.seek(SeekFrom::Start(0))?;
        let mut buf = [0u8; BUFFER_SIZE];
        loop {
            let n = self.backing.read(&mut buf)?;
            if n == 0 {
                break;
            }
            if let Some(pos) = buf[..n].iter().position(|&b| b == b'\n') {
                let extra = n - pos - 1;
                if extra > 0 {
                    self.backing.seek(SeekFrom::Current(-(extra as i64)))?;
                }
                break;
            }
        }
        let offset = self.backing.seek(SeekFrom::Current(0))?;
        self.head_offset = Some(offset);
        Ok(())
    }
}

impl<'a, Backing: Seek + Read> Iterator for Iter<'a, Backing> {
    type Item = Result<Entry>;

    fn next(&mut self) -> Option<Self::Item> {
        macro_rules! iter_try {
            ($expr:expr) => {{
                match $expr {
                    Ok(x) => x,
                    Err(e) => {
                        self.had_error = true;
                        return Some(Err(e.into()));
                    }
                }
            }};
        }
        if self.had_error {
            return None;
        }
        match self.seek_dir {
            Direction::Forward => {
                iter_try!(self.calc_first_entry_offset());
            }
            Direction::Backward => {
                self.seek_dir = Direction::Forward;
                if let Some(head_offset) = self.head_offset {
                    iter_try!(self.backing.seek(SeekFrom::Start(head_offset)));
                }
            }
        }
        let mut line = Vec::new();
        let n = self.backing.read_until(b'\n', &mut line).ok()?;
        if n == 0 {
            return None;
        }
        if line.last() == Some(&b'\n') {
            line.pop();
        }
        if line.is_empty() {
            return None;
        }
        Some(serde_json::from_slice(&line).map_err(Into::into))
    }
}

impl<'a, Backing: Seek + Read> DoubleEndedIterator for Iter<'a, Backing> {
    fn next_back(&mut self) -> Option<Self::Item> {
        todo!()
    }
}
