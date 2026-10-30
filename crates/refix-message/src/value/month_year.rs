use super::{Date, number};

/// A FIX MonthYear, such as a contract's maturity, with an optional day or week of the month.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct MonthYear {
    year: u16,
    month: u8,
    day_or_week: Option<DayOrWeek>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum DayOrWeek {
    Day(u8),
    Week(u8),
}

impl MonthYear {
    /// The month `month` of `year`, if it exists and the year fits FIX's four digits.
    pub const fn new(year: u16, month: u8) -> Option<Self> {
        Self::checked(year, month, None)
    }

    /// The day `day` of `month` in `year`, if that date exists.
    pub const fn with_day(year: u16, month: u8, day: u8) -> Option<Self> {
        Self::checked(year, month, Some(DayOrWeek::Day(day)))
    }

    /// The week `week` of `month` in `year`, from `1` to `5`.
    pub const fn with_week(year: u16, month: u8, week: u8) -> Option<Self> {
        Self::checked(year, month, Some(DayOrWeek::Week(week)))
    }

    /// The year, such as `2026`.
    pub const fn year(&self) -> u16 {
        self.year
    }

    /// The month, from `1` for January to `12` for December.
    pub const fn month(&self) -> u8 {
        self.month
    }

    /// The day of the month, if one was given.
    pub const fn day(&self) -> Option<u8> {
        match self.day_or_week {
            Some(DayOrWeek::Day(day)) => Some(day),
            _ => None,
        }
    }

    /// The week of the month, from `1` to `5`, if one was given.
    pub const fn week(&self) -> Option<u8> {
        match self.day_or_week {
            Some(DayOrWeek::Week(week)) => Some(week),
            _ => None,
        }
    }

    /// Reads a FIX MonthYear such as `202612`, `20261218` or `202612w3`.
    pub(crate) fn parse(text: &[u8]) -> Option<Self> {
        let (year_month, rest) = text.split_at_checked(6)?;
        let year = u16::try_from(number(&year_month[0..4])?).ok()?;
        let month = u8::try_from(number(&year_month[4..6])?).ok()?;
        match rest {
            [] => Self::new(year, month),
            [b'w', week] if week.is_ascii_digit() => Self::with_week(year, month, week - b'0'),
            [_, _] => Self::with_day(year, month, u8::try_from(number(rest)?).ok()?),
            _ => None,
        }
    }

    const fn checked(year: u16, month: u8, day_or_week: Option<DayOrWeek>) -> Option<Self> {
        // The month exists if its first day does.
        let valid = match day_or_week {
            None => Date::new(year, month, 1).is_some(),
            Some(DayOrWeek::Day(day)) => Date::new(year, month, day).is_some(),
            Some(DayOrWeek::Week(week)) => {
                Date::new(year, month, 1).is_some() && matches!(week, 1..=5)
            }
        };
        if valid {
            Some(Self {
                year,
                month,
                day_or_week,
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Option<MonthYear> {
        MonthYear::parse(text.as_bytes())
    }

    mod construction {
        use super::*;

        #[test]
        fn keeps_its_parts() {
            let month = MonthYear::new(2026, 12).unwrap();
            assert_eq!((month.year(), month.month()), (2026, 12));
            assert_eq!((month.day(), month.week()), (None, None));

            let day = MonthYear::with_day(2026, 12, 18).unwrap();
            assert_eq!((day.day(), day.week()), (Some(18), None));

            let week = MonthYear::with_week(2026, 12, 3).unwrap();
            assert_eq!((week.day(), week.week()), (None, Some(3)));
        }

        #[test]
        fn rejects_months_that_do_not_exist() {
            assert_eq!(MonthYear::new(2026, 0), None);
            assert_eq!(MonthYear::new(2026, 13), None);
            assert_eq!(MonthYear::new(10000, 1), None);
        }

        #[test]
        fn checks_the_day_against_its_month() {
            assert!(MonthYear::with_day(2024, 2, 29).is_some());
            assert_eq!(MonthYear::with_day(2026, 2, 29), None);
            assert_eq!(MonthYear::with_day(2026, 4, 31), None);
            assert_eq!(MonthYear::with_day(2026, 12, 0), None);
        }

        #[test]
        fn takes_weeks_one_to_five() {
            assert!(MonthYear::with_week(2026, 12, 1).is_some());
            assert!(MonthYear::with_week(2026, 12, 5).is_some());
            assert_eq!(MonthYear::with_week(2026, 12, 0), None);
            assert_eq!(MonthYear::with_week(2026, 12, 6), None);
            assert_eq!(MonthYear::with_week(2026, 13, 1), None);
        }
    }

    mod parsing {
        use super::*;

        #[test]
        fn reads_a_month() {
            assert_eq!(parse("202612"), MonthYear::new(2026, 12));
        }

        #[test]
        fn reads_a_day() {
            assert_eq!(parse("20261218"), MonthYear::with_day(2026, 12, 18));
        }

        #[test]
        fn reads_a_week() {
            assert_eq!(parse("202612w3"), MonthYear::with_week(2026, 12, 3));
        }

        #[test]
        fn rejects_values_that_do_not_exist() {
            for text in [
                "202600", "202613", "20260231", "20261200", "202612w0", "202612w6",
            ] {
                assert_eq!(parse(text), None, "{text:?}");
            }
        }

        #[test]
        fn rejects_other_shapes() {
            for text in [
                "",
                "2026",
                "20261",
                "2026121",
                "202612181",
                "2026-12",
                "202612W3",
                "202612w",
                "202612w33",
                "2026121a",
                "+02612",
                "202612 ",
            ] {
                assert_eq!(parse(text), None, "{text:?}");
            }
        }
    }

    mod equality {
        use super::*;

        #[test]
        fn tells_the_forms_apart() {
            assert_ne!(parse("202612"), parse("20261201"));
            assert_ne!(parse("20261201"), parse("202612w1"));
        }
    }
}
