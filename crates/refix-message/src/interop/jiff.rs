use crate::{Date, OutOfRange};
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
}
