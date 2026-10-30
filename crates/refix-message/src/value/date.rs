use super::number;

/// A FIX date, as in a UTCDateOnly or LocalMktDate field.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Date {
    year: u16,
    month: u8,
    day: u8,
}

impl Date {
    /// The date `year`-`month`-`day`, if it exists and the year fits FIX's four digits.
    pub const fn new(year: u16, month: u8, day: u8) -> Option<Self> {
        let valid = year <= 9999
            && matches!(month, 1..=12)
            && day >= 1
            && day <= days_in_month(year, month);
        if valid {
            Some(Self { year, month, day })
        } else {
            None
        }
    }

    /// The year, such as `2026`.
    pub const fn year(&self) -> u16 {
        self.year
    }

    /// The month, from `1` for January to `12` for December.
    pub const fn month(&self) -> u8 {
        self.month
    }

    /// The day of the month, from `1`.
    pub const fn day(&self) -> u8 {
        self.day
    }

    /// Reads a FIX date such as `20261030`.
    pub(crate) fn parse(text: &[u8]) -> Option<Self> {
        if text.len() != 8 {
            return None;
        }
        let year = u16::try_from(number(&text[0..4])?).ok()?;
        let month = u8::try_from(number(&text[4..6])?).ok()?;
        let day = u8::try_from(number(&text[6..8])?).ok()?;
        Self::new(year, month, day)
    }

    /// Days from 1970-01-01, by Howard Hinnant's `days_from_civil`.
    ///
    /// See <https://howardhinnant.github.io/date_algorithms.html#days_from_civil>.
    pub(super) fn days_since_epoch(self) -> i64 {
        let month = i64::from(self.month);
        let year = i64::from(self.year);
        let year = if month <= 2 { year - 1 } else { year };
        let era = year.div_euclid(400);
        let year_of_era = year.rem_euclid(400);
        let month_from_march = (month + 9) % 12;
        let day_of_year = (153 * month_from_march + 2) / 5 + i64::from(self.day) - 1;
        let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
        // An era is 146,097 days, and the epoch is day 719,468 from 0000-03-01.
        era * 146_097 + day_of_era - 719_468
    }
}

/// The number of days in `month` of `year`.
const fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        2 if is_leap_year(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

const fn is_leap_year(year: u16) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: u16, month: u8, day: u8) -> Date {
        Date::new(year, month, day).unwrap()
    }

    fn parse(text: &str) -> Option<Date> {
        Date::parse(text.as_bytes())
    }

    mod construction {
        use super::*;

        #[test]
        fn keeps_its_parts() {
            let value = date(2026, 10, 30);
            assert_eq!((value.year(), value.month(), value.day()), (2026, 10, 30));
        }

        #[test]
        fn accepts_february_29_only_in_leap_years() {
            assert!(Date::new(2024, 2, 29).is_some());
            assert!(Date::new(2000, 2, 29).is_some());
            assert_eq!(Date::new(2023, 2, 29), None);
            assert_eq!(Date::new(1900, 2, 29), None);
        }

        #[test]
        fn rejects_days_that_do_not_exist() {
            for (year, month, day) in [
                (2026, 0, 1),
                (2026, 13, 1),
                (2026, 10, 0),
                (2026, 10, 32),
                (2026, 4, 31),
            ] {
                assert_eq!(Date::new(year, month, day), None, "{year}-{month}-{day}");
            }
        }

        #[test]
        fn takes_the_years_fix_can_write() {
            assert!(Date::new(0, 1, 1).is_some());
            assert!(Date::new(9999, 12, 31).is_some());
            assert_eq!(Date::new(10000, 1, 1), None);
        }
    }

    mod parsing {
        use super::*;

        #[test]
        fn reads_a_date() {
            assert_eq!(parse("20261030"), Some(date(2026, 10, 30)));
        }

        #[test]
        fn reads_the_ends_of_the_fix_range() {
            assert_eq!(parse("00000101"), Some(date(0, 1, 1)));
            assert_eq!(parse("99991231"), Some(date(9999, 12, 31)));
        }

        #[test]
        fn rejects_dates_that_do_not_exist() {
            for text in ["20261301", "20261000", "20260431", "20230229"] {
                assert_eq!(parse(text), None, "{text:?}");
            }
        }

        #[test]
        fn rejects_other_shapes() {
            for text in [
                "",
                "2026103",
                "202610301",
                "2026-10-30",
                "2026103a",
                "+0261030",
                "20261030-",
            ] {
                assert_eq!(parse(text), None, "{text:?}");
            }
        }
    }

    mod ordering {
        use super::*;

        #[test]
        fn orders_chronologically() {
            assert!(date(2025, 12, 31) < date(2026, 1, 1));
            assert!(date(2026, 1, 31) < date(2026, 2, 1));
            assert!(date(2026, 2, 1) < date(2026, 2, 2));
        }
    }

    mod calendar {
        use super::*;

        #[test]
        fn the_epoch_is_day_zero() {
            assert_eq!(date(1970, 1, 1).days_since_epoch(), 0);
            assert_eq!(date(1969, 12, 31).days_since_epoch(), -1);
        }

        #[test]
        fn counts_days_from_the_epoch() {
            assert_eq!(date(2000, 3, 1).days_since_epoch(), 11_017);
            assert_eq!(date(2026, 10, 30).days_since_epoch(), 20_756);
        }

        #[test]
        fn consecutive_dates_are_consecutive_days() {
            for year in 1600..=2400 {
                for month in 1..=12 {
                    let last = date(year, month, days_in_month(year, month));
                    let next = match month {
                        12 => date(year + 1, 1, 1),
                        _ => date(year, month + 1, 1),
                    };
                    assert_eq!(
                        last.days_since_epoch() + 1,
                        next.days_since_epoch(),
                        "{year}-{month}"
                    );
                }
            }
        }

        #[test]
        fn february_has_29_days_in_leap_years() {
            assert_eq!(days_in_month(2024, 2), 29);
            assert_eq!(days_in_month(2000, 2), 29);
            assert_eq!(days_in_month(2023, 2), 28);
            assert_eq!(days_in_month(1900, 2), 28);
        }

        #[test]
        fn other_months_have_fixed_lengths() {
            assert_eq!(days_in_month(2026, 1), 31);
            assert_eq!(days_in_month(2026, 4), 30);
            assert_eq!(days_in_month(2026, 12), 31);
        }
    }
}
