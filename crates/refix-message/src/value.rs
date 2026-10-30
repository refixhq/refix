mod date;
mod decimal;
mod error;
mod time_precision;
mod utc_timestamp;

pub use date::Date;
pub use decimal::Decimal;
pub use error::InvalidValue;
pub use time_precision::TimePrecision;
pub use utc_timestamp::UtcTimestamp;

/// The value of `digits`, if they're all ASCII digits.
fn number(digits: &[u8]) -> Option<i64> {
    digits.iter().try_fold(0, |value, &byte| {
        byte.is_ascii_digit()
            .then(|| value * 10 + i64::from(byte - b'0'))
    })
}
