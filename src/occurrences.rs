use chrono::{Datelike, Days, Months, NaiveDate, Weekday};
use getset::Getters;

use crate::{DayFilter, Frequency, RecurrenceRule, ResolveDirection};

/// An individual occurrence of a recurrence rule.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Getters)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[getset(get = "pub")]
pub struct Occurrence {
    /// When is this specific occurrence at?
    at: NaiveDate,
}

/// An iterator over occurrences of a [`RecurrenceRule`].
///
/// > **Warning! This iterator may be infinite!**
pub struct Iter<'a> {
    pub(super) currently_at: NaiveDate,
    pub(super) index: u64,
    pub(super) rule: &'a RecurrenceRule,
}

impl Iterator for Iter<'_> {
    type Item = Occurrence;

    /// Get the next occurrence of this iterator.
    ///
    /// ## Panics
    ///
    /// This will panic if the date or month defined in the rule is invalid.
    fn next(&mut self) -> Option<Self::Item> {
        if self
            .rule
            .not_after()
            .is_some_and(|not_after| self.currently_at > not_after)
        {
            // Past end date, no more occurrences
            return None;
        }
        if self
            .rule
            .max_occurrences()
            .is_some_and(|max_occurrences| self.index >= max_occurrences)
        {
            // Past max occurrences, no more occurrences
            return None;
        }

        // Establish next date
        let mut next_date = match self.rule.frequency() {
            Frequency::Weekly { days } => {
                let mut next_date = self.currently_at;

                // Move forward at least 1 day
                next_date = next_date.checked_add_days(Days::new(1))?;

                // Move forward until in date filter
                while !is_weekday_in_filter(next_date.weekday(), *days) {
                    next_date = next_date.checked_add_days(Days::new(1))?;
                }

                next_date
            }
            Frequency::Monthly { date } => {
                let mut next_date = self.currently_at;

                // Establish if the next occurrence is this year or next
                let need_next_month = (self.currently_at.day0() + 1) >= u32::from(*date);
                if need_next_month {
                    next_date = next_date.checked_add_months(Months::new(1))?;
                }

                next_date
                    .with_day(u32::from(*date))
                    .expect("invalid rule: rule contains invalid date")
            }
            Frequency::Yearly { date, month } => {
                let (mut next_date_year, next_date_month, next_date_day) = (
                    self.currently_at.year(),
                    u32::from(*month),
                    u32::from(*date),
                );

                // Establish if the next occurrence is this year or next
                let need_next_year = match (self.currently_at.month0() + 1).cmp(&u32::from(*month))
                {
                    std::cmp::Ordering::Greater => true,
                    std::cmp::Ordering::Equal => (self.currently_at.day0() + 1) >= u32::from(*date),
                    std::cmp::Ordering::Less => false,
                };
                if need_next_year {
                    next_date_year += 1;
                }

                // Set month and date
                NaiveDate::from_ymd_opt(next_date_year, next_date_month, next_date_day)
                    .expect("invalid rule: rule contains invalid value")
            }
        };

        // Check if meets day filter
        if !is_weekday_in_filter(next_date.weekday(), *self.rule.day_filter()) {
            // If not, resolve in appropriate direction
            let direction = *self.rule.resolve();
            while !is_weekday_in_filter(next_date.weekday(), *self.rule.day_filter()) {
                match direction {
                    ResolveDirection::IntoFuture => {
                        next_date = next_date.checked_add_days(Days::new(1))?;
                    }
                    ResolveDirection::IntoPast => {
                        next_date = next_date.checked_sub_days(Days::new(1))?;
                    }
                }
            }
        }

        let occ = Occurrence { at: next_date };
        self.index += 1;
        self.currently_at = next_date;
        Some(occ)
    }
}

fn is_weekday_in_filter(weekday: Weekday, filter: DayFilter) -> bool {
    match weekday {
        Weekday::Mon => filter.contains(DayFilter::MONDAY),
        Weekday::Tue => filter.contains(DayFilter::TUESDAY),
        Weekday::Wed => filter.contains(DayFilter::WEDNESDAY),
        Weekday::Thu => filter.contains(DayFilter::THURSDAY),
        Weekday::Fri => filter.contains(DayFilter::FRIDAY),
        Weekday::Sat => filter.contains(DayFilter::SATURDAY),
        Weekday::Sun => filter.contains(DayFilter::SUNDAY),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_next_yearly() {
        let not_before = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let mut iter = Iter {
            index: 0,
            currently_at: not_before.checked_sub_days(Days::new(1)).unwrap(),
            rule: &RecurrenceRule {
                not_before,
                not_after: None,
                max_occurrences: None,
                frequency: Frequency::Yearly { date: 1, month: 4 },
                day_filter: crate::DayFilter::WEEKDAYS,
                resolve: crate::ResolveDirection::IntoFuture,
            },
        };

        assert_eq!(
            iter.next(),
            Some(Occurrence {
                at: NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
            })
        );
        assert_eq!(
            iter.next(),
            Some(Occurrence {
                at: NaiveDate::from_ymd_opt(2026, 4, 1).unwrap(),
            })
        );
        assert_eq!(
            iter.next(),
            Some(Occurrence {
                at: NaiveDate::from_ymd_opt(2027, 4, 1).unwrap(),
            })
        );
        // Resolves forward to Monday in 2028.
        assert_eq!(
            iter.next(),
            Some(Occurrence {
                at: NaiveDate::from_ymd_opt(2028, 4, 3).unwrap(),
            })
        );
    }

    #[test]
    fn resolve_next_monthly() {
        let not_before = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let mut iter = Iter {
            index: 0,
            currently_at: not_before.checked_sub_days(Days::new(1)).unwrap(),
            rule: &RecurrenceRule {
                not_before,
                not_after: None,
                max_occurrences: None,
                frequency: Frequency::Monthly { date: 1 },
                day_filter: crate::DayFilter::WEEKDAYS,
                resolve: crate::ResolveDirection::IntoFuture,
            },
        };

        assert_eq!(
            iter.next(),
            Some(Occurrence {
                at: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            })
        );
        // Resolves forward to Monday in Feb.
        assert_eq!(
            iter.next(),
            Some(Occurrence {
                at: NaiveDate::from_ymd_opt(2025, 2, 3).unwrap(),
            })
        );
        // Resolves forward to Monday in Feb.
        assert_eq!(
            iter.next(),
            Some(Occurrence {
                at: NaiveDate::from_ymd_opt(2025, 3, 3).unwrap(),
            })
        );
        assert_eq!(
            iter.next(),
            Some(Occurrence {
                at: NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
            })
        );
    }

    #[test]
    fn resolve_next_weekly() {
        let not_before = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let mut iter = Iter {
            index: 0,
            currently_at: not_before.checked_sub_days(Days::new(1)).unwrap(),
            rule: &RecurrenceRule {
                not_before,
                not_after: None,
                max_occurrences: None,
                frequency: Frequency::Weekly {
                    days: DayFilter::WEEKENDS,
                },
                day_filter: crate::DayFilter::ANYDAY,
                resolve: crate::ResolveDirection::IntoFuture,
            },
        };

        assert_eq!(
            iter.next(),
            Some(Occurrence {
                at: NaiveDate::from_ymd_opt(2025, 1, 4).unwrap(),
            })
        );
        assert_eq!(
            iter.next(),
            Some(Occurrence {
                at: NaiveDate::from_ymd_opt(2025, 1, 5).unwrap(),
            })
        );
        assert_eq!(
            iter.next(),
            Some(Occurrence {
                at: NaiveDate::from_ymd_opt(2025, 1, 11).unwrap(),
            })
        );
        assert_eq!(
            iter.next(),
            Some(Occurrence {
                at: NaiveDate::from_ymd_opt(2025, 1, 12).unwrap(),
            })
        );
    }
}
