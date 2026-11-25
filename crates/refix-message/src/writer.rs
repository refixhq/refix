mod error;

use crate::framing::SOH;
use crate::message::RawField;
use crate::{Date, Decimal, MonthYear, RawMessage, Tag, Time, UtcTimestamp};
use bytes::Bytes;
pub use error::WriteError;

const NANOS_PER_SECOND: u64 = 1_000_000_000;
const SECONDS_PER_DAY: u32 = 86_400;

/// Writes fields and gives them back as a [`RawMessage`].
///
/// The fields are written in the order of the calls.
///
/// ```
/// use refix_message::{Decimal, FieldWriter, Tag};
///
/// let mut body = FieldWriter::new();
/// body.write_str(Tag(11), "ORDER-1")?;
/// body.write_str(Tag(55), "ACME")?;
/// body.write_decimal(Tag(44), Decimal::new(10125, -2));
/// let body = body.finish();
///
/// assert_eq!(body.to_string(), "11=ORDER-1|55=ACME|44=101.25");
/// assert_eq!(body.get_str(Tag(55))?, Some("ACME"));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Default)]
pub struct FieldWriter {
    bytes: Vec<u8>,
    fields: Vec<RawField>,
}

impl FieldWriter {
    /// A writer with nothing written.
    pub fn new() -> Self {
        Self::default()
    }

    /// A writer with room for `bytes` bytes and `fields` fields.
    pub fn with_capacity(bytes: usize, fields: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(bytes),
            fields: Vec::with_capacity(fields),
        }
    }

    /// Writes a field with `value` as its bytes.
    ///
    /// An empty value and a value that holds the separator are errors.
    /// Nothing is written for them.
    ///
    /// # Panics
    ///
    /// If the fields written pass 4 GiB.
    pub fn write(&mut self, tag: Tag, value: &[u8]) -> Result<(), WriteError> {
        if value.is_empty() {
            return Err(WriteError::EmptyValue { tag });
        }
        if value.contains(&SOH) {
            return Err(WriteError::SeparatorInValue { tag });
        }
        let value_start = self.begin(tag);
        self.bytes.extend_from_slice(value);
        self.end(tag, value_start);
        Ok(())
    }

    /// Writes a field with `value` as its text.
    ///
    /// Fails and panics as [`write`](Self::write) does.
    pub fn write_str(&mut self, tag: Tag, value: &str) -> Result<(), WriteError> {
        self.write(tag, value.as_bytes())
    }

    /// Writes a field with `value` as an integer.
    ///
    /// Panics as [`write`](Self::write) does.
    pub fn write_int(&mut self, tag: Tag, value: i64) {
        let value_start = self.begin(tag);
        if value < 0 {
            self.bytes.push(b'-');
        }
        let mut buffer = [0; 20];
        self.bytes
            .extend_from_slice(digits(&mut buffer, value.unsigned_abs()));
        self.end(tag, value_start);
    }

    /// Writes a field with `Y` for true or `N` for false.
    ///
    /// Panics as [`write`](Self::write) does.
    pub fn write_bool(&mut self, tag: Tag, value: bool) {
        let value_start = self.begin(tag);
        self.bytes.push(if value { b'Y' } else { b'N' });
        self.end(tag, value_start);
    }

    /// Writes a field with `value` as a decimal, keeping its scale.
    ///
    /// A positive exponent is written as zeros, so `Decimal::new(15, 3)` is `15000`.
    /// Panics as [`write`](Self::write) does.
    pub fn write_decimal(&mut self, tag: Tag, value: Decimal) {
        let value_start = self.begin(tag);
        if value.mantissa() < 0 {
            self.bytes.push(b'-');
        }
        let mut buffer = [0; 20];
        let digits = digits(&mut buffer, value.mantissa().unsigned_abs());
        let scale = usize::from(value.exponent().unsigned_abs());
        if value.exponent() >= 0 {
            self.bytes.extend_from_slice(digits);
            self.zeros(scale);
        } else if digits.len() > scale {
            let (whole, fraction) = digits.split_at(digits.len() - scale);
            self.bytes.extend_from_slice(whole);
            self.bytes.push(b'.');
            self.bytes.extend_from_slice(fraction);
        } else {
            self.bytes.extend_from_slice(b"0.");
            self.zeros(scale - digits.len());
            self.bytes.extend_from_slice(digits);
        }
        self.end(tag, value_start);
    }

    /// Writes a field with `value` as a date, such as `20261103`.
    ///
    /// Panics as [`write`](Self::write) does.
    pub fn write_date(&mut self, tag: Tag, value: Date) {
        let value_start = self.begin(tag);
        self.date(value);
        self.end(tag, value_start);
    }

    /// Writes a field with `value` as a time of day, such as `09:30:36.250`.
    ///
    /// The fraction has the digits of the value's precision.
    /// Digits past it are cut, not rounded.
    /// Panics as [`write`](Self::write) does.
    pub fn write_time(&mut self, tag: Tag, value: Time) {
        let value_start = self.begin(tag);
        self.time(value);
        self.end(tag, value_start);
    }

    /// Writes a field with `value` as a UTC timestamp, such as `20261103-09:30:36.250`.
    ///
    /// The fraction has the digits of the value's precision.
    /// Digits past it are cut, not rounded.
    /// Panics as [`write`](Self::write) does.
    pub fn write_utc_timestamp(&mut self, tag: Tag, value: UtcTimestamp) {
        let value_start = self.begin(tag);
        self.date(value.date());
        self.bytes.push(b'-');
        self.time(value.time());
        self.end(tag, value_start);
    }

    /// Writes a field with `value` as a month and year, such as `202612`, `20261218` or `202612w3`.
    ///
    /// Panics as [`write`](Self::write) does.
    pub fn write_month_year(&mut self, tag: Tag, value: MonthYear) {
        let value_start = self.begin(tag);
        self.two_digits(u32::from(value.year() / 100));
        self.two_digits(u32::from(value.year() % 100));
        self.two_digits(u32::from(value.month()));
        if let Some(day) = value.day() {
            self.two_digits(u32::from(day));
        } else if let Some(week) = value.week() {
            self.bytes.extend_from_slice(&[b'w', b'0' + week]);
        }
        self.end(tag, value_start);
    }

    /// Writes a data field after the length field that counts its bytes.
    ///
    /// The value may hold the separator.
    /// An empty value is an error, and nothing is written for it.
    ///
    /// Panics as [`write`](Self::write) does.
    pub fn write_data(
        &mut self,
        length_tag: Tag,
        tag: Tag,
        value: &[u8],
    ) -> Result<(), WriteError> {
        if value.is_empty() {
            return Err(WriteError::EmptyValue { tag });
        }
        let length_start = self.begin(length_tag);
        let mut buffer = [0; 20];
        self.bytes
            .extend_from_slice(digits(&mut buffer, value.len() as u64));
        self.end(length_tag, length_start);
        let value_start = self.begin(tag);
        self.bytes.extend_from_slice(value);
        self.end(tag, value_start);
        Ok(())
    }

    /// The fields written, as a message.
    pub fn finish(self) -> RawMessage {
        RawMessage::new(Bytes::from(self.bytes), self.fields)
    }

    /// Writes the tag and `=`, and returns where the value starts.
    fn begin(&mut self, tag: Tag) -> usize {
        let mut buffer = [0; 20];
        self.bytes
            .extend_from_slice(digits(&mut buffer, u64::from(tag.0)));
        self.bytes.push(b'=');
        self.bytes.len()
    }

    /// Writes the separator and records the field whose value started at `value_start`.
    fn end(&mut self, tag: Tag, value_start: usize) {
        let value_end = self.bytes.len();
        self.bytes.push(SOH);
        assert!(
            self.bytes.len() <= u32::MAX as usize,
            "the fields written pass 4 GiB"
        );
        self.fields.push(RawField {
            tag,
            value_start: value_start as u32,
            value_end: value_end as u32,
        });
    }

    fn zeros(&mut self, count: usize) {
        self.bytes.resize(self.bytes.len() + count, b'0');
    }

    fn date(&mut self, date: Date) {
        self.two_digits(u32::from(date.year() / 100));
        self.two_digits(u32::from(date.year() % 100));
        self.two_digits(u32::from(date.month()));
        self.two_digits(u32::from(date.day()));
    }

    fn time(&mut self, time: Time) {
        let nanos = time.nanos_since_midnight() as u64;
        let seconds = (nanos / NANOS_PER_SECOND) as u32;
        // The second after 23:59:59 is a leap second.
        let (hour, minute, second) = if seconds == SECONDS_PER_DAY {
            (23, 59, 60)
        } else {
            (seconds / 3_600, seconds / 60 % 60, seconds % 60)
        };
        self.two_digits(hour);
        self.bytes.push(b':');
        self.two_digits(minute);
        self.bytes.push(b':');
        self.two_digits(second);

        let count = usize::from(time.precision().fraction_digits());
        if count > 0 {
            let mut fraction = [b'0'; 9];
            let mut rest = nanos % NANOS_PER_SECOND;
            for digit in fraction.iter_mut().rev() {
                *digit = b'0' + (rest % 10) as u8;
                rest /= 10;
            }
            self.bytes.push(b'.');
            self.bytes.extend_from_slice(&fraction[..count]);
        }
    }

    fn two_digits(&mut self, number: u32) {
        self.bytes
            .extend_from_slice(&[b'0' + (number / 10) as u8, b'0' + (number % 10) as u8]);
    }
}

/// The decimal digits of `number`, written at the end of `buffer`.
fn digits(buffer: &mut [u8; 20], mut number: u64) -> &[u8] {
    let mut start = buffer.len();
    loop {
        start -= 1;
        buffer[start] = b'0' + (number % 10) as u8;
        number /= 10;
        if number == 0 {
            return &buffer[start..];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TimePrecision;

    #[test]
    fn writes_fields_in_the_order_of_the_calls() {
        let mut writer = FieldWriter::new();
        writer.write_str(Tag(453), "2").unwrap();
        writer.write_str(Tag(448), "AL").unwrap();
        writer.write(Tag(448), b"BX").unwrap();

        let message = writer.finish();

        assert_eq!(message.bytes().as_ref(), b"453=2\x01448=AL\x01448=BX\x01");
        let entries: Vec<_> = message.entries().collect();
        assert_eq!(
            entries,
            [
                (Tag(453), &b"2"[..]),
                (Tag(448), &b"AL"[..]),
                (Tag(448), &b"BX"[..])
            ]
        );
    }

    #[test]
    fn written_fields_read_back_through_the_getters() {
        let mut writer = FieldWriter::with_capacity(32, 2);
        writer.write_str(Tag(11), "ORDER-1").unwrap();
        writer.write_str(Tag(4_294_967_295), "100").unwrap();

        let message = writer.finish();

        assert_eq!(message.get_str(Tag(11)), Ok(Some("ORDER-1")));
        assert_eq!(message.get_int(Tag(4_294_967_295)), Ok(Some(100)));
        assert_eq!(message.get(Tag(55)), None);
    }

    #[test]
    fn a_writer_with_nothing_written_finishes_with_no_fields() {
        let message = FieldWriter::new().finish();

        assert!(message.bytes().is_empty());
        assert_eq!(message.entries().count(), 0);
    }

    #[test]
    fn rejects_an_empty_value_and_writes_nothing() {
        let mut writer = FieldWriter::new();
        writer.write_str(Tag(11), "ORDER-1").unwrap();

        let result = writer.write_str(Tag(58), "");

        assert_eq!(result, Err(WriteError::EmptyValue { tag: Tag(58) }));
        assert_eq!(writer.finish().bytes().as_ref(), b"11=ORDER-1\x01");
    }

    #[test]
    fn rejects_a_value_holding_the_separator_and_writes_nothing() {
        let mut writer = FieldWriter::new();
        writer.write_str(Tag(11), "ORDER-1").unwrap();

        let result = writer.write(Tag(58), b"late\x0110=000");

        assert_eq!(result, Err(WriteError::SeparatorInValue { tag: Tag(58) }));
        assert_eq!(writer.finish().bytes().as_ref(), b"11=ORDER-1\x01");
    }

    #[test]
    fn writes_an_integer_that_reads_back() {
        for value in [0, 7, 100, -5, i64::MAX, i64::MIN] {
            let mut writer = FieldWriter::new();
            writer.write_int(Tag(38), value);

            let message = writer.finish();

            assert_eq!(message.get_str(Tag(38)), Ok(Some(&*value.to_string())));
            assert_eq!(message.get_int(Tag(38)), Ok(Some(value)));
        }
    }

    #[test]
    fn writes_a_boolean_as_y_or_n() {
        let mut writer = FieldWriter::new();
        writer.write_bool(Tag(43), true);
        writer.write_bool(Tag(97), false);

        let message = writer.finish();

        assert_eq!(message.bytes().as_ref(), b"43=Y\x0197=N\x01");
        assert_eq!(message.get_bool(Tag(43)), Ok(Some(true)));
        assert_eq!(message.get_bool(Tag(97)), Ok(Some(false)));
    }

    #[test]
    fn writes_a_decimal_with_its_scale() {
        let cases = [
            (Decimal::new(10125, -2), "101.25"),
            (Decimal::new(150, -2), "1.50"),
            (Decimal::new(-5, -3), "-0.005"),
            (Decimal::new(5, -1), "0.5"),
            (Decimal::new(0, -2), "0.00"),
            (Decimal::new(23, 0), "23"),
            (Decimal::new(-15, 3), "-15000"),
            (Decimal::new(i64::MIN, -19), "-0.9223372036854775808"),
        ];
        for (value, expected) in cases {
            let mut writer = FieldWriter::new();
            writer.write_decimal(Tag(44), value);

            let message = writer.finish();

            assert_eq!(message.get_str(Tag(44)), Ok(Some(expected)));
            assert_eq!(value.to_string(), expected);
        }
    }

    #[test]
    fn a_written_decimal_reads_back_with_the_same_scale() {
        let mut writer = FieldWriter::new();
        writer.write_decimal(Tag(44), Decimal::new(150, -2));

        let read = writer.finish().get_decimal(Tag(44)).unwrap().unwrap();

        assert_eq!((read.mantissa(), read.exponent()), (150, -2));
    }

    #[test]
    fn writes_the_widest_exponents() {
        let mut writer = FieldWriter::new();
        writer.write_decimal(Tag(44), Decimal::new(1, i8::MAX));
        writer.write_decimal(Tag(31), Decimal::new(1, i8::MIN));

        let message = writer.finish();

        let large = format!("1{}", "0".repeat(127));
        let small = format!("0.{}1", "0".repeat(127));
        assert_eq!(message.get_str(Tag(44)), Ok(Some(&*large)));
        assert_eq!(message.get_str(Tag(31)), Ok(Some(&*small)));
    }

    #[test]
    fn writes_a_date_that_reads_back() {
        let date = Date::new(2026, 11, 3).unwrap();
        let mut writer = FieldWriter::new();
        writer.write_date(Tag(75), date);

        let message = writer.finish();

        assert_eq!(message.get_str(Tag(75)), Ok(Some("20261103")));
        assert_eq!(message.get_date(Tag(75)), Ok(Some(date)));
    }

    #[test]
    fn writes_a_time_with_the_digits_of_its_precision() {
        let tenths = TimePrecision::from_fraction_digits(1).unwrap();
        let cases = [
            (TimePrecision::SECONDS, "09:30:36"),
            (tenths, "09:30:36.2"),
            (TimePrecision::MILLIS, "09:30:36.250"),
            (TimePrecision::MICROS, "09:30:36.250999"),
            (TimePrecision::NANOS, "09:30:36.250999999"),
        ];
        for (precision, expected) in cases {
            let time = Time::new(34_236_250_999_999, precision).unwrap();
            let mut writer = FieldWriter::new();
            writer.write_time(Tag(273), time);

            let message = writer.finish();

            assert_eq!(message.get_str(Tag(273)), Ok(Some(expected)));
            let read = message.get_time(Tag(273)).unwrap().unwrap();
            assert_eq!(read.precision(), precision);
        }
    }

    #[test]
    fn writes_a_leap_second_that_reads_back() {
        let time = Time::new(86_400_500_000_000, TimePrecision::MILLIS).unwrap();
        let mut writer = FieldWriter::new();
        writer.write_time(Tag(273), time);

        let message = writer.finish();

        assert_eq!(message.get_str(Tag(273)), Ok(Some("23:59:60.500")));
        assert_eq!(message.get_time(Tag(273)), Ok(Some(time)));
    }

    #[test]
    fn writes_a_utc_timestamp_that_reads_back() {
        let stamp = UtcTimestamp::new(1_793_370_591_123_456_000, TimePrecision::MICROS);
        let mut writer = FieldWriter::new();
        writer.write_utc_timestamp(Tag(60), stamp);

        let message = writer.finish();

        assert_eq!(
            message.get_str(Tag(60)),
            Ok(Some("20261030-14:29:51.123456"))
        );
        let read = message.get_utc_timestamp(Tag(60)).unwrap().unwrap();
        assert_eq!(read.unix_nanos(), stamp.unix_nanos());
        assert_eq!(read.precision(), TimePrecision::MICROS);
    }

    #[test]
    fn writes_a_utc_timestamp_before_1970() {
        let mut writer = FieldWriter::new();
        writer.write_utc_timestamp(Tag(60), UtcTimestamp::new(-1, TimePrecision::NANOS));

        let message = writer.finish();

        assert_eq!(
            message.get_str(Tag(60)),
            Ok(Some("19691231-23:59:59.999999999"))
        );
    }

    #[test]
    fn writes_a_month_year_in_each_form() {
        let cases = [
            (MonthYear::new(2026, 12).unwrap(), "202612"),
            (MonthYear::with_day(2026, 12, 18).unwrap(), "20261218"),
            (MonthYear::with_week(2026, 12, 3).unwrap(), "202612w3"),
        ];
        for (value, expected) in cases {
            let mut writer = FieldWriter::new();
            writer.write_month_year(Tag(200), value);

            let message = writer.finish();

            assert_eq!(message.get_str(Tag(200)), Ok(Some(expected)));
            assert_eq!(message.get_month_year(Tag(200)), Ok(Some(value)));
        }
    }

    #[test]
    fn writes_the_length_before_a_data_field() {
        let mut writer = FieldWriter::new();
        writer.write_data(Tag(95), Tag(96), b"a\x01b=c").unwrap();

        let message = writer.finish();

        assert_eq!(message.bytes().as_ref(), b"95=5\x0196=a\x01b=c\x01");
        assert_eq!(message.get_int(Tag(95)), Ok(Some(5)));
        assert_eq!(message.get(Tag(96)), Some(&b"a\x01b=c"[..]));
    }

    #[test]
    fn rejects_an_empty_data_value_and_writes_nothing() {
        let mut writer = FieldWriter::new();

        let result = writer.write_data(Tag(95), Tag(96), b"");

        assert_eq!(result, Err(WriteError::EmptyValue { tag: Tag(96) }));
        assert!(writer.finish().bytes().is_empty());
    }
}
