use super::{TimePrecision, number};
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

/// A FIX time of day, as in a UTCTimeOnly or LocalMktTime field, keeping its precision.
///
/// Equality and ordering compare the time and ignore the precision.
#[derive(Clone, Copy, Debug)]
pub struct Time {
    nanos_since_midnight: i64,
    precision: TimePrecision,
}

impl Time {
    /// The time `nanos_since_midnight` after midnight, written to `precision`.
    ///
    /// `None` past the end of a day, which has 86,401 seconds with a leap second.
    pub const fn new(nanos_since_midnight: i64, precision: TimePrecision) -> Option<Self> {
        if matches!(nanos_since_midnight, 0..86_401_000_000_000) {
            Some(Self {
                nanos_since_midnight,
                precision,
            })
        } else {
            None
        }
    }

    /// Nanoseconds since midnight.
    pub const fn nanos_since_midnight(&self) -> i64 {
        self.nanos_since_midnight
    }

    /// The precision the value was written to, such as `MILLIS` for `.123`.
    pub const fn precision(&self) -> TimePrecision {
        self.precision
    }

    /// Reads a FIX time such as `14:29:51.123`.
    ///
    /// A leap second, `:60`, counts as the first second of the next minute.
    pub(crate) fn parse(text: &[u8]) -> Option<Self> {
        let (time, rest) = text.split_at_checked(8)?;
        if time[2] != b':' || time[5] != b':' {
            return None;
        }
        let hour = number(&time[0..2])?;
        let minute = number(&time[3..5])?;
        let second = number(&time[6..8])?;
        if hour > 23 || minute > 59 || second > 60 {
            return None;
        }
        let (precision, fraction) = match rest {
            [] => (TimePrecision::SECONDS, 0),
            [b'.', digits @ ..] if !digits.is_empty() => {
                let count = u8::try_from(digits.len()).ok()?;
                let precision = TimePrecision::from_fraction_digits(count)?;
                (precision, number(digits)?)
            }
            _ => return None,
        };
        let seconds = hour * 3_600 + minute * 60 + second;
        Self::new(
            seconds * 1_000_000_000 + fraction * precision.unit_nanos(),
            precision,
        )
    }
}

impl PartialEq for Time {
    fn eq(&self, other: &Self) -> bool {
        self.nanos_since_midnight == other.nanos_since_midnight
    }
}

impl Eq for Time {}

impl Hash for Time {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.nanos_since_midnight.hash(state);
    }
}

impl PartialOrd for Time {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Time {
    fn cmp(&self, other: &Self) -> Ordering {
        self.nanos_since_midnight.cmp(&other.nanos_since_midnight)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Option<Time> {
        Time::parse(text.as_bytes())
    }

    fn parts(text: &str) -> (i64, TimePrecision) {
        let value = parse(text).unwrap();
        (value.nanos_since_midnight(), value.precision())
    }

    mod construction {
        use super::*;

        #[test]
        fn keeps_its_parts() {
            let value = Time::new(52_191_123_000_000, TimePrecision::MILLIS).unwrap();
            assert_eq!(value.nanos_since_midnight(), 52_191_123_000_000);
            assert_eq!(value.precision(), TimePrecision::MILLIS);
        }

        #[test]
        fn takes_a_day_with_a_leap_second() {
            assert!(Time::new(0, TimePrecision::NANOS).is_some());
            assert!(Time::new(86_400_999_999_999, TimePrecision::NANOS).is_some());
            assert_eq!(Time::new(86_401_000_000_000, TimePrecision::NANOS), None);
            assert_eq!(Time::new(-1, TimePrecision::NANOS), None);
        }
    }

    mod parsing {
        use super::*;

        #[test]
        fn reads_whole_seconds() {
            assert_eq!(
                parts("14:29:51"),
                (52_191_000_000_000, TimePrecision::SECONDS)
            );
        }

        #[test]
        fn reads_the_common_precisions() {
            assert_eq!(
                parts("14:29:51.123"),
                (52_191_123_000_000, TimePrecision::MILLIS)
            );
            assert_eq!(
                parts("14:29:51.123456"),
                (52_191_123_456_000, TimePrecision::MICROS)
            );
            assert_eq!(
                parts("14:29:51.123456789"),
                (52_191_123_456_789, TimePrecision::NANOS)
            );
        }

        #[test]
        fn reads_any_fraction_up_to_nine_digits() {
            for (fraction, nanos) in [
                ("1", 100_000_000),
                ("1234", 123_400_000),
                ("1234567", 123_456_700),
            ] {
                let (since_midnight, precision) = parts(&format!("14:29:51.{fraction}"));
                assert_eq!(since_midnight, 52_191_000_000_000 + nanos, "{fraction}");
                assert_eq!(usize::from(precision.fraction_digits()), fraction.len());
            }
        }

        #[test]
        fn keeps_the_precision_of_a_zero_fraction() {
            assert_eq!(
                parts("14:29:51.000"),
                (52_191_000_000_000, TimePrecision::MILLIS)
            );
        }

        #[test]
        fn reads_the_ends_of_the_day() {
            assert_eq!(parts("00:00:00").0, 0);
            assert_eq!(parts("23:59:59.999999999").0, 86_399_999_999_999);
        }

        #[test]
        fn reads_a_leap_second_after_the_last_second_of_the_day() {
            assert_eq!(parts("23:59:60").0, 86_400_000_000_000);
            assert_eq!(parts("23:59:60.999999999").0, 86_400_999_999_999);
        }

        #[test]
        fn rejects_times_out_of_range() {
            for text in ["24:00:00", "14:60:00", "14:29:61"] {
                assert_eq!(parse(text), None, "{text:?}");
            }
        }

        #[test]
        fn rejects_an_empty_or_sub_nanosecond_fraction() {
            for fraction in [".", ".1234567890", ".123456789012"] {
                let text = format!("14:29:51{fraction}");
                assert_eq!(parse(&text), None, "{text:?}");
            }
        }

        #[test]
        fn rejects_other_shapes() {
            for text in [
                "",
                "14:29",
                "14:29:5",
                "142951",
                "14.29.51",
                "14:29:51Z",
                "14:29:51,123",
                "14:29:5a",
                "+4:29:51",
                "14:29:51.12a",
                "T14:29:51",
            ] {
                assert_eq!(parse(text), None, "{text:?}");
            }
        }
    }

    mod equality {
        use super::*;
        use std::hash::DefaultHasher;

        fn hash_of(value: Time) -> u64 {
            let mut hasher = DefaultHasher::new();
            value.hash(&mut hasher);
            hasher.finish()
        }

        #[test]
        fn compares_times_across_precisions() {
            assert_eq!(parse("14:29:51"), parse("14:29:51.000000"));
            assert_ne!(parse("14:29:51"), parse("14:29:51.001"));
        }

        #[test]
        fn equal_times_hash_alike() {
            assert_eq!(
                hash_of(Time::new(1_000, TimePrecision::MICROS).unwrap()),
                hash_of(Time::new(1_000, TimePrecision::NANOS).unwrap())
            );
        }

        #[test]
        fn orders_by_time() {
            assert!(parse("14:29:51.999") < parse("14:29:52"));
            assert!(parse("23:59:59.999999999") < parse("23:59:60"));
        }
    }
}
