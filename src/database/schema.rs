use std::collections::BTreeMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::args::CategoryMatch;

#[derive(Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct Info {
    pub(super) goals: BTreeMap<Arc<str>, u64>, // BTreeMap to ensure categories are sorted alphabetically
    pub(super) categories: Vec<Arc<str>>,
}

impl Info {
    pub fn goals(&self) -> &BTreeMap<Arc<str>, u64> {
        &self.goals
    }

    pub fn goals_mut(&mut self) -> &mut BTreeMap<Arc<str>, u64> {
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

    pub fn remove_categories(&mut self, cm: &CategoryMatch) -> Vec<Arc<str>> {
        let mut removed: Vec<Arc<str>> = Vec::new();

        self.goals.retain(|k, _| {
            if cm.matches(k) {
                removed.push(k.clone());
                false
            } else {
                true
            }
        });

        self.categories.retain(|c| {
            if cm.matches(c) {
                if !removed.contains(c) {
                    removed.push(c.clone());
                }
                false
            } else {
                true
            }
        });

        removed
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    /// Category of the entry
    pub category: Arc<str>,
    /// UTC timestamp for when the Entry starts
    pub start_time: u64,
    /// UTC timestamp for when the Entry ends
    pub end_time: u64,
}
