#![deny(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::unreadable_literal)]
#![doc = include_str!("../README.md")]

/// Occurrences of this rule, and iterators to handle them.
pub mod occurrences;

use bitflags::bitflags;
use chrono::NaiveDate;
use getset::{Getters, Setters};
use occurrences::Iter;

/// A recurrence rule
#[derive(Getters, Setters, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[getset(get = "pub")]
pub struct RecurrenceRule {
    /// This rule is not valid before the specified [`NaiveDate`] (inclusive)
    not_before: NaiveDate,

    /// This rule is not valid after the specified [`NaiveDate`]
    not_after: Option<NaiveDate>,

    /// The maximum number of occurrences for this rule, or unlimited if [`None`].
    #[getset(set = "pub")]
    max_occurrences: Option<u64>,

    /// At what frequency should this rule reoccur?
    #[getset(set = "pub")]
    frequency: Frequency,

    /// A bitflag day filter, xMTWTFSS, 01111111 means this can be paid any day.
    #[getset(set = "pub")]
    day_filter: DayFilter,

    /// If the bitflag day filter cannot be met, should it be resolved into the future or the past?
    #[getset(set = "pub")]
    resolve: ResolveDirection,
}

impl RecurrenceRule {
    /// Create a new recurrence rule.
    #[must_use]
    pub fn new(frequency: Frequency, not_before: NaiveDate) -> Self {
        Self {
            not_before,
            not_after: None,
            max_occurrences: None,
            frequency,
            day_filter: DayFilter::EVERYDAY,
            resolve: ResolveDirection::IntoFuture,
        }
    }

    /// Set the date at which this rule is not valid before. Will return false
    /// if it wasn't able to set due to the provided date being after the
    /// `not_after` date.
    #[must_use = "If you do not need to validate the checking, use `set_not_before_unchecked"]
    pub fn set_not_before(&mut self, not_before: NaiveDate) -> bool {
        if let Some(not_after) = &self.not_after {
            if &not_before > not_after {
                return false;
            }
        }
        self.set_not_before_unchecked(not_before);
        true
    }

    /// Set the date at which this rule is not valid before.
    ///
    /// Preferably, you should use [`Self::set_not_before`].
    pub fn set_not_before_unchecked(&mut self, not_before: NaiveDate) {
        self.not_before = not_before;
    }

    /// Set the date at which this rule is not valid before. Will return false
    /// if it wasn't able to set due to the provided date being after the
    /// `not_after` date.
    #[must_use = "If you do not need to validate the checking, use `set_not_after_unchecked"]
    pub fn set_not_after(&mut self, not_after: Option<NaiveDate>) -> bool {
        if let Some(not_after_dt) = &not_after {
            if not_after_dt < &self.not_before {
                return false;
            }
        }
        self.set_not_after_unchecked(not_after);
        true
    }

    /// Set the date at which this rule is not valid after.
    ///
    /// Preferably, you should use [`Self::set_not_after`].
    pub fn set_not_after_unchecked(&mut self, not_after: Option<NaiveDate>) {
        self.not_after = not_after;
    }

    /// Create an iterator for all occurrences, starting at the provided `start_point` (exclusive).
    #[must_use]
    pub fn iter_after(&self, start_point: &NaiveDate) -> Iter {
        Iter {
            currently_at: *start_point,
            index: 0,
            rule: self,
        }
    }

    /// Create an iterator for all occurrences, starting at [`Self::not_before`] (inclusive).
    ///
    /// ## Panics
    ///
    /// This will panic if [`Self::not_before`] is the beginning of time.
    #[must_use]
    pub fn iter(&self) -> Iter {
        self.iter_after(
            &self
                .not_before()
                .checked_sub_days(chrono::Days::new(1))
                .unwrap(),
        )
    }
}

impl<'a> IntoIterator for &'a RecurrenceRule {
    type Item = occurrences::Occurrence;
    type IntoIter = occurrences::Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// The frequency of a recurring rule
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Frequency {
    /// Reoccur weekly
    Weekly {
        /// Which days should this be taken on
        days: DayFilter,
    },
    /// Reoccur monthly
    Monthly {
        /// The date of the month (1-indexed)
        date: u8,
    },
    /// Reoccur yearly
    Yearly {
        /// The date of the month (1-indexed)
        date: u8,
        /// The month of the year (1-indexed)
        month: u8,
    },
}

/// If the desired day cannot be resolved, should we find the next day into the
/// future that satisfies the requirement, or the day into the past.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ResolveDirection {
    /// Resolve by looking forward into the future for the next valid day
    IntoFuture,
    /// Resolve by looking back into the past for the next valid day
    IntoPast,
}

bitflags! {
    /// Filter by days of the week
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct DayFilter: u8 {
        /// Monday
        const MONDAY = 0b1000000;
        /// Tuesday
        const TUESDAY = 0b0100000;
        /// Wednesday
        const WEDNESDAY = 0b0010000;
        /// Thursday
        const THURSDAY = 0b0001000;
        /// Friday
        const FRIDAY = 0b0000100;
        /// Saturday
        const SATURDAY = 0b0000010;
        /// Sunday
        const SUNDAY = 0b0000001;

        /// Every day of the week. Equivalent to `ANYDAY`.
        const EVERYDAY = Self::MONDAY.bits()
                        | Self::TUESDAY.bits()
                        | Self::WEDNESDAY.bits()
                        | Self::THURSDAY.bits()
                        | Self::FRIDAY.bits()
                        | Self::SATURDAY.bits()
                        | Self::SUNDAY.bits();
        /// Any day of the week. Equivalent to `EVERYDAY`.
        const ANYDAY = Self::EVERYDAY.bits();
        /// Only weekdays
        const WEEKDAYS = Self::MONDAY.bits()
                        | Self::TUESDAY.bits()
                        | Self::WEDNESDAY.bits()
                        | Self::THURSDAY.bits()
                        | Self::FRIDAY.bits();
        /// Only weekends
        const WEEKENDS = Self::SATURDAY.bits()
                        | Self::SUNDAY.bits();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_before_checking() {
        let mut rule = RecurrenceRule::new(
            Frequency::Monthly { date: 1 },
            NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
        );
        rule.set_not_after_unchecked(Some(NaiveDate::from_ymd_opt(2000, 1, 1).unwrap()));
        let allowed = rule.set_not_before(NaiveDate::from_ymd_opt(2001, 1, 1).unwrap());
        assert!(!allowed);
        let allowed = rule.set_not_before(NaiveDate::from_ymd_opt(1999, 1, 1).unwrap());
        assert!(allowed);
    }

    #[test]
    fn test_not_after_checking() {
        let mut rule = RecurrenceRule::new(
            Frequency::Monthly { date: 1 },
            NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
        );
        let allowed = rule.set_not_after(Some(NaiveDate::from_ymd_opt(1999, 1, 1).unwrap()));
        assert!(!allowed);
        let allowed = rule.set_not_after(Some(NaiveDate::from_ymd_opt(2001, 1, 1).unwrap()));
        assert!(allowed);
    }
}
