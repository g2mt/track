use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Info {
    pub(super) goals: HashMap<String, u64>,
    pub(super) categories: Vec<String>,
}

impl Info {
    pub fn goals(&self) -> &HashMap<String, u64> {
        &self.goals
    }

    pub fn goals_mut(&mut self) -> &mut HashMap<String, u64> {
        &mut self.goals
    }

    pub fn categories(&self) -> &[String] {
        &self.categories
    }

    pub fn add_category(&mut self, category: String) -> bool {
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
