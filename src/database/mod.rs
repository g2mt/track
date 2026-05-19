mod base;
pub mod iter;
pub mod range;
pub mod schema;

#[cfg(test)]
mod tests;

use std::ops::{Deref, DerefMut};

use anyhow::Result;
pub use base::Database;
pub use schema::{CategoryData, Entry, Frequency, Info};

use crate::utils::io::FileWithPath;
use crate::utils::io::traits::Changeable;

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
    locked: bool,
}

impl ReloadableDb {
    pub fn reload(mut self) -> std::io::Result<(Self, bool)> {
        if self.db.backing.changed() {
            let (path, options) = self.db.backing.into_open_args();
            let file = FileWithPath::open(path, options)?;
            file.unlock()?;
            self.db = Database::new(file);
            self.locked = false;
            Ok((self, true))
        } else {
            Ok((self, false))
        }
    }

    pub fn try_lock<F, R>(&mut self, f: F) -> Result<R>
    where
        F: FnOnce(&mut Self) -> Result<R>,
    {
        if self.locked {
            panic!("nested try_lock");
        }
        self.db.backing.try_lock()?;
        self.locked = true;
        let r = f(self)?;
        self.db.backing.unlock()?;
        self.locked = false;
        Ok(r)
    }
}

impl TryInto<ReloadableDb> for NormalDb {
    type Error = std::io::Error;

    fn try_into(self) -> Result<ReloadableDb, Self::Error> {
        self.backing.unlock()?;
        Ok(ReloadableDb {
            db: self,
            locked: false,
        })
    }
}

impl DerefMut for ReloadableDb {
    fn deref_mut(&mut self) -> &mut Self::Target {
        if !self.locked {
            panic!("dereferencing unlocked ReloadableDb");
        }
        &mut self.db
    }
}

impl Deref for ReloadableDb {
    type Target = Database<FileWithPath>;

    fn deref(&self) -> &Self::Target {
        if !self.locked {
            panic!("dereferencing unlocked ReloadableDb");
        }
        &self.db
    }
}
