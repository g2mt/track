use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Info {
    pub(super) goals: HashMap<Arc<str>, u64>,
    pub(super) categories: Vec<Arc<str>>,
}

impl Info {
    pub fn goals(&self) -> &HashMap<Arc<str>, u64> {
        &self.goals
    }

    pub fn goals_mut(&mut self) -> &mut HashMap<Arc<str>, u64> {
        &mut self.goals
    }

    pub fn categories(&self) -> &[Arc<str>] {
        &self.categories
    }

    pub fn add_category(&mut self, category: Arc<str>) -> bool {
        if self.categories.contains(&category) {
            false
        } else {
            self.categories.push(category);
            true
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Entry<'info> {
    pub(super) category: &'info str,
    pub(super) start_time: u64,
    pub(super) end_time: u64,
}

impl<'info> Entry<'info> {
    pub fn category(&self) -> &'info str {
        self.category
    }

    pub fn start_time(&self) -> u64 {
        self.start_time
    }

    pub fn end_time(&self) -> u64 {
        self.end_time
    }
}
