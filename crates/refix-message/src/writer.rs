mod error;

use crate::framing::SOH;
use crate::message::RawField;
use crate::{RawMessage, Tag};
use bytes::Bytes;
pub use error::WriteError;

/// Writes fields and gives them back as a [`RawMessage`].
///
/// The fields are written in the order of the calls.
///
/// ```
/// use refix_message::{FieldWriter, Tag};
///
/// let mut body = FieldWriter::new();
/// body.write_str(Tag(11), "ORDER-1")?;
/// body.write_str(Tag(55), "ACME")?;
/// let body = body.finish();
///
/// assert_eq!(body.to_string(), "11=ORDER-1|55=ACME");
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
        let mut digits = [0; 10];
        self.bytes.extend_from_slice(decimal(&mut digits, tag.0));
        self.bytes.push(b'=');
        let value_start = self.bytes.len();
        self.bytes.extend_from_slice(value);
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
        Ok(())
    }

    /// Writes a field with `value` as its text.
    ///
    /// Fails and panics as [`write`](Self::write) does.
    pub fn write_str(&mut self, tag: Tag, value: &str) -> Result<(), WriteError> {
        self.write(tag, value.as_bytes())
    }

    /// The fields written, as a message.
    pub fn finish(self) -> RawMessage {
        RawMessage::new(Bytes::from(self.bytes), self.fields)
    }
}

/// The decimal digits of `number`, written at the end of `buffer`.
fn decimal(buffer: &mut [u8; 10], mut number: u32) -> &[u8] {
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
}
