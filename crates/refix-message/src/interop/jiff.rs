use crate::{Date, OutOfRange, Time, TimePrecision};
use jiff_02::civil;

impl From<Date> for civil::Date {
    fn from(date: Date) -> Self {
        civil::Date::new(date.year() as i16, date.month() as i8, date.day() as i8)
            .expect("years 0 to 9999 are within jiff's range")
    }
}

impl TryFrom<civil::Date> for Date {
    type Error = OutOfRange;

    fn try_from(date: civil::Date) -> Result<Self, Self::Error> {
        u16::try_from(date.year())
            .ok()
            .and_then(|year| Date::new(year, date.month() as u8, date.day() as u8))
            .ok_or(OutOfRange)
    }
}

impl From<Time> for civil::Time {
    /// jiff has no leap second, so `23:59:60` clamps to `23:59:59.999999999`.
    fn from(time: Time) -> Self {
        let nanos = time.nanos_since_midnight().min(86_399_999_999_999);
        let seconds = nanos / 1_000_000_000;
        civil::Time::new(
            (seconds / 3_600) as i8,
            (seconds / 60 % 60) as i8,
            (seconds % 60) as i8,
            (nanos % 1_000_000_000) as i32,
        )
        .expect("the clamped time is within a day")
    }
}

impl From<civil::Time> for Time {
    fn from(time: civil::Time) -> Self {
        let seconds = i64::from(time.hour()) * 3_600
            + i64::from(time.minute()) * 60
            + i64::from(time.second());
        let nanos = seconds * 1_000_000_000 + i64::from(time.subsec_nanosecond());
        Time::new(nanos, exact_precision(nanos)).expect("a jiff time is within a day")
    }
}

/// The coarsest of seconds, millis, micros and nanos that holds `nanos` exactly.
fn exact_precision(nanos: i64) -> TimePrecision {
    [
        TimePrecision::SECONDS,
        TimePrecision::MILLIS,
        TimePrecision::MICROS,
    ]
    .into_iter()
    .find(|precision| nanos % precision.unit_nanos() == 0)
    .unwrap_or(TimePrecision::NANOS)
}

#[cfg(test)]
mod tests {
    use super::*;

    mod date {
        use super::*;

        #[test]
        fn converts_to_jiff() {
            let date = Date::new(2026, 10, 30).unwrap();
            assert_eq!(civil::Date::from(date), civil::date(2026, 10, 30));
        }

        #[test]
        fn converts_from_jiff() {
            let date = Date::new(2026, 10, 30).unwrap();
            assert_eq!(Date::try_from(civil::date(2026, 10, 30)), Ok(date));
        }

        #[test]
        fn round_trips_the_first_and_last_fix_years() {
            for date in [
                Date::new(0, 1, 1).unwrap(),
                Date::new(9999, 12, 31).unwrap(),
            ] {
                assert_eq!(Date::try_from(civil::Date::from(date)), Ok(date));
            }
        }

        #[test]
        fn rejects_a_year_before_zero() {
            assert_eq!(Date::try_from(civil::date(-1, 12, 31)), Err(OutOfRange));
        }
    }

    mod time {
        use super::*;

        #[test]
        fn converts_to_jiff() {
            let time = Time::new(52_191_123_000_000, TimePrecision::MILLIS).unwrap();
            assert_eq!(
                civil::Time::from(time),
                civil::time(14, 29, 51, 123_000_000)
            );
        }

        #[test]
        fn converts_from_jiff() {
            let time = Time::from(civil::time(14, 29, 51, 123_000_000));
            assert_eq!(time.nanos_since_midnight(), 52_191_123_000_000);
        }

        #[test]
        fn round_trips_the_first_and_last_nanoseconds_of_the_day() {
            for jiff in [civil::Time::MIN, civil::Time::MAX] {
                assert_eq!(civil::Time::from(Time::from(jiff)), jiff);
            }
        }

        #[test]
        fn clamps_a_leap_second_to_the_end_of_the_day() {
            let leap = Time::new(86_400_500_000_000, TimePrecision::MILLIS).unwrap();
            assert_eq!(civil::Time::from(leap), civil::Time::MAX);
        }

        #[test]
        fn takes_the_coarsest_exact_precision_from_jiff() {
            for (subsec_nanosecond, precision) in [
                (0, TimePrecision::SECONDS),
                (100_000_000, TimePrecision::MILLIS),
                (123_000_000, TimePrecision::MILLIS),
                (123_456_000, TimePrecision::MICROS),
                (123_456_789, TimePrecision::NANOS),
            ] {
                let time = Time::from(civil::time(14, 29, 51, subsec_nanosecond));
                assert_eq!(time.precision(), precision, "{subsec_nanosecond}");
            }
        }
    }
}
