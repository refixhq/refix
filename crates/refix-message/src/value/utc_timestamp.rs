use super::TimePrecision;
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

/// A FIX UTCTimestamp as nanoseconds since the Unix epoch, keeping its precision.
///
/// Equality and ordering compare the instant and ignore the precision.
#[derive(Clone, Copy, Debug)]
pub struct UtcTimestamp {
    unix_nanos: i64,
    precision: TimePrecision,
}

impl UtcTimestamp {
    /// The instant `unix_nanos` after the Unix epoch, written to `precision`.
    pub const fn new(unix_nanos: i64, precision: TimePrecision) -> Self {
        Self {
            unix_nanos,
            precision,
        }
    }

    /// Nanoseconds since 1970-01-01 00:00:00 UTC.
    pub const fn unix_nanos(&self) -> i64 {
        self.unix_nanos
    }

    /// The precision the value was written to, such as `MILLIS` for `.123`.
    pub const fn precision(&self) -> TimePrecision {
        self.precision
    }

    /// Reads a FIX timestamp such as `20261030-14:29:51.123`.
    pub(crate) fn parse(text: &[u8]) -> Option<Self> {
        let (date_time, rest) = text.split_at_checked(17)?;
        let (precision, fraction) = match rest {
            [] => (TimePrecision::SECONDS, 0),
            [b'.', digits @ ..] if !digits.is_empty() => {
                let count = u8::try_from(digits.len()).ok()?;
                let precision = TimePrecision::from_fraction_digits(count)?;
                (precision, number(digits)?)
            }
            _ => return None,
        };
        let nanos = i128::from(seconds_since_epoch(date_time)?) * 1_000_000_000
            + i128::from(fraction * precision.unit_nanos());
        Some(Self::new(i64::try_from(nanos).ok()?, precision))
    }
}

impl PartialEq for UtcTimestamp {
    fn eq(&self, other: &Self) -> bool {
        self.unix_nanos == other.unix_nanos
    }
}

impl Eq for UtcTimestamp {}

impl Hash for UtcTimestamp {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.unix_nanos.hash(state);
    }
}

impl PartialOrd for UtcTimestamp {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for UtcTimestamp {
    fn cmp(&self, other: &Self) -> Ordering {
        self.unix_nanos.cmp(&other.unix_nanos)
    }
}

/// Seconds from the epoch to a `YYYYMMDD-HH:MM:SS` date and time.
///
/// A leap second, `:60`, counts as the first second of the next minute.
fn seconds_since_epoch(text: &[u8]) -> Option<i64> {
    if text.len() != 17 || text[8] != b'-' || text[11] != b':' || text[14] != b':' {
        return None;
    }
    let year = number(&text[0..4])?;
    let month = number(&text[4..6])?;
    let day = number(&text[6..8])?;
    let hour = number(&text[9..11])?;
    let minute = number(&text[12..14])?;
    let second = number(&text[15..17])?;
    let valid = (1..=12).contains(&month)
        && (1..=days_in_month(year, month)).contains(&day)
        && hour <= 23
        && minute <= 59
        && second <= 60;
    if !valid {
        return None;
    }
    let days = days_from_civil(year, month, day);
    Some(days * 86_400 + hour * 3_600 + minute * 60 + second)
}

/// The value of `digits`, if they're all ASCII digits.
fn number(digits: &[u8]) -> Option<i64> {
    digits.iter().try_fold(0, |value, &byte| {
        byte.is_ascii_digit()
            .then(|| value * 10 + i64::from(byte - b'0'))
    })
}

/// Days from 1970-01-01 to a date, by Howard Hinnant's `days_from_civil`.
///
/// See <https://howardhinnant.github.io/date_algorithms.html#days_from_civil>.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year.rem_euclid(400);
    let month_from_march = (month + 9) % 12;
    let day_of_year = (153 * month_from_march + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    // An era is 146,097 days, and the epoch is day 719,468 from 0000-03-01.
    era * 146_097 + day_of_era - 719_468
}

/// The number of days in `month` of `year`.
fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        2 if is_leap_year(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

fn is_leap_year(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Option<UtcTimestamp> {
        UtcTimestamp::parse(text.as_bytes())
    }

    fn parts(text: &str) -> (i64, TimePrecision) {
        let value = parse(text).unwrap();
        (value.unix_nanos(), value.precision())
    }

    mod parsing {
        use super::*;

        #[test]
        fn reads_whole_seconds() {
            assert_eq!(
                parts("20261030-14:29:51"),
                (1_793_370_591_000_000_000, TimePrecision::SECONDS)
            );
        }

        #[test]
        fn reads_the_common_precisions() {
            assert_eq!(
                parts("20261030-14:29:51.123"),
                (1_793_370_591_123_000_000, TimePrecision::MILLIS)
            );
            assert_eq!(
                parts("20261030-14:29:51.123456"),
                (1_793_370_591_123_456_000, TimePrecision::MICROS)
            );
            assert_eq!(
                parts("20261030-14:29:51.123456789"),
                (1_793_370_591_123_456_789, TimePrecision::NANOS)
            );
        }

        #[test]
        fn reads_any_fraction_up_to_nine_digits() {
            for (fraction, nanos) in [
                ("1", 100_000_000),
                ("1234", 123_400_000),
                ("1234567", 123_456_700),
            ] {
                let (unix_nanos, precision) = parts(&format!("20261030-14:29:51.{fraction}"));
                assert_eq!(unix_nanos, 1_793_370_591_000_000_000 + nanos, "{fraction}");
                assert_eq!(usize::from(precision.fraction_digits()), fraction.len());
            }
        }

        #[test]
        fn keeps_the_precision_of_a_zero_fraction() {
            assert_eq!(
                parts("20261030-14:29:51.000"),
                (1_793_370_591_000_000_000, TimePrecision::MILLIS)
            );
        }

        #[test]
        fn reads_times_around_the_epoch() {
            assert_eq!(parts("19700101-00:00:00").0, 0);
            assert_eq!(parts("19691231-23:59:59.999").0, -1_000_000);
        }

        #[test]
        fn folds_a_leap_second_into_the_next_day() {
            assert_eq!(parts("20161231-23:59:60").0, parts("20170101-00:00:00").0);
        }

        #[test]
        fn accepts_february_29_only_in_leap_years() {
            assert!(parse("20240229-00:00:00").is_some());
            assert!(parse("20000229-00:00:00").is_some());
            assert_eq!(parse("20230229-00:00:00"), None);
            assert_eq!(parse("19000229-00:00:00"), None);
        }

        #[test]
        fn rejects_parts_out_of_range() {
            for text in [
                "20260001-00:00:00",
                "20261301-00:00:00",
                "20261000-00:00:00",
                "20261032-00:00:00",
                "20260431-00:00:00",
                "20261030-24:00:00",
                "20261030-14:60:00",
                "20261030-14:29:61",
            ] {
                assert_eq!(parse(text), None, "{text:?}");
            }
        }

        #[test]
        fn rejects_an_empty_or_sub_nanosecond_fraction() {
            for fraction in [".", ".1234567890", ".123456789012"] {
                let text = format!("20261030-14:29:51{fraction}");
                assert_eq!(parse(&text), None, "{text:?}");
            }
        }

        #[test]
        fn rejects_other_shapes() {
            for text in [
                "",
                "20261030",
                "20261030-14:29",
                "2026-10-30T14:29:51",
                "20261030T14:29:51",
                "20261030-14.29.51",
                "20261030-14:29:51Z",
                "20261030-14:29:51,123",
                "20261030-14:29:5a",
                "+0261030-14:29:51",
                "20261030-14:29:51.12a",
            ] {
                assert_eq!(parse(text), None, "{text:?}");
            }
        }

        #[test]
        fn reads_the_whole_i64_range() {
            assert_eq!(parts("22620411-23:47:16.854775807").0, i64::MAX);
            assert_eq!(parts("16770921-00:12:43.145224192").0, i64::MIN);
        }

        #[test]
        fn rejects_instants_beyond_i64() {
            for text in [
                "22620411-23:47:16.854775808",
                "16770921-00:12:43.145224191",
                "99991231-23:59:59",
                "00000101-00:00:00",
            ] {
                assert_eq!(parse(text), None, "{text:?}");
            }
        }
    }

    mod equality {
        use super::*;
        use std::hash::DefaultHasher;

        fn hash_of(value: UtcTimestamp) -> u64 {
            let mut hasher = DefaultHasher::new();
            value.hash(&mut hasher);
            hasher.finish()
        }

        #[test]
        fn compares_instants_across_precisions() {
            assert_eq!(
                parse("20261030-14:29:51"),
                parse("20261030-14:29:51.000000")
            );
            assert_ne!(parse("20261030-14:29:51"), parse("20261030-14:29:51.001"));
        }

        #[test]
        fn equal_instants_hash_alike() {
            assert_eq!(
                hash_of(UtcTimestamp::new(1_000, TimePrecision::MICROS)),
                hash_of(UtcTimestamp::new(1_000, TimePrecision::NANOS))
            );
        }

        #[test]
        fn orders_by_instant() {
            assert!(
                UtcTimestamp::new(1, TimePrecision::NANOS)
                    < UtcTimestamp::new(1_000_000_000, TimePrecision::SECONDS)
            );
        }
    }

    mod calendar {
        use super::*;

        #[test]
        fn the_epoch_is_day_zero() {
            assert_eq!(days_from_civil(1970, 1, 1), 0);
            assert_eq!(days_from_civil(1969, 12, 31), -1);
        }

        #[test]
        fn counts_days_from_the_epoch() {
            assert_eq!(days_from_civil(2000, 3, 1), 11_017);
            assert_eq!(days_from_civil(2026, 10, 30), 20_756);
        }

        #[test]
        fn consecutive_dates_are_consecutive_days() {
            for year in 1600..=2400 {
                for month in 1..=12 {
                    let last = days_from_civil(year, month, days_in_month(year, month));
                    let next = match month {
                        12 => days_from_civil(year + 1, 1, 1),
                        _ => days_from_civil(year, month + 1, 1),
                    };
                    assert_eq!(last + 1, next, "{year}-{month}");
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
