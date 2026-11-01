use super::exact_precision;
use crate::{Date, OutOfRange, Time, UtcTimestamp};
use time_03::{Month, UtcDateTime};

impl From<Date> for time_03::Date {
    fn from(date: Date) -> Self {
        let month = Month::try_from(date.month()).expect("a FIX month is a month");
        time_03::Date::from_calendar_date(date.year().into(), month, date.day())
            .expect("years 0 to 9999 are within time's range")
    }
}

impl TryFrom<time_03::Date> for Date {
    type Error = OutOfRange;

    fn try_from(date: time_03::Date) -> Result<Self, Self::Error> {
        u16::try_from(date.year())
            .ok()
            .and_then(|year| Date::new(year, date.month().into(), date.day()))
            .ok_or(OutOfRange)
    }
}

impl From<Time> for time_03::Time {
    /// time has no leap second, so `23:59:60` clamps to `23:59:59.999999999`.
    fn from(time: Time) -> Self {
        let nanos = time.nanos_since_midnight().min(86_399_999_999_999);
        let seconds = nanos / 1_000_000_000;
        time_03::Time::from_hms_nano(
            (seconds / 3_600) as u8,
            (seconds / 60 % 60) as u8,
            (seconds % 60) as u8,
            (nanos % 1_000_000_000) as u32,
        )
        .expect("the clamped time is within a day")
    }
}

impl From<time_03::Time> for Time {
    fn from(time: time_03::Time) -> Self {
        let (hour, minute, second, nanosecond) = time.as_hms_nano();
        let seconds = i64::from(hour) * 3_600 + i64::from(minute) * 60 + i64::from(second);
        let nanos = seconds * 1_000_000_000 + i64::from(nanosecond);
        Time::new(nanos, exact_precision(nanos)).expect("a time from time is within a day")
    }
}

impl From<UtcTimestamp> for UtcDateTime {
    fn from(timestamp: UtcTimestamp) -> Self {
        UtcDateTime::from_unix_timestamp_nanos(i128::from(timestamp.unix_nanos()))
            .expect("an i64 of nanoseconds is within time's range")
    }
}

impl TryFrom<UtcDateTime> for UtcTimestamp {
    type Error = OutOfRange;

    fn try_from(timestamp: UtcDateTime) -> Result<Self, Self::Error> {
        let nanos = i64::try_from(timestamp.unix_timestamp_nanos()).map_err(|_| OutOfRange)?;
        Ok(UtcTimestamp::new(nanos, exact_precision(nanos)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TimePrecision;

    fn calendar_date(year: i32, month: Month, day: u8) -> time_03::Date {
        time_03::Date::from_calendar_date(year, month, day).unwrap()
    }

    mod date {
        use super::*;

        #[test]
        fn converts_to_time() {
            let date = Date::new(2026, 10, 30).unwrap();
            assert_eq!(
                time_03::Date::from(date),
                calendar_date(2026, Month::October, 30)
            );
        }

        #[test]
        fn converts_from_time() {
            let date = Date::new(2026, 10, 30).unwrap();
            assert_eq!(
                Date::try_from(calendar_date(2026, Month::October, 30)),
                Ok(date)
            );
        }

        #[test]
        fn round_trips_the_first_and_last_fix_years() {
            for date in [
                Date::new(0, 1, 1).unwrap(),
                Date::new(9999, 12, 31).unwrap(),
            ] {
                assert_eq!(Date::try_from(time_03::Date::from(date)), Ok(date));
            }
        }

        #[test]
        fn rejects_a_year_before_zero() {
            assert_eq!(
                Date::try_from(calendar_date(-1, Month::December, 31)),
                Err(OutOfRange)
            );
        }
    }

    mod time {
        use super::*;

        #[test]
        fn converts_to_time() {
            let time = Time::new(52_191_123_000_000, TimePrecision::MILLIS).unwrap();
            assert_eq!(
                time_03::Time::from(time),
                time_03::Time::from_hms_milli(14, 29, 51, 123).unwrap()
            );
        }

        #[test]
        fn converts_from_time() {
            let time = Time::from(time_03::Time::from_hms_milli(14, 29, 51, 123).unwrap());
            assert_eq!(time.nanos_since_midnight(), 52_191_123_000_000);
            assert_eq!(time.precision(), TimePrecision::MILLIS);
        }

        #[test]
        fn round_trips_the_first_and_last_nanoseconds_of_the_day() {
            for time in [time_03::Time::MIDNIGHT, time_03::Time::MAX] {
                assert_eq!(time_03::Time::from(Time::from(time)), time);
            }
        }

        #[test]
        fn clamps_a_leap_second_to_the_end_of_the_day() {
            let leap = Time::new(86_400_500_000_000, TimePrecision::MILLIS).unwrap();
            assert_eq!(time_03::Time::from(leap), time_03::Time::MAX);
        }
    }

    mod timestamp {
        use super::*;

        fn transact_time() -> UtcDateTime {
            calendar_date(2026, Month::October, 8)
                .with_hms_milli(9, 30, 0, 123)
                .unwrap()
                .as_utc()
        }

        #[test]
        fn converts_to_time() {
            let timestamp = UtcTimestamp::new(1_791_451_800_123_000_000, TimePrecision::MILLIS);
            assert_eq!(UtcDateTime::from(timestamp), transact_time());
        }

        #[test]
        fn converts_from_time() {
            let timestamp = UtcTimestamp::try_from(transact_time()).unwrap();
            assert_eq!(timestamp.unix_nanos(), 1_791_451_800_123_000_000);
            assert_eq!(timestamp.precision(), TimePrecision::MILLIS);
        }

        #[test]
        fn round_trips_the_ends_of_an_i64() {
            for nanos in [i64::MIN, i64::MAX] {
                let timestamp = UtcTimestamp::new(nanos, TimePrecision::NANOS);
                assert_eq!(
                    UtcTimestamp::try_from(UtcDateTime::from(timestamp)),
                    Ok(timestamp)
                );
            }
        }

        #[test]
        fn rejects_a_timestamp_beyond_an_i64_of_nanoseconds() {
            let past_the_end =
                UtcDateTime::from_unix_timestamp_nanos(i128::from(i64::MAX) + 1).unwrap();
            for time in [past_the_end, UtcDateTime::MIN, UtcDateTime::MAX] {
                assert_eq!(UtcTimestamp::try_from(time), Err(OutOfRange));
            }
        }
    }
}
