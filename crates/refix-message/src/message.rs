use crate::framing::{Outcome, SOH, Scanner};
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
///
/// The index doesn't form any opinions on architecture.
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
    pub fn find(&self, tag: u32, from: Slot) -> Option<(Slot, &[u8])> {
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

pub struct TokenizeError;

impl Tokenizer {
    pub fn tokenize(&self, bytes: Bytes) -> Result<RawMessage, TokenizeError> {
        check_frame(&bytes)?;
        let fields = tokenize_fields(&bytes);

        Ok(RawMessage::new(bytes, fields))
    }
}

fn tokenize_fields(bytes: &[u8]) -> Vec<RawField> {
    let mut fields = Vec::with_capacity(256);
    let mut pos = 0;

    while pos < bytes.len() {
        match next_field(bytes, pos) {
            Some((field, next)) => {
                fields.push(field);
                pos = next;
            }
            None => {
                todo!()
            }
        }
    }

    fields
}

fn next_field(bytes: &[u8], pos: usize) -> Option<(RawField, usize)> {
    match find_delimiter(bytes, pos) {
        None => None,
        Some((delimiter_pos, b'=')) => {
            let value_end = match find_soh(bytes, delimiter_pos) {
                None => bytes.len(),
                Some(end) => end,
            };
            let tag = parse_u32(&bytes[pos..delimiter_pos]).unwrap();
            let field = RawField {
                tag,
                value_start: (delimiter_pos + 1) as u32,
                value_end: value_end as u32,
            };

            Some((field, value_end))
        }
        Some((delimiter_pos, SOH)) => {
            let field = RawField {
                tag: MALFORMED_TAG,
                value_start: pos as u32,
                value_end: delimiter_pos as u32,
            };

            Some((field, delimiter_pos))
        }
        Some(_) => panic!("unexpected delimiter - this can never happen"),
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

/// Offset of the first `=` or SOH at or after `from`, with the byte found.
fn find_delimiter(bytes: &[u8], from: usize) -> Option<(usize, u8)> {
    bytes[from..]
        .iter()
        .position(|&b| b == b'=' || b == SOH)
        .map(|rel| (from + rel, bytes[from + rel]))
}

/// Offset for the first SOH at or after `from`.
fn find_soh(bytes: &[u8], from: usize) -> Option<usize> {
    bytes[from..]
        .iter()
        .position(|&b| b == b'=' || b == SOH)
        .map(|rel| from + rel)
}

fn check_frame(bytes: &Bytes) -> Result<(), TokenizeError> {
    let scanner = Scanner::new(bytes.len());
    match scanner.scan(bytes) {
        Outcome::Frame(frame) => {
            if frame.bytes.len() != bytes.len() {
                Err(TokenizeError)
            } else {
                Ok(())
            }
        }
        Outcome::Incomplete => Err(TokenizeError),
        Outcome::Garbled { .. } => Err(TokenizeError),
    }
}
