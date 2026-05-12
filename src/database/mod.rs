use std::io::{Read, Seek, SeekFrom, Write};

use anyhow::Result;

pub mod schema;
pub use schema::{Entry, Info};

use crate::io_utils::Truncate;

#[cfg(test)]
mod tests_entry;
#[cfg(test)]
mod tests_info;

pub struct Database<Backing: Seek + Read> {
    backing: Backing,
}

const PADDING_SIZE: usize = 128;

impl<Backing: Seek + Read> Database<Backing> {
    pub fn new(backing: Backing) -> Self {
        Self { backing }
    }

    pub fn read_info(&mut self) -> Result<Option<Info>> {
        self.backing.seek(SeekFrom::Start(0))?;

        let mut first_line_bytes = Vec::new();
        let mut buf = [0u8; PADDING_SIZE];
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
}

impl<Backing: Seek + Read + Write> Database<Backing> {
    pub fn write_info(&mut self, info: &Info) -> Result<()> {
        self.backing.seek(SeekFrom::Start(0))?;

        // Read just the first line to find where \n is
        let mut first_line_bytes = Vec::new();
        let mut buf = [0u8; PADDING_SIZE];
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

        let json = serde_json::to_string(info)?;
        // Total line length (JSON + 1 for \n) must be a multiple of PADDING_SIZE
        let line_len = json.len() + 1;
        let padded_len = line_len.next_multiple_of(PADDING_SIZE);
        let padding = padded_len - line_len;

        let mut new_line = json.into_bytes();
        new_line.reserve_exact(padding + 1);
        for _ in 0..padding {
            new_line.push(b' ');
        }
        new_line.push(b'\n');

        if new_line.len() <= first_line_end {
            // Overwrite in-place: the old padding (or lack thereof) is replaced by
            // the new content; any leftover old bytes after \n are never read.
            self.backing.seek(std::io::SeekFrom::Start(0))?;
            self.backing.write_all(&new_line)?;
        } else {
            // New line is longer than the old one – shift the rest of the file.
            let mut rest = Vec::new();
            self.backing.read_to_end(&mut rest)?;
            self.backing.seek(std::io::SeekFrom::Start(0))?;
            self.backing.write_all(&new_line)?;
            self.backing.write_all(&rest)?;
        }

        Ok(())
    }
}

impl<Backing: Seek + Read + Write + Truncate> Database<Backing> {
    pub fn append_entry(&mut self, entry: &Entry) -> Result<()> {
        self.backing.seek(SeekFrom::End(0))?;
        let json = serde_json::to_string(entry)?;
        self.backing.write_all(json.as_bytes())?;
        self.backing.write_all(b"\n")?;
        Ok(())
    }

    pub fn update_last_entry(&mut self, entry: &Entry) -> Result<()> {
        let json = serde_json::to_string(entry)?;
        let entry_line = format!("{}\n", json);
        let entry_bytes = entry_line.as_bytes();

        let mut pos = self.backing.seek(SeekFrom::End(0))?;
        while pos > 0 {
            let read_size = (pos as usize).min(PADDING_SIZE);
            pos -= read_size as u64;
            self.backing.seek(SeekFrom::Start(pos))?;
            let mut buf = vec![0u8; read_size];
            self.backing.read_exact(&mut buf)?;
            if let Some(last_nl) = buf.iter().rposition(|&b| b == b'\n') {
                self.backing
                    .seek(SeekFrom::Start(pos + (last_nl as u64) + 1))?;
                self.backing.write_all(entry_bytes)?;
                let stream_position = self.backing.stream_position()?;
                self.backing.set_len(stream_position)?;
                return Ok(());
            }
        }
        Ok(())
    }
}
