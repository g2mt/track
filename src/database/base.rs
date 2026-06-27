use std::io::{Read, Seek, SeekFrom, Write};
use std::ops::RangeBounds;

use anyhow::Result;
use time::OffsetDateTime;

use super::*;
use crate::utils::io::traits::Truncate;

pub struct MergeResult {
    /// Number of new entries merged from source
    pub new_source_entries: u64,
    /// The latest entry shared between the merge destination and source, includes its offset in
    /// the destination database
    pub common_span_entry: Option<(Span, Entry)>,
}

pub struct Database<Backing: Seek + Read> {
    #[cfg(test)]
    pub(crate) backing: Backing,
    #[cfg(not(test))]
    pub(super) backing: Backing,
}

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

impl<Backing: Seek + Read + Write + Truncate> Database<Backing> {
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

    /// Appends an entry as a new line at the end of the backing storage,
    /// returning the byte span of the appended entry.
    pub fn append_entry(&mut self, entry: &Entry) -> Result<Span> {
        self.backing.seek(SeekFrom::End(0))?;
        let start = self.backing.stream_position()?;
        let json = serde_json::to_string(entry)?;
        self.backing.write_all(json.as_bytes())?;
        self.backing.write_all(b"\n")?;
        let end = self.backing.stream_position()?;
        Ok(Span::new(start, end))
    }

    /// Replaces the entry in the span with a new entry in place, and may allocate new space in the
    /// backing.
    /// This function does not parse the older entry and assumes the
    /// span correctly contains the contents of an older entry.
    pub fn replace_entry(&mut self, span: Span, entry: &Entry) -> Result<()> {
        let json = serde_json::to_string(entry)?;
        let new_line = format!("{}\n", json);
        let new_bytes = new_line.as_bytes();

        self.backing.seek(SeekFrom::Start(span.end()))?;
        let mut rest = Vec::new();
        self.backing.read_to_end(&mut rest)?;

        self.backing.seek(SeekFrom::Start(span.start()))?;
        self.backing.write_all(new_bytes)?;
        self.backing.write_all(&rest)?;

        let new_len = span.start() + new_bytes.len() as u64 + rest.len() as u64;
        self.backing.set_len(new_len)?;

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

    /// Merges entries from `source` into `self`.
    ///
    /// Walks both databases backwards to find the latest common entry (compared by
    /// equality). Everything after that common entry is considered divergent.
    /// `self` is truncated to keep only entries up to (and including) the common
    /// entry, then all unique entries from both databases' divergent portions are
    /// appended, sorted by start time.
    pub fn merge(&mut self, source: &mut Self) -> Result<MergeResult> {
        let (mut divergent, new_source_entries, common_entry) = {
            let mut self_iter = self.entries();
            let mut source_iter = source.entries();

            let mut divergent = Vec::new();
            let mut new_source_entries: u64 = 0;
            let mut common_entry: Option<(Span, Entry)> = None;

            loop {
                let self_next = self_iter.next_back();
                let source_next = source_iter.next_back();

                match (self_next, source_next) {
                    (Some(Ok((ss, se))), Some(Ok((_, te)))) => {
                        if se == te {
                            common_entry = Some((ss, se));
                            break;
                        }
                        divergent.push(se);
                        divergent.push(te);
                        new_source_entries += 1;
                    }
                    (Some(Ok((_, se))), None) => {
                        divergent.push(se);
                        break;
                    }
                    (None, Some(Ok((_, te)))) => {
                        divergent.push(te);
                        new_source_entries += 1;
                        break;
                    }
                    (None, None) => break,
                    (Some(Err(e)), _) | (_, Some(Err(e))) => return Err(e),
                }
            }

            (divergent, new_source_entries, common_entry)
        };

        // Truncate self: remove everything after the common entry
        let truncate_start = common_entry
            .as_ref()
            .map(|(span, _)| Span::new(span.end(), span.end()));
        self.remove_span(truncate_start, None)?;

        // Append all divergent entries, sorted by start time
        divergent.sort_by_key(|e| e.start_time);
        for entry in &divergent {
            self.append_entry(entry)?;
        }

        Ok(MergeResult {
            new_source_entries,
            common_span_entry: common_entry,
        })
    }
}
