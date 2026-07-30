use crate::framing::{GarbledReason, Outcome, SOH, Scanner};
use bytes::Bytes;

/// Tag recorded for a run of bytes that could not be tokenised into a field.
pub const MALFORMED_TAG: u32 = 0;

/// A field's position in a message's index.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Slot(u32);

impl Slot {
    /// The first field of a message, always `BeginString`.
    pub const START: Slot = Slot(0);

    fn index(self) -> usize {
        self.0 as usize
    }
}

/// The tag and byte range of a field's value within the frame.
#[derive(Clone, Copy)]
pub(crate) struct RawField {
    tag: u32,
    value_start: u32,
    value_end: u32,
}

/// Raw bytes of a FIX message and an index of every field in wire order.
pub struct RawMessage {
    bytes: Bytes,
    fields: Vec<RawField>,
}

impl RawMessage {
    pub(crate) fn new(bytes: Bytes, fields: Vec<RawField>) -> Self {
        Self { bytes, fields }
    }

    /// Value of the first occurrence of `tag`, scanning from the start.
    pub fn get(&self, tag: u32) -> Option<&[u8]> {
        self.find(tag, Slot::START).map(|(_, value)| value)
    }

    /// First occurrence of `tag` at or after `from`, with its slot so the
    /// caller can continue or bound a range.
    fn find(&self, tag: u32, from: Slot) -> Option<(Slot, &[u8])> {
        let rest = self.fields.get(from.index()..)?;
        let offset = rest.iter().position(|field| field.tag == tag)?;
        let slot = Slot((from.index() + offset) as u32);
        Some((slot, self.value(slot)))
    }

    fn value(&self, slot: Slot) -> &[u8] {
        self.slice(self.fields[slot.index()])
    }

    fn slice(&self, field: RawField) -> &[u8] {
        &self.bytes[field.value_start as usize..field.value_end as usize]
    }
}

#[derive(Default)]
pub struct Tokenizer;

/// Issues that can arise when the bytes handed to the tokeniser are not a valid FIX message.
#[derive(Clone, Debug)]
pub enum TokenizeError {
    /// The bytes fail the frame-level checks.
    Garbled(GarbledReason),
    /// The bytes end mid-message.
    Incomplete,
    /// One message was found, but the input continues past it.
    TrailingBytes { frame_len: usize },
    /// The frame is too long for the index's `u32` offsets to address.
    TooLargeToIndex,
}

impl Tokenizer {
    pub fn tokenize(&self, bytes: Bytes) -> Result<RawMessage, TokenizeError> {
        check_frame(&bytes)?;
        let fields = tokenize_fields(&bytes);

        Ok(RawMessage::new(bytes, fields))
    }
}

fn tokenize_fields(bytes: &[u8]) -> Vec<RawField> {
    // TODO: decide what capacity to start with
    let mut fields = Vec::new();
    let mut pos = 0;

    while let Some((field, next)) = next_field(bytes, pos) {
        fields.push(field);
        pos = next;
    }

    fields
}

fn next_field(bytes: &[u8], pos: usize) -> Option<(RawField, usize)> {
    match find_tag_end(bytes, pos) {
        None => None,
        Some(TagEnd::Equals(delimiter_pos)) => {
            let value_end = match find_soh(bytes, delimiter_pos) {
                None => bytes.len(),
                Some(end) => end,
            };
            let tag = parse_u32(&bytes[pos..delimiter_pos]).unwrap_or(MALFORMED_TAG);
            let field = RawField {
                tag,
                value_start: (delimiter_pos + 1) as u32,
                value_end: value_end as u32,
            };

            Some((field, value_end + 1))
        }
        Some(TagEnd::Soh(delimiter_pos)) => {
            let field = RawField {
                tag: MALFORMED_TAG,
                value_start: pos as u32,
                value_end: delimiter_pos as u32,
            };

            Some((field, delimiter_pos + 1))
        }
    }
}

/// Parses ASCII digits into a `u32`. Rejects empty input, any non-digit
/// (including a leading sign) and overflow.
fn parse_u32(digits: &[u8]) -> Option<u32> {
    if digits.is_empty() || !digits.iter().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.iter().try_fold(0u32, |acc, &b| {
        acc.checked_mul(10)?.checked_add(u32::from(b - b'0'))
    })
}

enum TagEnd {
    Equals(usize),
    Soh(usize),
}

/// Offset of the first `=` or SOH at or after `from`.
///
/// This is specifically used to find the ending byte of a tag,
/// which would normally be `=`, but potentially a SOH in malformed
/// fields.
fn find_tag_end(bytes: &[u8], from: usize) -> Option<TagEnd> {
    bytes
        .get(from..)?
        .iter()
        .enumerate()
        .find_map(|(rel, &byte)| match byte {
            b'=' => Some(TagEnd::Equals(from + rel)),
            SOH => Some(TagEnd::Soh(from + rel)),
            _ => None,
        })
}

/// Offset for the first SOH at or after `from`.
fn find_soh(bytes: &[u8], from: usize) -> Option<usize> {
    bytes
        .get(from..)?
        .iter()
        .position(|&b| b == SOH)
        .map(|rel| from + rel)
}

fn check_frame(bytes: &Bytes) -> Result<(), TokenizeError> {
    if u32::try_from(bytes.len()).is_err() {
        return Err(TokenizeError::TooLargeToIndex);
    }

    let scanner = Scanner::new(bytes.len());
    match scanner.scan(bytes) {
        Outcome::Frame(frame) => {
            if frame.bytes.len() != bytes.len() {
                Err(TokenizeError::TrailingBytes {
                    frame_len: frame.bytes.len(),
                })
            } else {
                Ok(())
            }
        }
        Outcome::Incomplete => Err(TokenizeError::Incomplete),
        Outcome::Garbled { reason, .. } => Err(TokenizeError::Garbled(reason)),
    }
}

#[cfg(test)]
mod tests {
    use crate::test_utils::{construct_valid_frame, to_wire};
    use crate::{RawMessage, Tokenizer};
    use bytes::Bytes;

    /// Tokenises a `|`-delimited body wrapped in a valid FIX.4.4 frame,
    /// asserting the invariant every index must satisfy.
    #[track_caller]
    fn tokenize_body(body: &str) -> RawMessage {
        let frame = construct_valid_frame("FIX.4.4", body);
        let message = Tokenizer.tokenize(Bytes::from(frame)).unwrap();
        assert_tiles(&message);
        message
    }

    /// Every field of the frame, in wire order.
    #[track_caller]
    fn assert_entries(message: &RawMessage, expected: &[(u32, &str)]) {
        let expected: Vec<(u32, String)> = expected
            .iter()
            .map(|&(tag, value)| (tag, value.to_owned()))
            .collect();
        assert_eq!(entries_of(message), expected);
    }

    /// The fields between the preamble (8, 9) and the trailer (10), so bodies
    /// can be asserted without hand-computing BodyLength and CheckSum.
    #[track_caller]
    fn assert_body_entries(message: &RawMessage, expected: &[(u32, &str)]) {
        let entries = entries_of(message);
        let tags: Vec<u32> = entries.iter().map(|&(tag, _)| tag).collect();
        assert_eq!(
            tags[..2],
            [8, 9],
            "frame must open with BeginString, BodyLength"
        );
        assert_eq!(*tags.last().unwrap(), 10, "frame must close with CheckSum");

        let body: Vec<(u32, &str)> = entries[2..entries.len() - 1]
            .iter()
            .map(|(tag, value)| (*tag, value.as_str()))
            .collect();
        assert_eq!(body, expected);
    }

    fn entries_of(message: &RawMessage) -> Vec<(u32, String)> {
        message
            .fields
            .iter()
            .map(|&field| {
                (
                    field.tag,
                    String::from_utf8_lossy(message.slice(field)).into_owned(),
                )
            })
            .collect()
    }

    /// The index tiles the frame: each field starts one byte past the
    /// previous field's value, and the last value's SOH ends the frame.
    #[track_caller]
    fn assert_tiles(message: &RawMessage) {
        let mut expected_tag_start = 0;
        for (i, &field) in message.fields.iter().enumerate() {
            assert!(
                field.value_start >= expected_tag_start,
                "slot {i} starts before the previous field ended",
            );
            assert!(
                field.value_end >= field.value_start,
                "slot {i} has an inverted value range",
            );
            expected_tag_start = field.value_end + 1;
        }
        assert_eq!(
            expected_tag_start as usize,
            message.bytes.len(),
            "index stops short of the frame end",
        );
    }

    mod well_formed {
        use super::*;

        #[test]
        fn valid_message() {
            let frame = to_wire("8=FIX.4.4|9=41|35=0|49=A|56=B|34=1|52=20260730-10:00:00|10=123|");
            let message = Tokenizer.tokenize(Bytes::from(frame)).unwrap();
            assert_tiles(&message);

            let expected_fields = [
                (8, "FIX.4.4"),
                (9, "41"),
                (35, "0"),
                (49, "A"),
                (56, "B"),
                (34, "1"),
                (52, "20260730-10:00:00"),
                (10, "123"),
            ];

            assert_entries(&message, &expected_fields);
        }

        #[test]
        fn empty_value() {
            let message = tokenize_body("35=0|58=|");
            assert_body_entries(&message, &[(35, "0"), (58, "")]);
        }

        #[test]
        fn value_containing_equals() {
            let message = tokenize_body("35=0|58=px=1.23|");
            assert_body_entries(&message, &[(35, "0"), (58, "px=1.23")]);
        }

        #[test]
        fn duplicate_tags_all_indexed() {
            let message = tokenize_body("35=0|58=first|58=second|");
            assert_body_entries(&message, &[(35, "0"), (58, "first"), (58, "second")]);
        }
    }
}
