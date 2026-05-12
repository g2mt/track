use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};

use anyhow::Result;

use super::Entry;
use crate::database::BUFFER_SIZE;

pub struct Iter<'a, Backing: Seek + Read> {
    backing: BufReader<&'a mut Backing>,
}

impl<'a, Backing: Seek + Read> Iter<'a, Backing> {
    pub(super) fn try_new(backing: &'a mut Backing) -> Result<Self> {
        backing.seek(SeekFrom::Start(0))?;
        let mut buf = [0u8; BUFFER_SIZE];
        loop {
            let n = backing.read(&mut buf)?;
            if n == 0 {
                break;
            }
            if let Some(pos) = buf[..n].iter().position(|&b| b == b'\n') {
                let extra = n - pos - 1;
                if extra > 0 {
                    backing.seek(SeekFrom::Current(-(extra as i64)))?;
                }
                return Ok(Self {
                    backing: BufReader::new(backing),
                });
            }
        }
        Ok(Self {
            backing: BufReader::new(backing),
        })
    }
}

impl<'a, Backing: Seek + Read> Iterator for Iter<'a, Backing> {
    type Item = Result<Entry>;

    fn next(&mut self) -> Option<Self::Item> {
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
