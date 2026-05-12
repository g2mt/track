use std::io::{Read, Seek};

use anyhow::Result;

use super::Entry;

pub struct Iter<'a, Backing: Seek + Read> {
    backing: &'a mut Backing,
    done: bool,
}

impl<'a, Backing: Seek + Read> Iter<'a, Backing> {
    pub(super) fn new(backing: &'a mut Backing) -> Self {
        Self {
            backing,
            done: false,
        }
    }
}

impl<'a, Backing: Seek + Read> Iterator for Iter<'a, Backing> {
    type Item = Result<Entry>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let mut line = Vec::new();
        let mut buf = [0u8; 128];
        loop {
            let n = self.backing.read(&mut buf).ok()?;
            if n == 0 {
                self.done = true;
                break;
            }
            if let Some(pos) = buf[..n].iter().position(|&b| b == b'\n') {
                line.extend_from_slice(&buf[..pos]);
                break;
            }
            line.extend_from_slice(&buf[..n]);
        }

        if line.is_empty() {
            return None;
        }

        match serde_json::from_slice::<Entry>(&line) {
            Ok(entry) => Some(Ok(entry)),
            Err(e) => Some(Err(e.into())),
        }
    }
}