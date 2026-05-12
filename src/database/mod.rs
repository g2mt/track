use std::io::{Read, Seek, Write};

use anyhow::Result;

pub mod schema;
pub use schema::Info;

#[cfg(test)]
mod tests;

pub struct Database<Backing: Seek + Read> {
    backing: Backing,
}

const PADDING_SIZE: usize = 128;

impl<Backing: Seek + Read> Database<Backing> {
    pub fn new(backing: Backing) -> Self {
        Self { backing }
    }

    pub fn read_info(&mut self) -> Result<Info> {
        self.backing.seek(std::io::SeekFrom::Start(0))?;

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

        if first_line_bytes.is_empty() {
            anyhow::bail!("no info line found");
        }

        // Remove trailing newline and padding spaces
        let json_str = String::from_utf8(first_line_bytes)?;
        let json_str = json_str.trim_end_matches(&[' ', '\n'][..]);

        Ok(serde_json::from_str(json_str)?)
    }
}

impl<Backing: Seek + Read + Write> Database<Backing> {
    pub fn write_info(&mut self, info: &Info) -> Result<()> {
        self.backing.seek(std::io::SeekFrom::Start(0))?;

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
