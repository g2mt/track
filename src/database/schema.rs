use std::collections::BTreeMap;
use std::num::NonZeroU64;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use time::Weekday;

use crate::args::CategoryMatch;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Frequency {
    Day,
    Hour,
    DayOfWeek(Weekday),
    DayOfMonth(u8),
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CategoryData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub goal: Option<NonZeroU64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notify_every: Option<Frequency>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_notification: Option<NonZeroU64>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct Info {
    /// Categories stored in the database
    pub(super) categories: BTreeMap<Arc<str>, CategoryData>,
    /// Test-only padding data to control serialized line length
    #[cfg(test)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_data: Option<Arc<str>>,
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

    /// Returns the number of categories.
    pub fn len(&self) -> usize {
        self.categories.len()
    }

    /// Returns a reference to the data for a given category, or `None`.
    pub fn data(&self, category: &str) -> Option<&CategoryData> {
        self.categories.get(category)
    }

    /// Returns a mutable reference to the data for a given category, or `None`.
    pub fn data_mut(&mut self, category: &str) -> Option<&mut CategoryData> {
        self.categories.get_mut(category)
    }

    /// Adds a category with no goal set, does not overwite older CategoryData.
    /// Returns a mutable reference to CategoryData.
    pub fn add_category(&mut self, category: Arc<str>) -> &mut CategoryData {
        use std::collections::btree_map::Entry;
        let e = self.categories.entry(category);
        match e {
            Entry::Vacant(_) => e.or_insert(CategoryData::default()),
            Entry::Occupied(occ) => occ.into_mut(),
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
