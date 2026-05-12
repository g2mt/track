use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
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
pub struct Entry {
    pub category: Arc<str>,
    pub start_time: u64,
    pub end_time: u64,
}
