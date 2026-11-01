use crate::{Date, OutOfRange};
use chrono_04::{Datelike, NaiveDate};

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

#[cfg(test)]
mod tests {
    use super::*;

    fn naive_date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
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
}
