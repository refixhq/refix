/// How finely a time is written, as its count of fraction digits.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TimePrecision(u8);

impl TimePrecision {
    /// No fraction, as in `14:29:51`.
    pub const SECONDS: Self = Self(0);
    /// Three digits, as in `14:29:51.123`.
    pub const MILLIS: Self = Self(3);
    /// Six digits.
    pub const MICROS: Self = Self(6);
    /// Nine digits.
    pub const NANOS: Self = Self(9);

    /// The precision of a fraction with `count` digits, up to nine.
    pub const fn from_fraction_digits(count: u8) -> Option<Self> {
        if count <= 9 { Some(Self(count)) } else { None }
    }

    /// The number of fraction digits, such as `3` for `MILLIS`.
    pub const fn fraction_digits(self) -> u8 {
        self.0
    }

    /// Nanoseconds in one unit, such as `1_000_000` for `MILLIS`.
    pub(super) const fn unit_nanos(self) -> i64 {
        10_i64.pow(9 - self.0 as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_up_to_nine_digits() {
        assert_eq!(
            TimePrecision::from_fraction_digits(9),
            Some(TimePrecision::NANOS)
        );
        assert_eq!(TimePrecision::from_fraction_digits(10), None);
    }

    #[test]
    fn names_the_common_precisions() {
        for (precision, digits) in [
            (TimePrecision::SECONDS, 0),
            (TimePrecision::MILLIS, 3),
            (TimePrecision::MICROS, 6),
            (TimePrecision::NANOS, 9),
        ] {
            assert_eq!(precision.fraction_digits(), digits);
        }
    }

    #[test]
    fn scales_one_unit_to_nanos() {
        let tenths_of_millis = TimePrecision::from_fraction_digits(4).unwrap();
        assert_eq!(TimePrecision::SECONDS.unit_nanos(), 1_000_000_000);
        assert_eq!(tenths_of_millis.unit_nanos(), 100_000);
        assert_eq!(TimePrecision::NANOS.unit_nanos(), 1);
    }
}
