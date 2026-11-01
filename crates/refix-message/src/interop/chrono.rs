use super::exact_precision;
use crate::{Date, OutOfRange, Time};
use chrono_04::{Datelike, NaiveDate, NaiveTime, Timelike};

impl From<Date> for NaiveDate {
    fn from(date: Date) -> Self {
        NaiveDate::from_ymd_opt(date.year().into(), date.month().into(), date.day().into())
            .expect("years 0 to 9999 are within chrono's range")
    }
}

impl TryFrom<NaiveDate> for Date {
    type Error = OutOfRange;

    fn try_from(date: NaiveDate) -> Result<Self, Self::Error> {
        u16::try_from(date.year())
            .ok()
            .and_then(|year| Date::new(year, date.month() as u8, date.day() as u8))
            .ok_or(OutOfRange)
    }
}

impl From<Time> for NaiveTime {
    /// Keeps a leap second, which chrono holds as `23:59:59` and over a second of nanoseconds.
    fn from(time: Time) -> Self {
        let nanos = time.nanos_since_midnight();
        let seconds = (nanos / 1_000_000_000).min(86_399);
        let nano = nanos - seconds * 1_000_000_000;
        NaiveTime::from_num_seconds_from_midnight_opt(seconds as u32, nano as u32)
            .expect("a leap second follows 23:59:59")
    }
}

impl From<NaiveTime> for Time {
    /// A leap second before 23:59 counts as the first second of the next minute, as in a message.
    fn from(time: NaiveTime) -> Self {
        let nanos = i64::from(time.num_seconds_from_midnight()) * 1_000_000_000
            + i64::from(time.nanosecond());
        Time::new(nanos, exact_precision(nanos)).expect("a chrono time is within a day")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TimePrecision;

    fn naive_date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn naive_time(hour: u32, minute: u32, second: u32, nano: u32) -> NaiveTime {
        NaiveTime::from_hms_nano_opt(hour, minute, second, nano).unwrap()
    }

    mod date {
        use super::*;

        #[test]
        fn converts_to_chrono() {
            let date = Date::new(2026, 10, 30).unwrap();
            assert_eq!(NaiveDate::from(date), naive_date(2026, 10, 30));
        }

        #[test]
        fn converts_from_chrono() {
            let date = Date::new(2026, 10, 30).unwrap();
            assert_eq!(Date::try_from(naive_date(2026, 10, 30)), Ok(date));
        }

        #[test]
        fn round_trips_the_first_and_last_fix_years() {
            for date in [
                Date::new(0, 1, 1).unwrap(),
                Date::new(9999, 12, 31).unwrap(),
            ] {
                assert_eq!(Date::try_from(NaiveDate::from(date)), Ok(date));
            }
        }

        #[test]
        fn rejects_a_year_before_zero_or_after_9999() {
            for chrono in [naive_date(-1, 12, 31), naive_date(10_000, 1, 1)] {
                assert_eq!(Date::try_from(chrono), Err(OutOfRange));
            }
        }
    }

    mod time {
        use super::*;

        #[test]
        fn converts_to_chrono() {
            let time = Time::new(52_191_123_000_000, TimePrecision::MILLIS).unwrap();
            assert_eq!(NaiveTime::from(time), naive_time(14, 29, 51, 123_000_000));
        }

        #[test]
        fn converts_from_chrono() {
            let time = Time::from(naive_time(14, 29, 51, 123_000_000));
            assert_eq!(time.nanos_since_midnight(), 52_191_123_000_000);
            assert_eq!(time.precision(), TimePrecision::MILLIS);
        }

        #[test]
        fn round_trips_the_first_and_last_nanoseconds_of_the_day() {
            for nanos in [0, 86_400_999_999_999] {
                let time = Time::new(nanos, TimePrecision::NANOS).unwrap();
                assert_eq!(Time::from(NaiveTime::from(time)), time);
            }
        }

        #[test]
        fn keeps_a_leap_second_at_the_end_of_the_day() {
            let leap = Time::new(86_400_500_000_000, TimePrecision::MILLIS).unwrap();
            assert_eq!(NaiveTime::from(leap), naive_time(23, 59, 59, 1_500_000_000));
        }

        #[test]
        fn reads_an_earlier_leap_second_as_the_next_minute() {
            let time = Time::from(naive_time(14, 29, 59, 1_500_000_000));
            assert_eq!(time.nanos_since_midnight(), 52_200_500_000_000);
        }
    }
}
