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

    fn initial_seek_first_entry_offset(&mut self) -> Result<u64> {
        if let Some(head_offset) = self.head_offset {
            return Ok(head_offset);
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
        Ok(offset)
    }

    fn initial_seek_last_entry_offset(&mut self) -> Result<u64> {
        if let Some(offset) = self.tail_offset {
            return Ok(offset);
        }
        // The next_back function always expect that the tail_offset starts at a position
        // containing the new line character of the entry to be parsed
        let offset = self.backing.seek(SeekFrom::End(0))?;
        self.tail_offset = Some(offset);
        Ok(offset)
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

        let pos = match self.seek_dir {
            Direction::Forward => {
                iter_try!(self.initial_seek_first_entry_offset());
                self.head_offset.unwrap()
            }
            Direction::Backward => {
                self.seek_dir = Direction::Forward;
                if let Some(head_offset) = self.head_offset {
                    iter_try!(self.backing.seek(SeekFrom::Start(head_offset)))
                } else {
                    iter_try!(self.initial_seek_first_entry_offset());
                    self.head_offset.unwrap()
                }
            }
        };
        // End the iteration if the two ends cross
        if let Some(tail_offset) = self.tail_offset
            && pos > tail_offset
        {
            return None;
        }

        // Scan the next line
        let mut line = Vec::new();
        let n = iter_try!(self.backing.read_until(b'\n', &mut line));
        if n == 0 {
            return None;
        }
        *self.head_offset.as_mut().unwrap() += n as u64;

        // Decode the next line
        if line.last() == Some(&b'\n') {
            line.pop();
        }
        if line.is_empty() {
            return None;
        }
        // In order to allow serde_json deserialization to return an error
        // without ending the iterator, had_error setting is skipped here
        Some(serde_json::from_slice(&line).map_err(Into::into))
    }
}

impl<'a, Backing: Seek + Read> DoubleEndedIterator for Iter<'a, Backing> {
    fn next_back(&mut self) -> Option<Self::Item> {
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

        // Always seek the first entry offset to ensure that the info line is skipped
        iter_try!(self.initial_seek_first_entry_offset());
        // initial_seek_first_entry_offset may updates the file offset, so it is
        // necessary to seek to the correct position again
        let mut pos = match self.seek_dir {
            Direction::Forward => {
                self.seek_dir = Direction::Backward;
                if let Some(tail_offset) = self.tail_offset {
                    iter_try!(self.backing.seek(SeekFrom::Start(tail_offset)))
                } else {
                    iter_try!(self.initial_seek_last_entry_offset())
                }
            }
            Direction::Backward => {
                iter_try!(self.initial_seek_last_entry_offset())
            }
        };
        assert_eq!(pos, self.backing.seek(SeekFrom::Current(0)).unwrap());
        // eprintln!("pos={}", pos);

        // ensure that the back and front do not cross
        while pos > self.head_offset.unwrap() {
            pos = iter_try!(self.backing.seek(SeekFrom::Current(-(BUFFER_SIZE as i64))));
            let chunk_start = pos;
            let mut buf = vec![0u8; BUFFER_SIZE.try_into().unwrap()];
            let n = iter_try!(self.backing.read(&mut buf));
            buf.truncate(n);
            // eprintln!(
            //     "read: {} {:?}",
            //     pos,
            //     String::from_utf8(buf.clone()).unwrap()
            // );

            let last_nl: Option<u64> = buf
                .iter()
                .enumerate() // track byte indices
                .rev() // iterate from end of buffer
                .skip_while(|(_, b)| b.is_ascii_whitespace()) // skip trailing whitespace/newlines
                .find(|(_, b)| **b == b'\n') // find the newline before the entry
                .map(|(idx, _)| idx.try_into().unwrap()); // extract the index

            if let Some(last_nl) = last_nl {
                let newline_pos = pos.checked_add(last_nl).unwrap();
                // eprintln!("==> {}", newline_pos);
                // Seek to after the new line character
                iter_try!(
                    self.backing
                        .seek(SeekFrom::Start(newline_pos.checked_add(1).unwrap()))
                );

                // Scan the next line
                let mut line = Vec::new();
                let n = iter_try!(self.backing.read_until(b'\n', &mut line));
                // Seek back to the end of the previous line after scanning,
                // and set the seek position for the next iteration
                self.tail_offset = Some(iter_try!(self.backing.seek(SeekFrom::Start(newline_pos))));
                if n == 0 {
                    return None;
                }

                // Decode the next line
                if line.last() == Some(&b'\n') {
                    line.pop();
                }
                if line.is_empty() {
                    return None;
                }
                // In order to allow serde_json deserialization to return an error
                // without ending the iterator, had_error setting is skipped here
                return Some(serde_json::from_slice(&line).map_err(Into::into));
            } else {
                // no new line character found, go to the previous chunk
                // eprintln!("going back to {}", chunk_start);
                pos = iter_try!(self.backing.seek(SeekFrom::Start(chunk_start)));
            }
        }

        self.tail_offset = Some(0);
        None
    }
}
