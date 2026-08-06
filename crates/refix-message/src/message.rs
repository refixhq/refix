use crate::value::InvalidValue;
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
#[derive(Clone, Copy, Debug)]
pub(crate) struct RawField {
    pub(crate) tag: u32,
    pub(crate) value_start: u32,
    pub(crate) value_end: u32,
}

/// Raw bytes of a FIX message and an index of every field in wire order.
#[derive(Clone, Debug)]
pub struct RawMessage {
    bytes: Bytes,
    fields: Vec<RawField>,
}

impl RawMessage {
    pub(crate) fn new(bytes: Bytes, fields: Vec<RawField>) -> Self {
        Self { bytes, fields }
    }

    /// The whole frame, from `8=` to the SOH after CheckSum.
    pub fn bytes(&self) -> &Bytes {
        &self.bytes
    }

    /// Value of the first occurrence of `tag`, scanning from the start.
    pub fn get(&self, tag: u32) -> Option<&[u8]> {
        self.find(tag, Slot::START).map(|(_, value)| value)
    }

    /// First occurrence of `tag` as UTF-8 text; `Ok(None)` when absent.
    pub fn get_str(&self, tag: u32) -> Result<Option<&str>, InvalidValue> {
        let Some(value) = self.get(tag) else {
            return Ok(None);
        };
        let value = std::str::from_utf8(value).map_err(|_| InvalidValue { tag })?;
        Ok(Some(value))
    }

    /// First occurrence of `tag` parsed as an integer; `Ok(None)` when absent.
    pub fn get_int(&self, tag: u32) -> Result<Option<i64>, InvalidValue> {
        let Some(value) = self.get_str(tag)? else {
            return Ok(None);
        };
        let value = value.parse().map_err(|_| InvalidValue { tag })?;
        Ok(Some(value))
    }

    /// Every field as `(tag, value)`, in wire order, duplicates included.
    ///
    /// Includes an entry with [`MALFORMED_TAG`] for any byte run that could
    /// not be tokenised, so the entries cover the whole frame.
    pub fn entries(&self) -> impl Iterator<Item = (u32, &[u8])> {
        self.fields
            .iter()
            .map(|&field| (field.tag, self.slice(field)))
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

    /// The index itself, for tests asserting offset-level invariants.
    #[cfg(test)]
    pub(crate) fn raw_fields(&self) -> &[RawField] {
        &self.fields
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framing::SOH;

    fn message_of(fields: &[(u32, &str)]) -> RawMessage {
        let mut bytes = Vec::new();
        let mut index = Vec::new();
        for &(tag, value) in fields {
            bytes.extend_from_slice(tag.to_string().as_bytes());
            bytes.push(b'=');
            let value_start = bytes.len() as u32;
            bytes.extend_from_slice(value.as_bytes());
            let value_end = bytes.len() as u32;
            bytes.push(SOH);
            index.push(RawField {
                tag,
                value_start,
                value_end,
            });
        }
        RawMessage::new(Bytes::from(bytes), index)
    }

    #[test]
    fn get_returns_first_occurrence() {
        let message = message_of(&[(35, "0"), (58, "first"), (58, "second")]);
        assert_eq!(message.get(58), Some(b"first".as_slice()));
    }

    #[test]
    fn get_absent_tag() {
        let message = message_of(&[(35, "0")]);
        assert_eq!(message.get(58), None);
    }
}
