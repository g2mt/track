use std::io::{Read, Seek};
use std::ops::{Bound, RangeBounds};

use anyhow::Result;
use time::OffsetDateTime;

use super::{Entry, Span, iter};

pub struct LatestRange<'a, Backing: Seek + Read, R: RangeBounds<OffsetDateTime>> {
    iter: iter::Iter<'a, Backing>,
    range: R,
}

impl<'a, Backing: Seek + Read, R: RangeBounds<OffsetDateTime>> LatestRange<'a, Backing, R> {
    pub(super) fn new(backing: &'a mut Backing, range: R) -> Self {
        Self {
            iter: iter::Iter::new(backing),
            range,
        }
    }
}

fn bound_ts(bound: Bound<&OffsetDateTime>, lower: bool) -> Option<u64> {
    match bound {
        Bound::Included(dt) => Some(dt.unix_timestamp() as u64),
        Bound::Excluded(dt) => {
            let ts = dt.unix_timestamp() as u64;
            if lower {
                // For the lower bound, Excluded means we start *after* dt
                ts.checked_add(1)
            } else {
                // For the upper bound, Excluded means we end *before* dt
                ts.checked_sub(1)
            }
        }
        Bound::Unbounded => None,
    }
}

impl<'a, Backing: Seek + Read, R: RangeBounds<OffsetDateTime>> Iterator
    for LatestRange<'a, Backing, R>
{
    type Item = Result<(Span, Entry)>;

    fn next(&mut self) -> Option<Self::Item> {
        let from_ts = bound_ts(self.range.start_bound(), true);
        let to_ts = bound_ts(self.range.end_bound(), false);

        loop {
            let (span, entry) = match self.iter.next_back() {
                Some(Ok(item)) => item,
                Some(Err(e)) => return Some(Err(e)),
                None => return None,
            };

            if let Some(from) = from_ts {
                if entry.start_time < from {
                    return None;
                }
            }
            if let Some(to) = to_ts {
                if entry.start_time > to {
                    continue;
                }
            }

            return Some(Ok((span, entry)));
        }
    }
}
