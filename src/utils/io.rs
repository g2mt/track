use std::fs::File;
use std::io::{self, Read, Seek, Write};
use std::path::PathBuf;
use std::time::SystemTime;

pub mod traits {
    pub trait Truncate {
        fn set_len(&self, size: u64) -> std::io::Result<()>;
    }

    pub trait Changeable {
        fn changed(&self) -> bool;
    }
}

/// A `File` bundled with its `PathBuf`, implementing `Seek`, `Read`, `Write`,
/// and `Truncate` by delegating to the inner file.
#[derive(Debug)]
pub struct FileWithPath {
    file: File,
    open_args: (PathBuf, std::fs::OpenOptions),
    snap: (Option<SystemTime>, u64),
}

impl FileWithPath {
    pub fn open(path: PathBuf, options: std::fs::OpenOptions) -> io::Result<Self> {
        let file = options.open(&path)?;
        file.try_lock()?;
        let metadata = file.metadata()?;
        let snap = (metadata.modified().ok(), metadata.len());
        Ok(Self {
            file,
            open_args: (path, options),
            snap,
        })
    }

    pub fn into_open_args(self) -> (PathBuf, std::fs::OpenOptions) {
        self.open_args
    }

    pub fn unlock(&self) -> io::Result<()> {
        self.file.unlock()
    }
}

impl Seek for FileWithPath {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.file.seek(pos)
    }
}

impl Read for FileWithPath {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.file.read(buf)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        self.file.read_to_end(buf)
    }
}

impl Write for FileWithPath {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.file.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.file.write_all(buf)
    }
}

impl self::traits::Truncate for FileWithPath {
    fn set_len(&self, size: u64) -> std::io::Result<()> {
        self.file.set_len(size)
    }
}

impl self::traits::Changeable for FileWithPath {
    fn changed(&self) -> bool {
        std::fs::metadata(&self.open_args.0)
            .ok()
            .is_some_and(|meta| {
                let size_changed = meta.len() != self.snap.1;
                let mtime_changed = self.snap.0 != meta.modified().ok();
                size_changed || mtime_changed
            })
    }
}
