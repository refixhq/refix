use crate::{Date, OutOfRange};
use time_03::Month;

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
