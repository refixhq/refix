use crate::Tag;
use crate::value::InvalidValue;
use bytes::Bytes;

/// A field's position in a message's index.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Slot(u32);

impl Slot {
    /// The first field of a message, always `BeginString`.
    pub const START: Slot = Slot(0);

    fn index(self) -> usize {
        self.0 as usize
    }

    /// The slot after this one.
    pub(crate) fn next(self) -> Slot {
        Slot(self.0 + 1)
    }
}

/// The tag and byte range of a field's value within the frame.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RawField {
    pub(crate) tag: Tag,
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

    /// The whole frame as a [`Scope`].
    pub fn scope(&self) -> Scope<'_> {
        Scope {
            message: self,
            start: Slot::START,
            end: Slot(self.fields.len() as u32),
        }
    }

    /// Value of the first occurrence of `tag`, scanning from the start.
    pub fn get(&self, tag: Tag) -> Option<&[u8]> {
        self.scope().get(tag)
    }

    /// First occurrence of `tag` as UTF-8 text or `Ok(None)` when absent.
    pub fn get_str(&self, tag: Tag) -> Result<Option<&str>, InvalidValue> {
        self.scope().get_str(tag)
    }

    /// First occurrence of `tag` parsed as an integer or `Ok(None)` when absent.
    pub fn get_int(&self, tag: Tag) -> Result<Option<i64>, InvalidValue> {
        self.scope().get_int(tag)
    }

    /// Every field as `(tag, value)`, in wire order, duplicates included.
    ///
    /// Includes an entry with [`Tag::MALFORMED`] for any byte run that could
    /// not be tokenised, so the entries cover the whole frame.
    pub fn entries(&self) -> impl Iterator<Item = (Tag, &[u8])> {
        self.fields
            .iter()
            .map(|&field| (field.tag, self.slice(field)))
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

/// A bounded view of a message's fields - the whole frame or one group
/// instance.
#[derive(Clone, Copy, Debug)]
pub struct Scope<'a> {
    message: &'a RawMessage,
    start: Slot,
    end: Slot,
}

impl<'a> Scope<'a> {
    /// Value of the first occurrence of `tag` in this scope.
    pub fn get(&self, tag: Tag) -> Option<&'a [u8]> {
        self.find(tag, self.start).map(|(_, value)| value)
    }

    /// First occurrence of `tag` as UTF-8 text or `Ok(None)` when absent.
    pub fn get_str(&self, tag: Tag) -> Result<Option<&'a str>, InvalidValue> {
        let Some(value) = self.get(tag) else {
            return Ok(None);
        };
        let value = std::str::from_utf8(value).map_err(|_| InvalidValue { tag })?;
        Ok(Some(value))
    }

    /// First occurrence of `tag` parsed as an integer or `Ok(None)` when absent.
    pub fn get_int(&self, tag: Tag) -> Result<Option<i64>, InvalidValue> {
        let Some(value) = self.get_str(tag)? else {
            return Ok(None);
        };
        let value = value.parse().map_err(|_| InvalidValue { tag })?;
        Ok(Some(value))
    }

    /// First occurrence of `tag` at or after `from` and before the end of
    /// the scope, with its slot so the caller can continue or bound a range.
    pub(crate) fn find(&self, tag: Tag, from: Slot) -> Option<(Slot, &'a [u8])> {
        let fields = self.message.fields.get(from.index()..self.end.index())?;
        let offset = fields.iter().position(|field| field.tag == tag)?;
        let slot = Slot((from.index() + offset) as u32);
        Some((slot, self.message.value(slot)))
    }

    /// The fields `start..end` of the same message.
    pub(crate) fn narrow(&self, start: Slot, end: Slot) -> Scope<'a> {
        Scope {
            message: self.message,
            start,
            end,
        }
    }

    pub(crate) fn start(&self) -> Slot {
        self.start
    }

    pub(crate) fn end(&self) -> Slot {
        self.end
    }

    pub(crate) fn tag_at(&self, slot: Slot) -> Tag {
        self.message.fields[slot.index()].tag
    }

    pub(crate) fn value_at(&self, slot: Slot) -> &'a [u8] {
        self.message.value(slot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framing::SOH;

    fn message_of<V: AsRef<[u8]>>(fields: &[(u32, V)]) -> RawMessage {
        let mut bytes = Vec::new();
        let mut index = Vec::new();
        for (tag, value) in fields {
            bytes.extend_from_slice(tag.to_string().as_bytes());
            bytes.push(b'=');
            let value_start = bytes.len() as u32;
            bytes.extend_from_slice(value.as_ref());
            let value_end = bytes.len() as u32;
            bytes.push(SOH);
            index.push(RawField {
                tag: Tag(*tag),
                value_start,
                value_end,
            });
        }
        RawMessage::new(Bytes::from(bytes), index)
    }

    #[test]
    fn get_returns_first_occurrence() {
        let message = message_of(&[(35, "0"), (58, "first"), (58, "second")]);
        assert_eq!(message.get(Tag(58)), Some(b"first".as_slice()));
    }

    #[test]
    fn get_absent_tag() {
        let message = message_of(&[(35, "0")]);
        assert_eq!(message.get(Tag(58)), None);
    }

    #[test]
    fn get_str_returns_text() {
        let message = message_of(&[(58, "hello")]);
        assert_eq!(message.get_str(Tag(58)), Ok(Some("hello")));
    }

    #[test]
    fn get_str_absent_tag() {
        let message = message_of(&[(35, "0")]);
        assert_eq!(message.get_str(Tag(58)), Ok(None));
    }

    #[test]
    fn get_str_empty_value() {
        let message = message_of(&[(58, "")]);
        assert_eq!(message.get_str(Tag(58)), Ok(Some("")));
    }

    #[test]
    fn get_str_rejects_invalid_utf8() {
        let message = message_of(&[(58, b"caf\xE9".as_slice())]);
        assert_eq!(message.get_str(Tag(58)), Err(InvalidValue { tag: Tag(58) }));
    }

    #[test]
    fn get_int_parses_digits() {
        let message = message_of(&[(38, "200")]);
        assert_eq!(message.get_int(Tag(38)), Ok(Some(200)));
    }

    #[test]
    fn get_int_parses_a_negative_value() {
        let message = message_of(&[(38, "-5")]);
        assert_eq!(message.get_int(Tag(38)), Ok(Some(-5)));
    }

    #[test]
    fn get_int_absent_tag() {
        let message = message_of(&[(35, "0")]);
        assert_eq!(message.get_int(Tag(38)), Ok(None));
    }

    #[test]
    fn get_int_rejects_garbage() {
        let message = message_of(&[(38, "12x3")]);
        assert_eq!(message.get_int(Tag(38)), Err(InvalidValue { tag: Tag(38) }));
    }

    #[test]
    fn get_int_rejects_an_empty_value() {
        let message = message_of(&[(38, "")]);
        assert_eq!(message.get_int(Tag(38)), Err(InvalidValue { tag: Tag(38) }));
    }

    #[test]
    fn get_int_rejects_invalid_utf8() {
        let message = message_of(&[(38, b"\xE9".as_slice())]);
        assert_eq!(message.get_int(Tag(38)), Err(InvalidValue { tag: Tag(38) }));
    }

    mod scope {
        use super::*;

        /// A scope over the fields in slots `start..end` of `message`.
        fn scope_of(message: &RawMessage, start: u32, end: u32) -> Scope<'_> {
            Scope {
                message,
                start: Slot(start),
                end: Slot(end),
            }
        }

        #[test]
        fn a_scope_does_not_read_past_its_end() {
            // Two party instances; only the second has a PartyRole(452).
            let message = message_of(&[(453, "2"), (448, "AL"), (448, "BOB"), (452, "3")]);
            let first_instance = scope_of(&message, 1, 2);

            assert_eq!(first_instance.get(Tag(448)), Some(b"AL".as_slice()));
            assert_eq!(first_instance.get(Tag(452)), None);
        }

        #[test]
        fn a_scope_does_not_read_before_its_start() {
            let message = message_of(&[(453, "2"), (448, "AL"), (448, "BOB"), (452, "3")]);
            let second_instance = scope_of(&message, 2, 4);

            assert_eq!(second_instance.get(Tag(448)), Some(b"BOB".as_slice()));
            assert_eq!(second_instance.get(Tag(453)), None);
        }

        #[test]
        fn conversions_stay_within_the_scope() {
            let message = message_of(&[(38, "100"), (58, "first"), (38, "x"), (58, "second")]);
            let tail = scope_of(&message, 2, 4);

            assert_eq!(tail.get_str(Tag(58)), Ok(Some("second")));
            assert_eq!(tail.get_int(Tag(38)), Err(InvalidValue { tag: Tag(38) }));
        }

        #[test]
        fn an_empty_scope_finds_nothing() {
            let message = message_of(&[(35, "D")]);
            assert_eq!(scope_of(&message, 1, 1).get(Tag(35)), None);
        }

        #[test]
        fn the_whole_frame_scope_reads_like_the_message() {
            let message = message_of(&[(35, "D"), (38, "200"), (58, "hello")]);
            let scope = message.scope();

            assert_eq!(scope.get(Tag(35)), message.get(Tag(35)));
            assert_eq!(scope.get_int(Tag(38)), Ok(Some(200)));
            assert_eq!(scope.get_str(Tag(58)), Ok(Some("hello")));
            assert_eq!(scope.get(Tag(99)), None);
        }

        #[test]
        fn values_outlive_the_scope_they_were_read_from() {
            let message = message_of(&[(58, "kept")]);
            let value = {
                let scope = message.scope();
                scope.get(Tag(58))
            };

            assert_eq!(value, Some(b"kept".as_slice()));
        }
    }
}
