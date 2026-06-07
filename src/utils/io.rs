use std::fs::{File, TryLockError};
use std::io::{self, Read, Seek, Write};
use std::path::PathBuf;
use std::thread::sleep;
use std::time::{Duration, SystemTime};

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
    fn try_lock_with_retry(file: &File) -> Result<(), TryLockError> {
        for attempt in 0..3 {
            match file.try_lock() {
                Ok(()) => return Ok(()),
                Err(e) if attempt < 2 => sleep(Duration::from_secs(5)),
                Err(e) => return Err(e),
            }
        }
        unreachable!()
    }

    pub fn open(path: PathBuf, options: std::fs::OpenOptions) -> io::Result<Self> {
        let file = options.open(&path)?;
        Self::try_lock_with_retry(&file)?;
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

    pub fn try_lock(&self) -> Result<(), TryLockError> {
        Self::try_lock_with_retry(&self.file)
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
            .is_some_and(|meta| (meta.modified().ok(), meta.len()) != self.snap)
    }
}
