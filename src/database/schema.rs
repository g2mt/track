use std::collections::BTreeMap;
use std::num::NonZeroU64;
use std::sync::Arc;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::str::FromStr;
use time::{OffsetDateTime, Time, Weekday};

use crate::args::CategoryMatch;

#[derive(Debug, Clone, PartialEq)]
pub enum Frequency {
    Day,
    Hour,
    DayOfWeek(Weekday),
    DayOfMonth(u8),
}

impl Serialize for Frequency {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Frequency::Day => serializer.serialize_str("day"),
            Frequency::Hour => serializer.serialize_str("hour"),
            Frequency::DayOfWeek(wd) => {
                let s = match wd {
                    Weekday::Monday => "mon",
                    Weekday::Tuesday => "tue",
                    Weekday::Wednesday => "wed",
                    Weekday::Thursday => "thu",
                    Weekday::Friday => "fri",
                    Weekday::Saturday => "sat",
                    Weekday::Sunday => "sun",
                };
                serializer.serialize_str(s)
            }
            Frequency::DayOfMonth(day) => serializer.serialize_str(&day.to_string()),
        }
    }
}

impl<'de> Deserialize<'de> for Frequency {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        FromStr::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl FromStr for Frequency {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "day" => Ok(Frequency::Day),
            "hour" => Ok(Frequency::Hour),
            "mon" => Ok(Frequency::DayOfWeek(Weekday::Monday)),
            "tue" => Ok(Frequency::DayOfWeek(Weekday::Tuesday)),
            "wed" => Ok(Frequency::DayOfWeek(Weekday::Wednesday)),
            "thu" => Ok(Frequency::DayOfWeek(Weekday::Thursday)),
            "fri" => Ok(Frequency::DayOfWeek(Weekday::Friday)),
            "sat" => Ok(Frequency::DayOfWeek(Weekday::Saturday)),
            "sun" => Ok(Frequency::DayOfWeek(Weekday::Sunday)),
            _ => {
                if let Ok(day) = s.parse::<u8>() {
                    if (1..=31).contains(&day) {
                        return Ok(Frequency::DayOfMonth(day));
                    }
                }
                Err(format!(
                    "invalid frequency: '{s}'. expected: day, hour, mon-sun, or 1-31"
                ))
            }
        }
    }
}

impl Frequency {
    /// Compute the next notification datetime after `now` for this frequency.
    pub fn next_date(&self, now: OffsetDateTime) -> OffsetDateTime {
        match self {
            Frequency::Day => {
                let tomorrow = now.date().next_day().unwrap();
                tomorrow
                    .with_time(Time::MIDNIGHT)
                    .assume_offset(now.offset())
            }
            Frequency::Hour => {
                let this_hour = now.truncate_to_hour();
                this_hour.saturating_add(time::Duration::HOUR)
            }
            Frequency::DayOfWeek(weekday) => {
                let target_date = now.date();
                now.replace_time(time::Time::from_hms(0, 0, 0).unwrap())
                    .replace_date(target_date.next_occurrence(*weekday))
            }
            Frequency::DayOfMonth(day) => {
                let mut target_date = now.date().next_day().unwrap();
                while target_date.day() != *day {
                    target_date = target_date.next_day().unwrap();
                }
                now.replace_time(time::Time::from_hms(0, 0, 0).unwrap())
                    .replace_date(target_date)
            }
        }
    }
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

    /// Returns an iterator over (category_name, mutable category_data) pairs.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&Arc<str>, &mut CategoryData)> {
        self.categories.iter_mut()
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
