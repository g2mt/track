mod base;
pub mod iter;
pub mod range;
pub mod schema;

#[cfg(test)]
mod tests;

pub use base::Database;
pub use schema::{CategoryData, Entry, Frequency, Info};

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

pub type MainDatabase = Database<crate::utils::io::FileWithPath>;
