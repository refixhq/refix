use super::{FieldWriter, WriteError, WrittenSize};
use crate::{RawMessage, Tag};

const BEGIN_STRING: Tag = Tag(8);
const BODY_LENGTH: Tag = Tag(9);
const MSG_TYPE: Tag = Tag(35);
const CHECK_SUM: Tag = Tag(10);

/// The most bytes that `8=`, `9=` with its digits, `10=000` and their separators take.
const FRAMING_LEN: usize = 23;

/// Frames a header and a body into a whole message.
///
/// Writes BeginString (8), BodyLength (9) and MsgType (35), then the fields of `header` and of
/// `body`, then CheckSum (10).
///
/// It is an error for `header` or `body` to hold one of those four fields.
/// `begin_string` and `msg_type` are checked as [`FieldWriter::write_str`] checks a value.
///
/// ```
/// use refix_message::{FieldWriter, Tag, frame};
///
/// let mut header = FieldWriter::new();
/// header.write_str(Tag(49), "BUYSIDE")?;
/// header.write_str(Tag(56), "VENUE")?;
/// header.write_int(Tag(34), 7);
/// let mut body = FieldWriter::new();
/// body.write_str(Tag(11), "ORDER-1")?;
///
/// let message = frame("FIX.4.4", "D", &header.finish(), &body.finish())?;
///
/// assert_eq!(
///     message.to_string(),
///     "8=FIX.4.4|9=41|35=D|49=BUYSIDE|56=VENUE|34=7|11=ORDER-1|10=041"
/// );
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Panics
///
/// If the message passes 4 GiB.
pub fn frame(
    begin_string: &str,
    msg_type: &str,
    header: &RawMessage,
    body: &RawMessage,
) -> Result<RawMessage, WriteError> {
    for part in [header, body] {
        if let Some(field) = part.raw_fields().iter().find(|field| is_framing(field.tag)) {
            return Err(WriteError::ReservedTag { tag: field.tag });
        }
    }
    let header_size = WrittenSize::of(header);
    let body_size = WrittenSize::of(body);
    let counted = "35=".len() + msg_type.len() + 1 + header_size.len + body_size.len;
    let len = begin_string.len() + counted + FRAMING_LEN;
    let fields = 4 + header.raw_fields().len() + body.raw_fields().len();

    let mut writer = FieldWriter::with_capacity(len, fields);
    writer.write_str(BEGIN_STRING, begin_string)?;
    writer.write_int(BODY_LENGTH, counted as i64);
    writer.write_str(MSG_TYPE, msg_type)?;
    writer.append(header, &header_size);
    writer.append(body, &body_size);
    writer.check_sum();
    Ok(writer.finish())
}

fn is_framing(tag: Tag) -> bool {
    matches!(tag, BEGIN_STRING | BODY_LENGTH | MSG_TYPE | CHECK_SUM)
}

impl FieldWriter {
    /// Writes CheckSum (10) for the bytes written so far.
    fn check_sum(&mut self) {
        let sum = self
            .bytes
            .iter()
            .fold(0u8, |sum, &byte| sum.wrapping_add(byte));
        let value_start = self.begin(CHECK_SUM);
        self.bytes
            .extend_from_slice(&[b'0' + sum / 100, b'0' + sum / 10 % 10, b'0' + sum % 10]);
        self.end(CHECK_SUM, value_start);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Tokenizer;
    use crate::test_utils::construct_valid_frame;
    use bytes::Bytes;

    fn header(seq: i64) -> RawMessage {
        let mut writer = FieldWriter::new();
        writer.write_str(Tag(49), "BUYSIDE").unwrap();
        writer.write_str(Tag(56), "VENUE").unwrap();
        writer.write_int(Tag(34), seq);
        writer.finish()
    }

    fn body() -> RawMessage {
        let mut writer = FieldWriter::new();
        writer.write_str(Tag(11), "ORDER-1").unwrap();
        writer.write_str(Tag(55), "ACME").unwrap();
        writer.finish()
    }

    fn empty() -> RawMessage {
        FieldWriter::new().finish()
    }

    #[test]
    fn frames_with_the_body_length_and_check_sum_of_the_bytes() {
        // Enough sequence numbers to pass through every width of both.
        for seq in 1..=1_000 {
            let message = frame("FIX.4.4", "D", &header(seq), &body()).unwrap();

            let fields = format!("35=D|49=BUYSIDE|56=VENUE|34={seq}|11=ORDER-1|55=ACME|");
            let expected = construct_valid_frame("FIX.4.4", &fields);
            assert_eq!(message.bytes().as_ref(), expected);
        }
    }

    #[test]
    fn a_framed_message_reads_back_through_the_tokenizer() {
        let mut body = FieldWriter::new();
        body.write_str(Tag(58), "signed").unwrap();
        body.write_data(Tag(95), Tag(96), b"a\x01b=c").unwrap();
        let message = frame("FIX.4.4", "B", &header(7), &body.finish()).unwrap();

        let read = Tokenizer::default()
            .tokenize(message.bytes().clone())
            .unwrap();

        assert!(read.entries().eq(message.entries()));
        assert_eq!(read.get(Tag(96)), Some(&b"a\x01b=c"[..]));
        assert_eq!(message.get_str(Tag(35)), Ok(Some("B")));
    }

    #[test]
    fn frames_a_message_with_no_body_fields() {
        let message = frame("FIX.4.4", "0", &header(8), &empty()).unwrap();

        assert_eq!(
            message.to_string(),
            "8=FIX.4.4|9=30|35=0|49=BUYSIDE|56=VENUE|34=8|10=154"
        );
    }

    #[test]
    fn frames_fields_read_with_another_separator() {
        let bytes = &b"11=ORDER-1|55=ACME|"[..];
        let fields = Tokenizer::default().tokenize_fields(bytes, b'|');
        let read = RawMessage::new(Bytes::from_static(bytes), fields);

        let message = frame("FIX.4.4", "D", &header(7), &read).unwrap();

        let expected = frame("FIX.4.4", "D", &header(7), &body()).unwrap();
        assert_eq!(message.bytes(), expected.bytes());
    }

    #[test]
    fn rejects_a_field_that_framing_writes() {
        for tag in [BEGIN_STRING, BODY_LENGTH, MSG_TYPE, CHECK_SUM] {
            let mut writer = FieldWriter::new();
            writer.write_str(tag, "1").unwrap();
            let fields = writer.finish();
            let error = Some(WriteError::ReservedTag { tag });

            assert_eq!(frame("FIX.4.4", "D", &fields, &body()).err(), error);
            assert_eq!(frame("FIX.4.4", "D", &header(7), &fields).err(), error);
        }
    }

    #[test]
    fn checks_the_begin_string_and_the_msg_type() {
        let empty_type = frame("FIX.4.4", "", &header(7), &body());
        let split_begin = frame("FIX\x014.4", "D", &header(7), &body());

        assert_eq!(
            empty_type.err(),
            Some(WriteError::EmptyValue { tag: MSG_TYPE })
        );
        assert_eq!(
            split_begin.err(),
            Some(WriteError::SeparatorInValue { tag: BEGIN_STRING })
        );
    }
}
