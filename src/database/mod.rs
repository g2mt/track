use std::io::{Read, Seek, SeekFrom, Write};
use std::ops::RangeBounds;

use anyhow::Result;

pub mod iter;
pub mod range;
pub mod schema;
pub use schema::{CategoryData, Entry, Frequency, Info};
use time::OffsetDateTime;

use crate::utils::io::Truncate;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy)]
pub struct Span {
    start: u64,
    end: u64,
}

impl Span {
    pub fn new(start: u64, end: u64) -> Self {
        Self { start, end }
    }

    pub fn start(&self) -> u64 {
        self.start
    }

    pub fn end(&self) -> u64 {
        self.end
    }
}

pub struct Database<Backing: Seek + Read> {
    backing: Backing,
}

const BUFFER_SIZE: usize = 128;

impl<Backing: Seek + Read> Database<Backing> {
    /// Creates a new `Database` wrapping the given backing storage.
    pub fn new(backing: Backing) -> Self {
        Self { backing }
    }

    fn end_of_info_pos(&mut self) -> Result<u64> {
        self.backing.seek(SeekFrom::Start(0))?;

        let mut res = 0u64;
        let mut buf = [0u8; BUFFER_SIZE];
        loop {
            let n = self.backing.read(&mut buf)?;
            if n == 0 {
                break;
            }
            if let Some(pos) = buf[..n].iter().position(|&b| b == b'\n') {
                res += pos as u64;
                break;
            }
            res += n as u64;
        }
        Ok(res)
    }

    /// Reads the metadata header from the first line of the backing storage.
    pub fn read_info(&mut self) -> Result<Option<Info>> {
        self.backing.seek(SeekFrom::Start(0))?;

        let mut first_line_bytes = Vec::new();
        let mut buf = [0u8; BUFFER_SIZE];
        loop {
            let n = self.backing.read(&mut buf)?;
            if n == 0 {
                break;
            }
            if let Some(pos) = buf[..n].iter().position(|&b| b == b'\n') {
                first_line_bytes.extend_from_slice(&buf[..=pos]);
                break;
            }
            first_line_bytes.extend_from_slice(&buf[..n]);
        }

        // Trim empty space at end of line
        if let Some(last_idx) = first_line_bytes
            .iter()
            .rposition(|&b| b != b'\n' && b != b' ')
        {
            first_line_bytes.truncate(last_idx + 1);
        } else {
            first_line_bytes.clear();
        }

        if first_line_bytes.is_empty() {
            Ok(None)
        } else {
            let json_str = String::from_utf8(first_line_bytes)?;
            Ok(Some(serde_json::from_str(&json_str)?))
        }
    }

    /// Returns a double-ended iterator over all entries in the backing storage.
    pub fn entries(&mut self) -> iter::Iter<'_, Backing> {
        iter::Iter::new(&mut self.backing)
    }

    /// Returns a double-ended iterator over the entries whose start date lies in a given range.
    /// This iterator walks through the file using `.entries().rev()`. It skips entries ocurring
    /// after the range, and ends once it encounters an entry ocurring before the range.
    pub fn latest_entries_range<R>(&mut self, range: R) -> range::LatestRange<'_, Backing, R>
    where
        R: RangeBounds<OffsetDateTime>,
    {
        range::LatestRange::new(&mut self.backing, range)
    }
}

impl<Backing: Seek + Read + Write> Database<Backing> {
    /// Writes the metadata header to the first line of the backing storage.
    pub fn write_info(&mut self, info: &Info) -> Result<()> {
        self.backing.seek(SeekFrom::Start(0))?;

        // Read just the first line to find where \n is
        let mut first_line_bytes = Vec::new();
        let mut buf = [0u8; BUFFER_SIZE];
        loop {
            let n = self.backing.read(&mut buf)?;
            if n == 0 {
                break;
            }
            if let Some(pos) = buf[..n].iter().position(|&b| b == b'\n') {
                first_line_bytes.extend_from_slice(&buf[..=pos]);
                break;
            }
            first_line_bytes.extend_from_slice(&buf[..n]);
        }
        let first_line_end = first_line_bytes.len();

        let json = serde_json::to_string(info)?.into_bytes();
        // Total line length (JSON + 1 for \n) must be a multiple of PADDING_SIZE
        let line_len = json.len() + 1;

        let padded_len = line_len.next_multiple_of(BUFFER_SIZE).max(first_line_end);
        let padding = padded_len - line_len;
        let mut new_line = json;
        new_line.reserve_exact(padding + 1);
        for _ in 0..padding {
            new_line.push(b' ');
        }
        new_line.push(b'\n');

        if new_line.len() == first_line_end {
            // Overwrite in-place: the old padding (or lack thereof) is replaced by
            // the new content; any leftover old bytes after \n are never read.
            self.backing.seek(std::io::SeekFrom::Start(0))?;
            self.backing.write_all(&new_line)?;
        } else if new_line.len() > first_line_end {
            // New line is longer than the old one – shift the rest of the file.
            let mut rest = Vec::new();
            self.backing.read_to_end(&mut rest)?;
            self.backing.seek(std::io::SeekFrom::Start(0))?;
            self.backing.write_all(&new_line)?;
            self.backing.write_all(&rest)?;
        } else {
            unreachable!();
        }

        Ok(())
    }
}

impl<Backing: Seek + Read + Write + Truncate> Database<Backing> {
    /// Appends an entry as a new line at the end of the backing storage.
    pub fn append_entry(&mut self, entry: &Entry) -> Result<()> {
        self.backing.seek(SeekFrom::End(0))?;
        let json = serde_json::to_string(entry)?;
        self.backing.write_all(json.as_bytes())?;
        self.backing.write_all(b"\n")?;
        Ok(())
    }

    /// Replaces the last entry in the backing storage in-place.
    pub fn update_last_entry(&mut self, entry: &Entry) -> Result<()> {
        let json = serde_json::to_string(entry)?;
        let entry_line = format!("{}\n", json);
        let entry_bytes = entry_line.as_bytes();

        let mut pos = self.backing.seek(SeekFrom::End(0))?;
        while pos > 0 {
            pos = self
                .backing
                .seek(SeekFrom::Current(-(BUFFER_SIZE as i64)))?;
            let chunk_start = pos;
            let mut buf = vec![0u8; BUFFER_SIZE.try_into().unwrap()];
            let n = self.backing.read(&mut buf)?;
            buf.truncate(n);

            let last_nl = buf
                .iter()
                .enumerate() // track byte indices
                .rev() // iterate from end of buffer
                .skip_while(|(_, b)| b.is_ascii_whitespace()) // skip trailing whitespace/newlines
                .find(|(_, b)| **b == b'\n') // find the newline before the entry
                .map(|(idx, _)| idx); // extract the index

            if let Some(last_nl) = last_nl {
                self.backing
                    .seek(SeekFrom::Start(pos + (last_nl as u64) + 1))?;
                self.backing.write_all(entry_bytes)?;
                let stream_position = self.backing.stream_position()?;
                self.backing.set_len(stream_position)?;
                return Ok(());
            } else {
                // no new line character found, go to the previous chunk
                pos = self.backing.seek(SeekFrom::Start(chunk_start))?;
            }
        }
        Ok(())
    }

    /// Removes the content in [start_span, end_span] from the backing, returning the number of
    /// bytes removed. `None` represents unbounded (start of file / end of file).
    pub fn remove_span(&mut self, start_span: Option<Span>, end_span: Option<Span>) -> Result<u64> {
        let start = match start_span {
            Some(s) => s.start(),
            None => self.end_of_info_pos()? + 1, // skip the newline character
        };
        let end = match end_span {
            Some(s) => s.end(),
            None => self.backing.seek(SeekFrom::End(0))?,
        };
        let removed = end.checked_sub(start).unwrap();

        self.backing.seek(SeekFrom::Start(end))?;
        let mut rest = Vec::new();
        self.backing.read_to_end(&mut rest)?;

        self.backing.seek(SeekFrom::Start(start))?;
        self.backing.write_all(&rest)?;

        let new_len = start + rest.len() as u64;
        self.backing.set_len(new_len)?;

        Ok(removed)
    }
}
