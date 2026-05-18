mod base;
pub mod iter;
pub mod range;
pub mod schema;

#[cfg(test)]
mod tests;

use std::fs::TryLockError;
use std::ops::{Deref, DerefMut};

pub use base::Database;
pub use schema::{CategoryData, Entry, Frequency, Info};

use crate::utils::io::traits::Changeable;
use crate::utils::io::FileWithPath;

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

pub const BUFFER_SIZE: usize = 128;

pub type NormalDb = Database<FileWithPath>;

pub struct ReloadableDb {
    db: Database<FileWithPath>,
    unlocked: bool,
}

impl ReloadableDb {
    pub fn reload(mut self) -> std::io::Result<(Self, bool)> {
        if self.db.backing.changed() {
            let (path, options) = self.db.backing.into_open_args();
            let file = FileWithPath::open(path, options)?;
            self.db = Database::new(file);
            self.unlocked = false;
            Ok((self, true))
        } else {
            Ok((self, false))
        }
    }

    pub fn unlock(&mut self) -> std::io::Result<()> {
        if self.unlocked {
            return Ok(());
        }
        self.db.backing.unlock()?;
        self.unlocked = true;
        Ok(())
    }

    pub fn try_lock(&mut self) -> Result<(), TryLockError> {
        self.db.backing.try_lock()?;
        self.unlocked = false;
        Ok(())
    }
}

impl Into<ReloadableDb> for NormalDb {
    fn into(self) -> ReloadableDb {
        ReloadableDb {
            db: self,
            unlocked: false,
        }
    }
}

impl DerefMut for ReloadableDb {
    fn deref_mut(&mut self) -> &mut Self::Target {
        if self.unlocked {
            panic!("dereferencing unlocked ReloadableDb");
        }
        &mut self.db
    }
}

impl Deref for ReloadableDb {
    type Target = Database<FileWithPath>;

    fn deref(&self) -> &Self::Target {
        if self.unlocked {
            panic!("dereferencing unlocked ReloadableDb");
        }
        &self.db
    }
}
