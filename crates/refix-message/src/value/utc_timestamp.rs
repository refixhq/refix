use super::{Date, Time, TimePrecision};
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
        let (date, rest) = text.split_at_checked(8)?;
        let date = Date::parse(date)?;
        let time = Time::parse(rest.strip_prefix(b"-")?)?;
        let nanos = i128::from(date.days_since_epoch()) * 86_400_000_000_000
            + i128::from(time.nanos_since_midnight());
        Some(Self::new(i64::try_from(nanos).ok()?, time.precision()))
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
        fn reads_times_around_the_epoch() {
            assert_eq!(parts("19700101-00:00:00").0, 0);
            assert_eq!(parts("19691231-23:59:59.999").0, -1_000_000);
        }

        #[test]
        fn folds_a_leap_second_into_the_next_day() {
            assert_eq!(parts("20161231-23:59:60").0, parts("20170101-00:00:00").0);
        }

        #[test]
        fn rejects_a_date_that_does_not_exist() {
            assert_eq!(parse("20230229-00:00:00"), None);
        }

        #[test]
        fn rejects_a_time_that_does_not_exist() {
            assert_eq!(parse("20261030-24:00:00"), None);
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
}
