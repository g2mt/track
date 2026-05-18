mod base;
pub mod iter;
pub mod range;
pub mod schema;

#[cfg(test)]
mod tests;

use std::error::Error;
use std::fmt::Display;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::sync::Arc;

pub use base::Database;
use parking_lot::{MappedMutexGuard, Mutex, MutexGuard};
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

pub trait MainDatabase: Deref<Target = Database<FileWithPath>> {
    fn new(file: FileWithPath) -> Self;
}

pub struct SingleThreadedDb {
    db: Database<FileWithPath>,
}

impl MainDatabase for SingleThreadedDb {
    fn new(file: FileWithPath) -> Self {
        Self {
            db: Database::new(file),
        }
    }
}

impl DerefMut for SingleThreadedDb {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.db
    }
}

impl Deref for SingleThreadedDb {
    type Target = Database<FileWithPath>;

    fn deref(&self) -> &Self::Target {
        &self.db
    }
}

enum MultiThreadedDbState {
    Loaded(Database<FileWithPath>),
    Unloaded((PathBuf, std::fs::OpenOptions)),
    Indeterminate,
}

pub struct MultiThreadedDb {
    state: Mutex<MultiThreadedDbState>,
}

#[derive(Debug)]
pub struct DatabaseChanged;

impl Display for DatabaseChanged {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("database changed, reload is needed")
    }
}

impl Error for DatabaseChanged {}

impl MultiThreadedDb {
    pub fn lock<'a>(
        &'a self,
    ) -> Result<MappedMutexGuard<'a, Database<FileWithPath>>, DatabaseChanged> {
        let guard = self.state.lock();
        MutexGuard::try_map(guard, |state| {
            if let MultiThreadedDbState::Loaded(loaded) = state {
                Some(loaded)
            } else {
                None
            }
        })
        .or(Err(DatabaseChanged {}))
    }

    pub fn reload<'a>(&'a self) -> std::io::Result<MappedMutexGuard<'a, Database<FileWithPath>>> {
        let guard = self.state.lock();
        MutexGuard::try_map_or_err(guard, |state| {
            if let MultiThreadedDbState::Unloaded((path, options)) =
                std::mem::replace(state, MultiThreadedDbState::Indeterminate)
            {
                let backing = FileWithPath::open(path, options)?;
                *state = MultiThreadedDbState::Loaded(Database::new(backing));
                if let MultiThreadedDbState::Loaded(db) = state {
                    Ok(db)
                } else {
                    unreachable!()
                }
            } else {
                panic!("reloading presently loaded");
            }
        })
        .map_err(|(_, e)| e)
    }

    pub fn take_if_changed(&self) -> bool {
        let mut state = self.state.lock();
        match std::mem::replace(&mut *state, MultiThreadedDbState::Indeterminate) {
            MultiThreadedDbState::Loaded(db) => {
                if db.backing.changed() {
                    *state = MultiThreadedDbState::Unloaded(db.backing.into_open_args());
                    return true;
                }
            }
            other => {
                *state = other;
            }
        }
        false
    }
}

impl Into<Arc<MultiThreadedDb>> for SingleThreadedDb {
    fn into(self) -> Arc<MultiThreadedDb> {
        Arc::new(MultiThreadedDb {
            state: Mutex::new(MultiThreadedDbState::Loaded(self.db)),
        })
    }
}
