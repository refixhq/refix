use crate::{Decimal, OutOfRange};

impl TryFrom<Decimal> for rust_decimal_1::Decimal {
    type Error = OutOfRange;

    /// Fails past rust_decimal's 28 fraction digits or 96 bits.
    fn try_from(decimal: Decimal) -> Result<Self, Self::Error> {
        let mantissa = i128::from(decimal.mantissa());
        let power = u32::from(decimal.exponent().unsigned_abs());
        let (mantissa, scale) = if decimal.exponent() >= 0 {
            let whole = (0..power).try_fold(mantissa, |value, _| value.checked_mul(10));
            (whole.ok_or(OutOfRange)?, 0)
        } else {
            (mantissa, power)
        };
        rust_decimal_1::Decimal::try_from_i128_with_scale(mantissa, scale).map_err(|_| OutOfRange)
    }
}

impl TryFrom<rust_decimal_1::Decimal> for Decimal {
    type Error = OutOfRange;

    /// Fails when the digits don't fit an `i64`, so round first.
    fn try_from(decimal: rust_decimal_1::Decimal) -> Result<Self, Self::Error> {
        let mantissa = i64::try_from(decimal.mantissa()).map_err(|_| OutOfRange)?;
        let scale = i8::try_from(decimal.scale()).expect("rust_decimal's scale is at most 28");
        Ok(Decimal::new(mantissa, -scale))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rust_decimal(mantissa: i64, scale: u32) -> rust_decimal_1::Decimal {
        rust_decimal_1::Decimal::new(mantissa, scale)
    }

    fn to_rust_decimal(mantissa: i64, exponent: i8) -> (i128, u32) {
        let converted =
            rust_decimal_1::Decimal::try_from(Decimal::new(mantissa, exponent)).unwrap();
        (converted.mantissa(), converted.scale())
    }

    fn from_rust_decimal(decimal: rust_decimal_1::Decimal) -> (i64, i8) {
        let converted = Decimal::try_from(decimal).unwrap();
        (converted.mantissa(), converted.exponent())
    }

    #[test]
    fn converts_to_rust_decimal_keeping_the_scale() {
        assert_eq!(to_rust_decimal(10125, -2), (10125, 2));
        assert_eq!(to_rust_decimal(150, -2), (150, 2));
    }

    #[test]
    fn converts_from_rust_decimal_keeping_the_scale() {
        assert_eq!(from_rust_decimal(rust_decimal(10125, 2)), (10125, -2));
        assert_eq!(from_rust_decimal(rust_decimal(150, 2)), (150, -2));
    }

    #[test]
    fn writes_a_positive_exponent_out_as_whole_digits() {
        assert_eq!(to_rust_decimal(5, 3), (5000, 0));
        assert_eq!(to_rust_decimal(0, i8::MAX), (0, 0));
    }

    #[test]
    fn round_trips_the_ends_of_an_i64() {
        for decimal in [Decimal::new(i64::MIN, -28), Decimal::new(i64::MAX, 0)] {
            let converted = rust_decimal_1::Decimal::try_from(decimal).unwrap();
            let back = Decimal::try_from(converted).unwrap();
            assert_eq!(
                (back.mantissa(), back.exponent()),
                (decimal.mantissa(), decimal.exponent())
            );
        }
    }

    #[test]
    fn rejects_more_than_28_fraction_digits() {
        assert!(rust_decimal_1::Decimal::try_from(Decimal::new(1, -28)).is_ok());
        assert_eq!(
            rust_decimal_1::Decimal::try_from(Decimal::new(1, -29)),
            Err(OutOfRange)
        );
    }

    #[test]
    fn rejects_a_value_past_96_bits() {
        assert!(rust_decimal_1::Decimal::try_from(Decimal::new(1, 28)).is_ok());
        assert_eq!(
            rust_decimal_1::Decimal::try_from(Decimal::new(1, 29)),
            Err(OutOfRange)
        );
    }

    #[test]
    fn rejects_digits_beyond_an_i64() {
        let third = rust_decimal_1::Decimal::ONE / rust_decimal_1::Decimal::from(3);
        assert_eq!(Decimal::try_from(third), Err(OutOfRange));
    }
}
