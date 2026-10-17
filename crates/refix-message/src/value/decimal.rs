use std::fmt;
use std::hash::{Hash, Hasher};

/// A FIX float, such as a price or quantity, as `mantissa * 10^exponent`.
#[derive(Clone, Copy, Debug)]
pub struct Decimal {
    mantissa: i64,
    exponent: i8,
}

impl Decimal {
    /// The decimal `mantissa * 10^exponent`.
    pub const fn new(mantissa: i64, exponent: i8) -> Self {
        Self { mantissa, exponent }
    }

    /// The value's digits as an integer, such as `10125` for `101.25`.
    pub const fn mantissa(&self) -> i64 {
        self.mantissa
    }

    /// The power of ten the mantissa is scaled by, such as `-2` for `101.25`.
    pub const fn exponent(&self) -> i8 {
        self.exponent
    }

    /// Reads a FIX float such as `-101.25`, keeping its scale.
    pub(crate) fn parse(text: &[u8]) -> Option<Self> {
        let (negative, unsigned) = match text.strip_prefix(b"-") {
            Some(rest) => (true, rest),
            None => (false, text),
        };
        let mut parts = unsigned.splitn(2, |&byte| byte == b'.');
        let whole = parts.next().unwrap_or_default();
        let fraction = parts.next().unwrap_or_default();
        if whole.is_empty() && fraction.is_empty() {
            return None;
        }
        let exponent = -i8::try_from(fraction.len()).ok()?;
        let mut mantissa: i64 = 0;
        for &byte in whole.iter().chain(fraction) {
            if !byte.is_ascii_digit() {
                return None;
            }
            mantissa = mantissa
                .checked_mul(10)?
                .checked_add(i64::from(byte - b'0'))?;
        }
        let mantissa = if negative { -mantissa } else { mantissa };
        Some(Self::new(mantissa, exponent))
    }

    /// The same value without trailing zeros, to compare across scales.
    fn normalized(self) -> (i64, i8) {
        if self.mantissa == 0 {
            return (0, 0);
        }
        let (mut mantissa, mut exponent) = (self.mantissa, self.exponent);
        while mantissa % 10 == 0 && exponent < i8::MAX {
            mantissa /= 10;
            exponent += 1;
        }
        (mantissa, exponent)
    }
}

impl PartialEq for Decimal {
    fn eq(&self, other: &Self) -> bool {
        self.normalized() == other.normalized()
    }
}

impl Eq for Decimal {}

impl Hash for Decimal {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.normalized().hash(state);
    }
}

impl fmt::Display for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.exponent >= 0 {
            write!(f, "{}", self.mantissa)?;
            for _ in 0..self.exponent {
                f.write_str("0")?;
            }
            return Ok(());
        }
        let sign = if self.mantissa < 0 { "-" } else { "" };
        let digits = self.mantissa.unsigned_abs().to_string();
        let scale = usize::from(self.exponent.unsigned_abs());
        if digits.len() > scale {
            let (whole, fraction) = digits.split_at(digits.len() - scale);
            write!(f, "{sign}{whole}.{fraction}")
        } else {
            write!(f, "{sign}0.{digits:0>scale$}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Option<Decimal> {
        Decimal::parse(text.as_bytes())
    }

    fn parts(text: &str) -> (i64, i8) {
        let value = parse(text).unwrap();
        (value.mantissa(), value.exponent())
    }

    mod parsing {
        use super::*;

        #[test]
        fn reads_the_scale_from_the_fraction() {
            assert_eq!(parts("101.25"), (10125, -2));
        }

        #[test]
        fn keeps_trailing_zeros() {
            assert_eq!(parts("1.50"), (150, -2));
        }

        #[test]
        fn reads_a_negative_value() {
            assert_eq!(parts("-0.005"), (-5, -3));
        }

        #[test]
        fn reads_an_integer() {
            assert_eq!(parts("100"), (100, 0));
        }

        #[test]
        fn accepts_a_point_on_either_end() {
            assert_eq!(parts("23."), (23, 0));
            assert_eq!(parts(".5"), (5, -1));
            assert_eq!(parts("-.5"), (-5, -1));
        }

        #[test]
        fn skips_leading_zeros() {
            assert_eq!(parts("00023.23"), (2323, -2));
        }

        #[test]
        fn rejects_text_without_digits() {
            for text in ["", "-", ".", "-."] {
                assert_eq!(parse(text), None, "{text:?}");
            }
        }

        #[test]
        fn rejects_other_characters() {
            for text in ["+5", "1e5", " 1", "1 ", "1.2.3", "--1", "1-"] {
                assert_eq!(parse(text), None, "{text:?}");
            }
        }

        #[test]
        fn rejects_a_mantissa_beyond_i64() {
            assert_eq!(parts("9223372036854775807"), (i64::MAX, 0));
            assert_eq!(parse("9223372036854775808"), None);
            assert_eq!(parse("1.0000000000000000000"), None);
        }

        #[test]
        fn rejects_more_than_127_fraction_digits() {
            assert_eq!(parts(&format!("0.{}", "0".repeat(127))), (0, -127));
            assert_eq!(parse(&format!("0.{}", "0".repeat(128))), None);
        }
    }

    mod display {
        use super::*;

        #[test]
        fn keeps_the_scale() {
            assert_eq!(Decimal::new(150, -2).to_string(), "1.50");
        }

        #[test]
        fn pads_a_small_fraction() {
            assert_eq!(Decimal::new(-5, -3).to_string(), "-0.005");
            assert_eq!(Decimal::new(0, -2).to_string(), "0.00");
        }

        #[test]
        fn writes_an_integer_without_a_point() {
            assert_eq!(Decimal::new(23, 0).to_string(), "23");
            assert_eq!(Decimal::new(15, 3).to_string(), "15000");
        }

        #[test]
        fn round_trips_the_wire_text() {
            for text in ["101.25", "-1.50", "0.005", "100"] {
                assert_eq!(parse(text).unwrap().to_string(), text);
            }
        }
    }

    mod equality {
        use super::*;
        use std::hash::DefaultHasher;

        fn hash_of(value: Decimal) -> u64 {
            let mut hasher = DefaultHasher::new();
            value.hash(&mut hasher);
            hasher.finish()
        }

        #[test]
        fn compares_by_value_across_scales() {
            assert_eq!(Decimal::new(150, -2), Decimal::new(15, -1));
            assert_eq!(Decimal::new(0, -2), Decimal::new(0, 0));
            assert_ne!(Decimal::new(150, -2), Decimal::new(151, -2));
        }

        #[test]
        fn equal_values_hash_alike() {
            assert_eq!(
                hash_of(Decimal::new(150, -2)),
                hash_of(Decimal::new(15, -1))
            );
        }
    }
}
