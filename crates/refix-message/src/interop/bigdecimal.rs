use crate::{Decimal, OutOfRange};
use bigdecimal_04::BigDecimal;
use bigdecimal_04::num_bigint::BigInt;

impl From<Decimal> for BigDecimal {
    fn from(decimal: Decimal) -> Self {
        BigDecimal::new(
            BigInt::from(decimal.mantissa()),
            -i64::from(decimal.exponent()),
        )
    }
}

impl TryFrom<&BigDecimal> for Decimal {
    type Error = OutOfRange;

    /// Fails when the digits don't fit an `i64` or the scale an `i8`, so round first.
    fn try_from(decimal: &BigDecimal) -> Result<Self, Self::Error> {
        let (digits, scale) = decimal.as_bigint_and_scale();
        let mantissa = i64::try_from(digits.as_ref()).map_err(|_| OutOfRange)?;
        let exponent = scale
            .checked_neg()
            .and_then(|exponent| i8::try_from(exponent).ok())
            .ok_or(OutOfRange)?;
        Ok(Decimal::new(mantissa, exponent))
    }
}

impl TryFrom<BigDecimal> for Decimal {
    type Error = OutOfRange;

    /// Fails when the digits don't fit an `i64` or the scale an `i8`, so round first.
    fn try_from(decimal: BigDecimal) -> Result<Self, Self::Error> {
        Decimal::try_from(&decimal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn big_decimal(digits: impl Into<BigInt>, scale: i64) -> BigDecimal {
        BigDecimal::new(digits.into(), scale)
    }

    fn to_big_decimal(mantissa: i64, exponent: i8) -> (BigInt, i64) {
        BigDecimal::from(Decimal::new(mantissa, exponent)).into_bigint_and_scale()
    }

    fn from_big_decimal(decimal: BigDecimal) -> (i64, i8) {
        let converted = Decimal::try_from(decimal).unwrap();
        (converted.mantissa(), converted.exponent())
    }

    #[test]
    fn converts_to_bigdecimal_keeping_the_scale() {
        assert_eq!(to_big_decimal(10125, -2), (BigInt::from(10125), 2));
        assert_eq!(to_big_decimal(150, -2), (BigInt::from(150), 2));
    }

    #[test]
    fn converts_from_bigdecimal_keeping_the_scale() {
        assert_eq!(from_big_decimal(big_decimal(10125, 2)), (10125, -2));
        assert_eq!(from_big_decimal(big_decimal(150, 2)), (150, -2));
    }

    #[test]
    fn keeps_a_positive_exponent_as_a_negative_scale() {
        assert_eq!(to_big_decimal(5, 3), (BigInt::from(5), -3));
        assert_eq!(from_big_decimal(big_decimal(5, -3)), (5, 3));
    }

    #[test]
    fn round_trips_the_ends_of_an_i64_and_an_i8() {
        for (mantissa, exponent) in [(i64::MIN, i8::MIN), (i64::MAX, i8::MAX)] {
            let converted = BigDecimal::from(Decimal::new(mantissa, exponent));
            assert_eq!(from_big_decimal(converted), (mantissa, exponent));
        }
    }

    #[test]
    fn converts_a_borrowed_bigdecimal() {
        let decimal = big_decimal(10125, 2);
        assert_eq!(Decimal::try_from(&decimal), Ok(Decimal::new(10125, -2)));
    }

    #[test]
    fn rejects_digits_beyond_an_i64() {
        let past_the_end = big_decimal(BigInt::from(i64::MAX) + 1, 0);
        let third = BigDecimal::from(1) / BigDecimal::from(3);
        for decimal in [past_the_end, third] {
            assert_eq!(Decimal::try_from(decimal), Err(OutOfRange));
        }
    }

    #[test]
    fn rejects_a_scale_beyond_an_i8() {
        for scale in [129, -128, i64::MIN, i64::MAX] {
            assert_eq!(Decimal::try_from(big_decimal(1, scale)), Err(OutOfRange));
        }
    }
}
