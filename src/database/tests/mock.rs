use std::cell::RefCell;
use std::io::{Read, Seek, SeekFrom, Write};

use crate::utils::io::traits::Truncate;

pub struct MockFile {
    pub data: RefCell<Vec<u8>>,
    pub pos: u64,
}

impl std::fmt::Debug for MockFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let data = self.data.borrow();
        let s = String::from_utf8_lossy(&data);
        f.debug_struct("MockFile")
            .field("data", &s)
            .field("pos", &self.pos)
            .finish()
    }
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
