use std::collections::BTreeMap;
use std::num::NonZeroU64;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::args::CategoryMatch;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[skip_serializing_none]
pub struct CategoryData {
    pub goal: Option<NonZeroU64>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct Info {
    pub(super) categories: BTreeMap<Arc<str>, CategoryData>, // BTreeMap to ensure categories are sorted alphabetically
}

impl Info {
    /// Returns an iterator over (category_name, category_data) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&Arc<str>, &CategoryData)> {
        self.categories.iter()
    }

    /// Returns an iterator over category names.
    pub fn keys(&self) -> impl Iterator<Item = &Arc<str>> {
        self.categories.keys()
    }

    /// Returns a reference to the data for a given category, or `None`.
    pub fn data(&self, category: &str) -> Option<&CategoryData> {
        self.categories.get(category)
    }

    /// Returns a mutable reference to the data for a given category, or `None`.
    pub fn data_mut(&mut self, category: &str) -> Option<&mut CategoryData> {
        self.categories.get_mut(category)
    }

    /// Inserts or replaces the data for a category.
    pub fn add_data(&mut self, category: Arc<str>, data: CategoryData) {
        self.categories.insert(category, data);
    }

    /// Adds a category with no goal set. Returns `true` if the category was newly added.
    pub fn add_category(&mut self, category: Arc<str>) -> bool {
        use std::collections::btree_map::Entry;
        match self.categories.entry(category) {
            Entry::Vacant(e) => {
                e.insert(CategoryData { goal: None });
                true
            }
            Entry::Occupied(_) => false,
        }
    }

    /// Removes categories matching the given pattern, returning the removed names.
    pub fn remove_categories(&mut self, cm: &CategoryMatch) -> Vec<Arc<str>> {
        let mut removed: Vec<Arc<str>> = Vec::new();
        self.categories.retain(|k, _| {
            if cm.matches(k) {
                removed.push(k.clone());
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
