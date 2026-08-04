use crate::framing::{GarbledReason, Outcome, SOH, Scanner};
use crate::message::RawField;
use crate::{MALFORMED_TAG, RawMessage, length_tags};
use bytes::Bytes;

/// Splits a framed FIX message into its fields.
#[derive(Clone, Debug)]
pub struct Tokenizer {
    length_tags: Vec<u32>,
}

impl Default for Tokenizer {
    fn default() -> Self {
        Self {
            length_tags: length_tags::STANDARD.to_vec(),
        }
    }
}

impl Tokenizer {
    /// Constructs a [`Tokenizer`] with additional (non-standard) length tags.
    ///
    /// The supplied values are additive, they never replace the standard set.
    /// Tag 0 is reserved as the malformed-field sentinel and is ignored.
    pub fn with_extra_length_tags(extras: impl IntoIterator<Item = u32>) -> Self {
        let mut length_tags: Vec<u32> = length_tags::STANDARD
            .iter()
            .copied()
            .chain(extras.into_iter().filter(|&tag| tag != MALFORMED_TAG))
            .collect();
        length_tags.sort_unstable();
        length_tags.dedup();
        Self { length_tags }
    }

    /// Tokenises exactly one complete FIX message into a [`RawMessage`].
    ///
    /// The bytes must contain a single well-framed message.
    /// Frame-level issues surface as [`TokenizeError`], while malformed
    /// fields inside a valid frame are indexed under [`MALFORMED_TAG`].
    pub fn tokenize(&self, bytes: Bytes) -> Result<RawMessage, TokenizeError> {
        check_frame(&bytes)?;
        let fields = self.tokenize_fields(&bytes);

        Ok(RawMessage::new(bytes, fields))
    }

    fn tokenize_fields(&self, bytes: &[u8]) -> Vec<RawField> {
        let mut fields = Vec::with_capacity(bytes.len() / 8);
        let mut pos = 0;
        let mut data_len: Option<usize> = None;

        while let Some((field, next)) = next_field(bytes, pos, data_len.take()) {
            data_len = self.pending_data_len(bytes, field);
            fields.push(field);
            pos = next;
        }

        fields
    }

    /// The value of `field` when it is a well-formed length tag, so the next
    /// field's value can be read by extent instead of scanned for SOH.
    fn pending_data_len(&self, bytes: &[u8], field: RawField) -> Option<usize> {
        if !self.is_length_tag(field.tag) {
            return None;
        }
        let value = &bytes[field.value_start as usize..field.value_end as usize];
        parse_u32(value).map(|len| len as usize)
    }

    fn is_length_tag(&self, tag: u32) -> bool {
        self.length_tags.first().is_some_and(|&min| tag >= min)
            && self.length_tags.binary_search(&tag).is_ok()
    }
}

/// Issues that can arise when the bytes handed to the tokeniser are not a valid FIX message.
#[derive(Clone, Copy, Debug, PartialEq)]
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

fn next_field(bytes: &[u8], pos: usize, data_len: Option<usize>) -> Option<(RawField, usize)> {
    match find_tag_end(bytes, pos) {
        None => None,
        Some(TagEnd::Equals(delimiter_pos)) => {
            let tag = parse_u32(&bytes[pos..delimiter_pos]).unwrap_or(MALFORMED_TAG);
            let value_start = delimiter_pos + 1;
            let value_end = data_len
                .and_then(|len| data_value_end(bytes, value_start, len))
                .or_else(|| find_soh(bytes, value_start))
                .unwrap_or(bytes.len());
            let field = RawField {
                tag,
                value_start: value_start as u32,
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

/// Parses ASCII digits into a `u32`.
///
/// Rejects empty input, any non-digit (including a leading sign) and overflow.
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

/// End of a length-delimited value.
///
/// Returns `None` unless the byte immediately after the claimed extent is SOH.
/// A length that overruns the frame or lands mid-value is distrusted and the
/// caller falls back to SOH scanning.
fn data_value_end(bytes: &[u8], value_start: usize, len: usize) -> Option<usize> {
    let end = value_start.checked_add(len)?;
    (bytes.get(end) == Some(&SOH)).then_some(end)
}

fn check_frame(bytes: &Bytes) -> Result<(), TokenizeError> {
    let scanner = Scanner::new(u32::MAX as usize);
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
        Outcome::Garbled {
            reason: GarbledReason::FrameTooLarge,
            ..
        } => Err(TokenizeError::TooLargeToIndex),
        Outcome::Garbled { reason, .. } => Err(TokenizeError::Garbled(reason)),
    }
}

#[cfg(test)]
mod tests {
    use crate::test_utils::{construct_valid_frame, to_wire};
    use crate::{RawMessage, TokenizeError, Tokenizer};
    use bytes::Bytes;

    /// Tokenises a `|`-delimited body wrapped in a valid FIX.4.4 frame,
    /// asserting the invariant every index must satisfy.
    #[track_caller]
    fn tokenize_body(body: &str) -> RawMessage {
        let frame = construct_valid_frame("FIX.4.4", body);
        let message = Tokenizer::default().tokenize(Bytes::from(frame)).unwrap();
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
            .entries()
            .map(|(tag, value)| (tag, String::from_utf8_lossy(value).into_owned()))
            .collect()
    }

    /// The index tiles the frame: each field starts one byte past the
    /// previous field's value, and the last value's SOH ends the frame.
    #[track_caller]
    fn assert_tiles(message: &RawMessage) {
        let mut expected_tag_start = 0;
        for (i, &field) in message.raw_fields().iter().enumerate() {
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
            message.bytes().len(),
            "index stops short of the frame end",
        );
    }

    mod well_formed {
        use super::*;

        #[test]
        fn valid_message() {
            let frame = to_wire("8=FIX.4.4|9=41|35=0|49=A|56=B|34=1|52=20260730-10:00:00|10=123|");
            let message = Tokenizer::default().tokenize(Bytes::from(frame)).unwrap();
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

        #[test]
        fn preamble_and_trailer_are_ordinary_fields() {
            let message = tokenize_body("35=0|");
            assert_eq!(message.get(8), Some(b"FIX.4.4".as_slice()));
            assert_eq!(message.get(9), Some(b"5".as_slice()));
            assert_eq!(message.get(35), Some(b"0".as_slice()));
            assert!(message.get(10).is_some());
        }
    }

    mod errors {
        use super::*;

        #[test]
        fn truncated_frame_is_incomplete() {
            let mut bytes = construct_valid_frame("FIX.4.4", "35=0|");
            bytes.pop();

            let error = Tokenizer::default()
                .tokenize(Bytes::from(bytes))
                .unwrap_err();
            assert_eq!(error, TokenizeError::Incomplete);
        }

        #[test]
        fn absurd_body_length_is_too_large_to_index() {
            let bytes = to_wire("8=FIX.4.4|9=4294967295|35=0|");

            let error = Tokenizer::default()
                .tokenize(Bytes::from(bytes))
                .unwrap_err();
            assert_eq!(error, TokenizeError::TooLargeToIndex);
        }

        #[test]
        fn two_messages_are_trailing_bytes() {
            let mut bytes = construct_valid_frame("FIX.4.4", "35=0|");
            let frame_len = bytes.len();
            bytes.extend_from_slice(&construct_valid_frame("FIX.4.4", "35=0|"));

            let error = Tokenizer::default()
                .tokenize(Bytes::from(bytes))
                .unwrap_err();
            assert_eq!(error, TokenizeError::TrailingBytes { frame_len });
        }
    }

    /// Tests for the MALFORMED_TAG sentinel value.
    mod sentinel_runs {
        use super::*;
        use crate::message::MALFORMED_TAG;

        #[test]
        fn run_without_equals() {
            let message = tokenize_body("35=0|junk|58=ok|");
            assert_body_entries(&message, &[(35, "0"), (MALFORMED_TAG, "junk"), (58, "ok")]);
        }

        #[test]
        fn fields_after_a_fault_remain_readable() {
            let message = tokenize_body("35=0|junk|58=ok|");
            assert_eq!(message.get(58), Some(b"ok".as_slice()));
        }

        #[test]
        fn non_numeric_tag() {
            let message = tokenize_body("35=0|abc=x|");
            assert_body_entries(&message, &[(35, "0"), (MALFORMED_TAG, "x")]);
        }

        #[test]
        fn empty_tag() {
            let message = tokenize_body("35=0|=x|");
            assert_body_entries(&message, &[(35, "0"), (MALFORMED_TAG, "x")]);
        }

        #[test]
        fn literal_tag_zero_is_a_sentinel() {
            let message = tokenize_body("35=0|0=x|");
            assert_body_entries(&message, &[(35, "0"), (MALFORMED_TAG, "x")]);
        }

        #[test]
        fn tag_overflowing_u32() {
            let message = tokenize_body("35=0|4294967296=x|");
            assert_body_entries(&message, &[(35, "0"), (MALFORMED_TAG, "x")]);
        }

        #[test]
        fn consecutive_runs() {
            let message = tokenize_body("35=0|junk|more|58=ok|");
            assert_body_entries(
                &message,
                &[
                    (35, "0"),
                    (MALFORMED_TAG, "junk"),
                    (MALFORMED_TAG, "more"),
                    (58, "ok"),
                ],
            );
        }
    }

    /// Tests for length-delimited data fields, whose values may contain SOH.
    mod data_fields {
        use super::*;
        use crate::message::MALFORMED_TAG;

        #[test]
        fn value_with_embedded_soh() {
            let message = tokenize_body("35=0|95=3|96=a|b|58=ok|");
            assert_body_entries(
                &message,
                &[(35, "0"), (95, "3"), (96, "a\x01b"), (58, "ok")],
            );
        }

        #[test]
        fn value_with_several_sohs() {
            let message = tokenize_body("35=0|95=5|96=a|b|c|58=ok|");
            assert_body_entries(
                &message,
                &[(35, "0"), (95, "5"), (96, "a\x01b\x01c"), (58, "ok")],
            );
        }

        #[test]
        fn zero_length_value() {
            let message = tokenize_body("35=0|95=0|96=|58=ok|");
            assert_body_entries(&message, &[(35, "0"), (95, "0"), (96, ""), (58, "ok")]);
        }

        #[test]
        fn data_value_last_in_body() {
            let message = tokenize_body("35=0|95=3|96=a|b|");
            assert_body_entries(&message, &[(35, "0"), (95, "3"), (96, "a\x01b")]);
        }

        #[test]
        fn overrunning_length_is_distrusted() {
            let message = tokenize_body("35=0|95=4294967295|96=ab|58=ok|");
            assert_body_entries(
                &message,
                &[(35, "0"), (95, "4294967295"), (96, "ab"), (58, "ok")],
            );
        }

        #[test]
        fn non_numeric_length_falls_back_to_scanning() {
            let message = tokenize_body("35=0|95=abc|96=x|y|58=ok|");
            assert_body_entries(
                &message,
                &[
                    (35, "0"),
                    (95, "abc"),
                    (96, "x"),
                    (MALFORMED_TAG, "y"),
                    (58, "ok"),
                ],
            );
        }

        #[test]
        fn short_length_surfaces_junk_after_the_data() {
            let message = tokenize_body("35=0|95=1|96=a|b|58=ok|");
            assert_body_entries(
                &message,
                &[
                    (35, "0"),
                    (95, "1"),
                    (96, "a"),
                    (MALFORMED_TAG, "b"),
                    (58, "ok"),
                ],
            );
        }

        #[test]
        fn dialect_extra_delimits_data() {
            let frame = construct_valid_frame("FIX.4.4", "35=0|5001=3|5002=a|b|58=ok|");
            let message = Tokenizer::with_extra_length_tags([5001])
                .tokenize(Bytes::from(frame))
                .unwrap();
            assert_tiles(&message);
            assert_body_entries(
                &message,
                &[(35, "0"), (5001, "3"), (5002, "a\x01b"), (58, "ok")],
            );
        }

        #[test]
        fn standard_set_survives_extras() {
            let frame = construct_valid_frame("FIX.4.4", "35=0|95=3|96=a|b|");
            let message = Tokenizer::with_extra_length_tags([5001])
                .tokenize(Bytes::from(frame))
                .unwrap();
            assert_tiles(&message);
            assert_body_entries(&message, &[(35, "0"), (95, "3"), (96, "a\x01b")]);
        }

        #[test]
        fn extra_below_the_standard_minimum() {
            let frame = construct_valid_frame("FIX.4.4", "35=0|42=3|58=a|b|11=ok|");
            let message = Tokenizer::with_extra_length_tags([42])
                .tokenize(Bytes::from(frame))
                .unwrap();
            assert_tiles(&message);
            assert_body_entries(
                &message,
                &[(35, "0"), (42, "3"), (58, "a\x01b"), (11, "ok")],
            );
        }

        #[test]
        fn tag_zero_extra_is_ignored() {
            let tokenizer = Tokenizer::with_extra_length_tags([0]);
            assert!(!tokenizer.is_length_tag(MALFORMED_TAG));
        }
    }
}
