use std::fmt::{self, Write};

use crate::{Date, Decimal, InvalidValue, KnownTags, MonthYear, Tag, Time, UtcTimestamp};
use bytes::Bytes;

/// A field's position in a message's index.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Slot(u32);

impl Slot {
    /// The first field of a message.
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
#[derive(Clone)]
pub struct RawMessage {
    bytes: Bytes,
    fields: Vec<RawField>,
}

impl RawMessage {
    pub(crate) fn new(bytes: Bytes, fields: Vec<RawField>) -> Self {
        Self { bytes, fields }
    }

    /// The bytes the message was read from.
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

    /// The scope over the fields from `start` up to `end`, or `None` if
    /// they don't fit this message.
    pub fn scope_between(&self, start: Slot, end: Slot) -> Option<Scope<'_>> {
        (start <= end && end.index() <= self.fields.len()).then_some(Scope {
            message: self,
            start,
            end,
        })
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

    /// First occurrence of `tag` parsed as a boolean or `Ok(None)` when absent.
    pub fn get_bool(&self, tag: Tag) -> Result<Option<bool>, InvalidValue> {
        self.scope().get_bool(tag)
    }

    /// First occurrence of `tag` parsed as a decimal or `Ok(None)` when absent.
    pub fn get_decimal(&self, tag: Tag) -> Result<Option<Decimal>, InvalidValue> {
        self.scope().get_decimal(tag)
    }

    /// First occurrence of `tag` parsed as a UTC timestamp or `Ok(None)` when absent.
    pub fn get_utc_timestamp(&self, tag: Tag) -> Result<Option<UtcTimestamp>, InvalidValue> {
        self.scope().get_utc_timestamp(tag)
    }

    /// First occurrence of `tag` parsed as a date or `Ok(None)` when absent.
    pub fn get_date(&self, tag: Tag) -> Result<Option<Date>, InvalidValue> {
        self.scope().get_date(tag)
    }

    /// First occurrence of `tag` parsed as a time of day or `Ok(None)` when absent.
    pub fn get_time(&self, tag: Tag) -> Result<Option<Time>, InvalidValue> {
        self.scope().get_time(tag)
    }

    /// First occurrence of `tag` parsed as a month or `Ok(None)` when absent.
    pub fn get_month_year(&self, tag: Tag) -> Result<Option<MonthYear>, InvalidValue> {
        self.scope().get_month_year(tag)
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

    /// The bytes of the field at `index`, from its tag to the end of its value.
    fn field_text(&self, index: usize) -> &[u8] {
        let start = match index.checked_sub(1) {
            Some(previous) => self.fields[previous].value_end as usize + 1,
            None => 0,
        };
        let end = self.fields[index].value_end as usize;
        self.bytes.get(start..end).unwrap_or_default()
    }

    /// The index itself.
    pub(crate) fn raw_fields(&self) -> &[RawField] {
        &self.fields
    }
}

impl fmt::Display for RawMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.scope().fmt(f)
    }
}

impl fmt::Debug for RawMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RawMessage({self})")
    }
}

/// A bounded view of a message's fields - the whole frame or one group
/// instance.
#[derive(Clone, Copy)]
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

    /// First occurrence of `tag` parsed as a boolean or `Ok(None)` when absent.
    pub fn get_bool(&self, tag: Tag) -> Result<Option<bool>, InvalidValue> {
        match self.get(tag) {
            None => Ok(None),
            Some(b"Y") => Ok(Some(true)),
            Some(b"N") => Ok(Some(false)),
            Some(_) => Err(InvalidValue { tag }),
        }
    }

    /// First occurrence of `tag` parsed as a decimal or `Ok(None)` when absent.
    pub fn get_decimal(&self, tag: Tag) -> Result<Option<Decimal>, InvalidValue> {
        let Some(value) = self.get(tag) else {
            return Ok(None);
        };

        let value = Decimal::parse(value).ok_or(InvalidValue { tag })?;
        Ok(Some(value))
    }

    /// First occurrence of `tag` parsed as a UTC timestamp or `Ok(None)` when absent.
    pub fn get_utc_timestamp(&self, tag: Tag) -> Result<Option<UtcTimestamp>, InvalidValue> {
        let Some(value) = self.get(tag) else {
            return Ok(None);
        };

        let value = UtcTimestamp::parse(value).ok_or(InvalidValue { tag })?;
        Ok(Some(value))
    }

    /// First occurrence of `tag` parsed as a date or `Ok(None)` when absent.
    pub fn get_date(&self, tag: Tag) -> Result<Option<Date>, InvalidValue> {
        let Some(value) = self.get(tag) else {
            return Ok(None);
        };

        let value = Date::parse(value).ok_or(InvalidValue { tag })?;
        Ok(Some(value))
    }

    /// First occurrence of `tag` parsed as a time of day or `Ok(None)` when absent.
    pub fn get_time(&self, tag: Tag) -> Result<Option<Time>, InvalidValue> {
        let Some(value) = self.get(tag) else {
            return Ok(None);
        };

        let value = Time::parse(value).ok_or(InvalidValue { tag })?;
        Ok(Some(value))
    }

    /// First occurrence of `tag` parsed as a month or `Ok(None)` when absent.
    pub fn get_month_year(&self, tag: Tag) -> Result<Option<MonthYear>, InvalidValue> {
        let Some(value) = self.get(tag) else {
            return Ok(None);
        };

        let value = MonthYear::parse(value).ok_or(InvalidValue { tag })?;
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

    /// The slot of this scope's first field.
    pub fn start(&self) -> Slot {
        self.start
    }

    /// The slot just past this scope's last field.
    pub fn end(&self) -> Slot {
        self.end
    }

    pub(crate) fn tag_at(&self, slot: Slot) -> Tag {
        self.message.fields[slot.index()].tag
    }

    pub(crate) fn value_at(&self, slot: Slot) -> &'a [u8] {
        self.message.value(slot)
    }

    /// Displays the fields whose tags are in `known`, as sent.
    pub fn display_known<'k>(&self, known: &KnownTags<'k>) -> impl fmt::Display + use<'a, 'k> {
        DisplayKnown {
            scope: *self,
            known: *known,
        }
    }

    // Writes the fields `keep` accepts, joined by pipes.
    fn write_fields(&self, f: &mut fmt::Formatter<'_>, keep: impl Fn(Tag) -> bool) -> fmt::Result {
        let mut first = true;
        for index in self.start.index()..self.end.index() {
            if !keep(self.message.fields[index].tag) {
                continue;
            }
            if !first {
                f.write_char('|')?;
            }
            first = false;
            for &byte in self.message.field_text(index) {
                match byte {
                    b'\\' => f.write_str("\\\\")?,
                    b' '..=b'~' => f.write_char(char::from(byte))?,
                    _ => write!(f, "\\x{byte:02x}")?,
                }
            }
        }
        Ok(())
    }
}

impl fmt::Display for Scope<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write_fields(f, |_| true)
    }
}

struct DisplayKnown<'a, 'k> {
    scope: Scope<'a>,
    known: KnownTags<'k>,
}

impl fmt::Display for DisplayKnown<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.scope.write_fields(f, |tag| self.known.contains(tag))
    }
}

impl fmt::Debug for Scope<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Scope({self})")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TimePrecision;
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

    #[test]
    fn get_bool_parses_y_and_n() {
        let message = message_of(&[(43, "Y"), (97, "N")]);
        assert_eq!(message.get_bool(Tag(43)), Ok(Some(true)));
        assert_eq!(message.get_bool(Tag(97)), Ok(Some(false)));
    }

    #[test]
    fn get_bool_absent_tag() {
        let message = message_of(&[(35, "0")]);
        assert_eq!(message.get_bool(Tag(43)), Ok(None));
    }

    #[test]
    fn get_bool_rejects_anything_but_y_or_n() {
        for value in ["y", "n", "1", "0", "YES", "true", ""] {
            let message = message_of(&[(43, value)]);
            assert_eq!(
                message.get_bool(Tag(43)),
                Err(InvalidValue { tag: Tag(43) }),
                "{value:?}"
            );
        }
    }

    #[test]
    fn get_decimal_parses_a_price() {
        let message = message_of(&[(44, "101.25")]);
        assert_eq!(
            message.get_decimal(Tag(44)),
            Ok(Some(Decimal::new(10125, -2)))
        );
    }

    #[test]
    fn get_decimal_absent_tag() {
        let message = message_of(&[(35, "0")]);
        assert_eq!(message.get_decimal(Tag(44)), Ok(None));
    }

    #[test]
    fn get_decimal_rejects_garbage() {
        let message = message_of(&[(44, "1e5")]);
        assert_eq!(
            message.get_decimal(Tag(44)),
            Err(InvalidValue { tag: Tag(44) })
        );
    }

    #[test]
    fn get_utc_timestamp_parses_a_sending_time() {
        let message = message_of(&[(52, "20261030-14:29:51.123")]);
        assert_eq!(
            message.get_utc_timestamp(Tag(52)),
            Ok(Some(UtcTimestamp::new(
                1_793_370_591_123_000_000,
                TimePrecision::MILLIS
            )))
        );
    }

    #[test]
    fn get_utc_timestamp_absent_tag() {
        let message = message_of(&[(35, "0")]);
        assert_eq!(message.get_utc_timestamp(Tag(52)), Ok(None));
    }

    #[test]
    fn get_utc_timestamp_rejects_garbage() {
        let message = message_of(&[(52, "2026-10-30T14:29:51Z")]);
        assert_eq!(
            message.get_utc_timestamp(Tag(52)),
            Err(InvalidValue { tag: Tag(52) })
        );
    }

    #[test]
    fn get_date_parses_a_trade_date() {
        let message = message_of(&[(75, "20261030")]);
        assert_eq!(message.get_date(Tag(75)), Ok(Date::new(2026, 10, 30)));
    }

    #[test]
    fn get_date_absent_tag() {
        let message = message_of(&[(35, "0")]);
        assert_eq!(message.get_date(Tag(75)), Ok(None));
    }

    #[test]
    fn get_date_rejects_garbage() {
        let message = message_of(&[(75, "2026-10-30")]);
        assert_eq!(
            message.get_date(Tag(75)),
            Err(InvalidValue { tag: Tag(75) })
        );
    }

    #[test]
    fn get_time_parses_an_entry_time() {
        let message = message_of(&[(273, "14:29:51.123")]);
        assert_eq!(
            message.get_time(Tag(273)),
            Ok(Time::new(52_191_123_000_000, TimePrecision::MILLIS))
        );
    }

    #[test]
    fn get_time_absent_tag() {
        let message = message_of(&[(35, "0")]);
        assert_eq!(message.get_time(Tag(273)), Ok(None));
    }

    #[test]
    fn get_time_rejects_garbage() {
        let message = message_of(&[(273, "14:29:51Z")]);
        assert_eq!(
            message.get_time(Tag(273)),
            Err(InvalidValue { tag: Tag(273) })
        );
    }

    #[test]
    fn get_month_year_parses_a_maturity() {
        let message = message_of(&[(200, "202612")]);
        assert_eq!(
            message.get_month_year(Tag(200)),
            Ok(MonthYear::new(2026, 12))
        );
    }

    #[test]
    fn get_month_year_absent_tag() {
        let message = message_of(&[(35, "0")]);
        assert_eq!(message.get_month_year(Tag(200)), Ok(None));
    }

    #[test]
    fn get_month_year_rejects_garbage() {
        let message = message_of(&[(200, "2026-12")]);
        assert_eq!(
            message.get_month_year(Tag(200)),
            Err(InvalidValue { tag: Tag(200) })
        );
    }

    #[test]
    fn a_message_prints_its_fields_joined_by_pipes() {
        let message = message_of(&[(8, "FIX.4.4"), (35, "0"), (10, "161")]);
        assert_eq!(message.to_string(), "8=FIX.4.4|35=0|10=161");
    }

    #[test]
    fn debug_wraps_the_printed_message() {
        let message = message_of(&[(8, "FIX.4.4"), (35, "0")]);
        assert_eq!(format!("{message:?}"), "RawMessage(8=FIX.4.4|35=0)");
    }

    #[test]
    fn an_empty_message_prints_nothing() {
        let message = message_of::<&str>(&[]);
        assert_eq!(message.to_string(), "");
    }

    #[test]
    fn a_byte_outside_printable_ascii_prints_as_an_escape() {
        let message = message_of(&[(58, b"caf\xE9".as_slice()), (96, b"a\x01b".as_slice())]);
        assert_eq!(message.to_string(), r"58=caf\xe9|96=a\x01b");
    }

    #[test]
    fn a_backslash_prints_doubled() {
        let message = message_of(&[(58, r"a\b")]);
        assert_eq!(message.to_string(), r"58=a\\b");
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
        fn a_scope_rebuilds_from_its_bounds() {
            let message = message_of(&[(38, "100"), (58, "first"), (38, "x"), (58, "second")]);
            let tail = scope_of(&message, 2, 4);

            let rebuilt = message.scope_between(tail.start(), tail.end()).unwrap();

            assert_eq!(rebuilt.get_str(Tag(58)), Ok(Some("second")));
        }

        #[test]
        fn bounds_that_do_not_fit_the_message_are_rejected() {
            let message = message_of(&[(38, "100"), (58, "first")]);

            assert!(message.scope_between(Slot(0), Slot(3)).is_none());
            assert!(message.scope_between(Slot(2), Slot(1)).is_none());
            assert!(message.scope_between(Slot(2), Slot(2)).is_some());
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
        fn a_scope_prints_only_its_fields() {
            let message = message_of(&[(453, "2"), (448, "AL"), (452, "3"), (448, "BOB")]);
            let first_instance = scope_of(&message, 1, 3);

            assert_eq!(first_instance.to_string(), "448=AL|452=3");
            assert_eq!(format!("{first_instance:?}"), "Scope(448=AL|452=3)");
        }

        #[test]
        fn an_empty_scope_prints_nothing() {
            let message = message_of(&[(35, "D")]);
            assert_eq!(scope_of(&message, 1, 1).to_string(), "");
        }

        #[test]
        fn display_known_prints_the_fields_with_a_known_tag_as_sent() {
            let message = message_of(&[(8, "FIX.4.4"), (35, "D"), (55, "ACME"), (49, "AL")]);
            let known = KnownTags::new(&[Tag(8), Tag(35), Tag(49)]);

            let printed = message.scope().display_known(&known).to_string();

            assert_eq!(printed, "8=FIX.4.4|35=D|49=AL");
        }

        #[test]
        fn display_known_prints_every_field_with_a_repeated_tag() {
            let message = message_of(&[(627, "2"), (628, "A"), (11, "ORD"), (628, "B")]);
            let known = KnownTags::new(&[Tag(627), Tag(628)]);

            let printed = message.scope().display_known(&known).to_string();

            assert_eq!(printed, "627=2|628=A|628=B");
        }

        #[test]
        fn display_known_stays_within_the_scope() {
            let message = message_of(&[(453, "2"), (448, "AL"), (452, "3"), (448, "BOB")]);
            let known = KnownTags::new(&[Tag(448)]);

            let printed = scope_of(&message, 1, 3).display_known(&known).to_string();

            assert_eq!(printed, "448=AL");
        }

        #[test]
        fn display_known_prints_nothing_when_no_tag_is_known() {
            let message = message_of(&[(35, "D"), (55, "ACME")]);
            let known = KnownTags::new(&[Tag(49)]);

            assert_eq!(message.scope().display_known(&known).to_string(), "");
        }

        #[test]
        fn display_known_escapes_as_a_scope_does() {
            let message = message_of(&[(58, b"caf\xE9".as_slice()), (96, b"a\\b".as_slice())]);
            let known = KnownTags::new(&[Tag(58), Tag(96)]);

            let printed = message.scope().display_known(&known).to_string();

            assert_eq!(printed, r"58=caf\xe9|96=a\\b");
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
