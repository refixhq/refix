//! Reading messages out of FIX logs.
//!
//! Loggers write a timestamp, a direction or a session before each message,
//! and other output between them.
//!
//! ```text
//! 20261103-09:30:01.123: 8=FIX.4.4|9=...|10=123|
//! 2026-11-03 09:30:01,124 INFO  [fix-session] Received: 8=FIX.4.4|9=...|10=045|
//! 2026-11-03 09:30:05,002 WARN  Heartbeat timeout, sending TestRequest
//! 2026-11-03 09:30:05,003 INFO  [fix-session] Sent: 8=FIX.4.4|9=...|10=210|
//! ```
//!
//! A [`LogReader`] keeps these bytes as the prefix of each [`LogRecord`]. Here
//! the last record's prefix holds the WARN line and its own line up to `8=`.

mod end;
mod start;

use crate::framing::SOH;
use crate::message::RawField;
use crate::tokenizer::parse_u32;
use crate::{Dialect, RawMessage, Tag, Tokenizer};
use bytes::{Buf, Bytes, BytesMut};
use end::find_end;
use start::{NextStart, find_start};

const BODY_LENGTH: Tag = Tag(9);
const CHECKSUM: Tag = Tag(10);
const MSG_TYPE: Tag = Tag(35);

/// Longest message read before it's cut.
const MAX_MESSAGE_LEN: usize = 1 << 20;

/// Longest prefix kept, nearest its message.
const MAX_PREFIX_LEN: usize = 1 << 20;

/// Reads the messages in a FIX log from incrementally fed bytes.
#[derive(Default)]
pub struct LogReader {
    tokenizer: Tokenizer,
    separator: Option<u8>,
    buf: BytesMut,
    /// Where the search for the next message start resumes.
    searched: usize,
    /// Whether the next record's prefix lost bytes to the cap.
    prefix_truncated: bool,
}

/// A message read from a log, with the bytes before it.
#[derive(Clone, Debug)]
pub struct LogRecord {
    /// The bytes between the previous record and this message.
    pub prefix: Bytes,
    /// The message, split at the separator it was logged with.
    pub message: RawMessage,
    /// The problems with the record, in the order found.
    pub diagnostics: Vec<LogDiagnostic>,
}

/// A problem a [`LogReader`] found with a record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogDiagnostic {
    /// The second field is not `9=`.
    MissingBodyLength,
    /// BodyLength is not a number.
    MalformedBodyLength,
    /// BodyLength does not point at the CheckSum field.
    BodyLengthMismatch,
    /// The field after BodyLength, or after BeginString without one, is not
    /// `35=`.
    MissingMsgType,
    /// CheckSum is not three digits.
    MalformedChecksum,
    /// The calculated checksum does not match the value of CheckSum.
    ChecksumMismatch,
    /// The message ends before its CheckSum field.
    Truncated,
    /// The prefix is cut to the 1 MiB nearest the message.
    PrefixTruncated,
}

impl LogReader {
    /// A reader for `dialect`.
    ///
    /// Each message's separator is detected from the byte after its
    /// BeginString, unless `separator` is given.
    pub fn new(dialect: Dialect<'_>, separator: Option<u8>) -> Self {
        Self {
            tokenizer: Tokenizer::new(dialect),
            separator,
            buf: BytesMut::new(),
            searched: 0,
            prefix_truncated: false,
        }
    }

    /// Appends bytes to the reader's buffer.
    pub fn feed(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    /// Consumes and returns the next record, or `None` until more bytes are
    /// fed.
    pub fn next_record(&mut self) -> Option<LogRecord> {
        self.read(false)
    }

    /// Ends the input and returns the records still pending.
    ///
    /// Bytes after the last message are dropped.
    pub fn finish(mut self) -> impl Iterator<Item = LogRecord> {
        std::iter::from_fn(move || self.read(true))
    }

    fn read(&mut self, at_end: bool) -> Option<LogRecord> {
        let range = self.searched..self.buf.len();
        let next = find_start(&self.buf, range, self.separator, at_end);
        let prefix_len = match next {
            NextStart::Found(start) => start.at,
            NextStart::Pending(at) => at,
            NextStart::NotFound => self.buf.len(),
        };
        let prefix_len = self.cap_prefix(prefix_len);
        self.searched = prefix_len;
        let NextStart::Found(start) = next else {
            return None;
        };

        let rest = &self.buf[prefix_len..];
        let end = find_end(
            &self.tokenizer,
            rest,
            start.separator,
            self.separator,
            at_end,
        )?;
        let prefix = self.buf.split_to(prefix_len).freeze();
        let bytes = self.buf.split_to(end.len).freeze();
        self.buf.advance(end.consumed - end.len);
        self.searched = 0;

        let mut diagnostics = Vec::new();
        if std::mem::take(&mut self.prefix_truncated) {
            diagnostics.push(LogDiagnostic::PrefixTruncated);
        }
        let fields = self.tokenizer.tokenize_fields(&bytes, start.separator);
        diagnostics.extend(diagnose(&bytes, &fields, start.separator, end.truncated));
        Some(LogRecord {
            prefix,
            message: RawMessage::new(bytes, fields),
            diagnostics,
        })
    }

    /// Drops the oldest bytes of a prefix of `len` bytes past the cap, and
    /// returns its length after.
    fn cap_prefix(&mut self, len: usize) -> usize {
        if len <= MAX_PREFIX_LEN {
            return len;
        }
        self.buf.advance(len - MAX_PREFIX_LEN);
        self.prefix_truncated = true;
        MAX_PREFIX_LEN
    }
}

/// What could not be verified about the message in `bytes`.
fn diagnose(
    bytes: &[u8],
    fields: &[RawField],
    separator: u8,
    truncated: bool,
) -> Vec<LogDiagnostic> {
    let value = |field: &RawField| &bytes[field.value_start as usize..field.value_end as usize];
    let mut diagnostics = Vec::new();

    let body_length = fields.get(1).filter(|field| field.tag == BODY_LENGTH);
    let declared_length = body_length.and_then(|field| parse_u32(value(field)));
    if body_length.is_none() {
        diagnostics.push(LogDiagnostic::MissingBodyLength);
    } else if declared_length.is_none() {
        diagnostics.push(LogDiagnostic::MalformedBodyLength);
    }

    let msg_type_index = if body_length.is_some() { 2 } else { 1 };
    if fields.get(msg_type_index).map(|field| field.tag) != Some(MSG_TYPE) {
        diagnostics.push(LogDiagnostic::MissingMsgType);
    }

    if truncated {
        diagnostics.push(LogDiagnostic::Truncated);
        return diagnostics;
    }

    // A message that isn't truncated ends with its CheckSum field.
    let Some((checksum, before)) = fields.split_last() else {
        return diagnostics;
    };
    let trailer_start = before
        .last()
        .map_or(0, |field| field.value_end as usize + 1);
    if let (Some(field), Some(declared)) = (body_length, declared_length)
        && field.value_end as usize + 1 + declared as usize != trailer_start
    {
        diagnostics.push(LogDiagnostic::BodyLengthMismatch);
    }

    match value(checksum) {
        stated @ [b'0'..=b'9', b'0'..=b'9', b'0'..=b'9'] => {
            let calculated = calculate_checksum(&bytes[..trailer_start], separator);
            if parse_u32(stated) != Some(u32::from(calculated)) {
                diagnostics.push(LogDiagnostic::ChecksumMismatch);
            }
        }
        _ => diagnostics.push(LogDiagnostic::MalformedChecksum),
    }
    diagnostics
}

/// The checksum of `bytes`, counting each separator as SOH.
fn calculate_checksum(bytes: &[u8], separator: u8) -> u8 {
    bytes
        .iter()
        .map(|&byte| if byte == separator { SOH } else { byte })
        .fold(0, u8::wrapping_add)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{calculate_checksum as checksum_of, construct_valid_frame, to_wire};

    /// A FIX.4.4 message with `body`, its fields ended by `separator`.
    fn message(body: &str, separator: u8) -> Vec<u8> {
        with_separator(construct_valid_frame("FIX.4.4", body), separator)
    }

    /// `head` and a correct CheckSum, with fields ended by `separator`.
    fn with_trailer(head: &str, separator: u8) -> Vec<u8> {
        let mut bytes = to_wire(head);
        let checksum = checksum_of(&bytes);
        bytes.extend_from_slice(&to_wire(&format!("10={checksum}|")));
        with_separator(bytes, separator)
    }

    fn with_separator(bytes: Vec<u8>, separator: u8) -> Vec<u8> {
        bytes
            .into_iter()
            .map(|byte| if byte == SOH { separator } else { byte })
            .collect()
    }

    /// Every record in `log`, fed in one piece.
    fn read(log: &[u8]) -> Vec<LogRecord> {
        read_with(LogReader::default(), log)
    }

    fn read_with(mut reader: LogReader, log: &[u8]) -> Vec<LogRecord> {
        reader.feed(log);
        let mut records: Vec<LogRecord> = std::iter::from_fn(|| reader.next_record()).collect();
        records.extend(reader.finish());
        records
    }

    #[track_caller]
    fn assert_record(
        record: &LogRecord,
        prefix: &[u8],
        message: &[u8],
        diagnostics: &[LogDiagnostic],
    ) {
        assert_eq!(record.prefix.as_ref(), prefix, "prefix");
        assert_eq!(record.message.bytes().as_ref(), message, "message");
        assert_eq!(record.diagnostics, diagnostics, "diagnostics");
    }

    #[test]
    fn reads_a_quickfix_file_log() {
        let first = message("35=A|34=1|", SOH);
        let second = message("35=0|34=2|", SOH);
        let log = [
            b"20261103-14:29:51.123 : ".as_slice(),
            &first,
            b"\n20261103-14:29:52.456 : ",
            &second,
            b"\n",
        ]
        .concat();

        let records = read(&log);

        assert_eq!(records.len(), 2);
        assert_record(&records[0], b"20261103-14:29:51.123 : ", &first, &[]);
        assert_record(&records[1], b"20261103-14:29:52.456 : ", &second, &[]);
    }

    #[test]
    fn reads_any_single_byte_separator() {
        for separator in [b'|', b';', b' ', b'^'] {
            let message = message("35=0|58=hi|", separator);

            let records = read(&message);

            assert_eq!(records.len(), 1, "separator {}", separator as char);
            assert_record(&records[0], b"", &message, &[]);
            assert_eq!(records[0].message.get(Tag(58)), Some(b"hi".as_slice()));
        }
    }

    #[test]
    fn detects_the_separator_of_each_message() {
        let first = message("35=0|", SOH);
        let second = message("35=0|", b'|');
        let log = [first.as_slice(), b"\n", &second, b"\n"].concat();

        let records = read(&log);

        assert_eq!(records.len(), 2);
        assert_record(&records[0], b"", &first, &[]);
        assert_record(&records[1], b"", &second, &[]);
    }

    #[test]
    fn a_given_separator_is_the_only_one_read() {
        let first = message("35=0|", SOH);
        let second = message("35=0|", b'|');
        let log = [first.as_slice(), b"\n", &second].concat();

        let records = read_with(LogReader::new(Dialect::new(), Some(b'|')), &log);

        assert_eq!(records.len(), 1);
        assert_record(
            &records[0],
            &[first.as_slice(), b"\n"].concat(),
            &second,
            &[],
        );
    }

    #[test]
    fn reads_data_fields_by_length() {
        let message = message("35=0|95=3|96=a|b|", b'|');

        let records = read(&message);

        assert_record(&records[0], b"", &message, &[]);
        assert_eq!(records[0].message.get(Tag(96)), Some(b"a|b".as_slice()));
    }

    #[test]
    fn text_mentioning_fix_is_not_a_message() {
        let message = message("35=0|", SOH);
        let prefix = b"58=FIX is down, 8=FIX protocol\n";
        let log = [prefix.as_slice(), &message].concat();

        let records = read(&log);

        assert_eq!(records.len(), 1);
        assert_record(&records[0], prefix, &message, &[]);
    }

    #[test]
    fn a_line_break_after_a_message_belongs_to_its_record() {
        let first = message("35=0|", SOH);
        let second = message("35=0|", SOH);
        let third = message("35=0|", SOH);
        let log = [first.as_slice(), b"\r\nIN ", &second, b"\nOUT ", &third].concat();

        let records = read(&log);

        assert_eq!(records.len(), 3);
        assert_record(&records[1], b"IN ", &second, &[]);
        assert_record(&records[2], b"OUT ", &third, &[]);
    }

    #[test]
    fn bytes_after_the_last_message_are_dropped() {
        let message = message("35=0|", SOH);
        let log = [message.as_slice(), b"\nshutting down\n"].concat();

        let records = read(&log);

        assert_eq!(records.len(), 1);
        assert_record(&records[0], b"", &message, &[]);
    }

    #[test]
    fn a_value_with_a_line_break_ends_where_body_length_says() {
        let message = message("35=0|58=two\nlines|", SOH);

        let records = read(&message);

        assert_eq!(records.len(), 1);
        assert_record(&records[0], b"", &message, &[]);
    }

    #[test]
    fn a_checksum_without_its_separator_ends_at_the_line_break() {
        let first = message("35=0|", b'|');
        let second = message("35=0|", b'|');
        let first_without = &first[..first.len() - 1];
        let log = [first_without, b"\r\n", &second].concat();

        let records = read(&log);

        assert_eq!(records.len(), 2);
        assert_record(&records[0], b"", first_without, &[]);
        assert_record(&records[1], b"", &second, &[]);
    }

    #[test]
    fn a_carriage_return_ending_the_log_is_not_part_of_the_message() {
        let message = message("35=0|", b'|');
        let without = &message[..message.len() - 1];
        let log = [without, b"\r"].concat();

        let records = read(&log);

        assert_eq!(records.len(), 1);
        assert_record(&records[0], b"", without, &[]);
    }

    mod diagnostics {
        use super::*;

        #[test]
        fn a_rewritten_checksum_is_a_mismatch() {
            let mut message = message("35=0|", b'|');
            let last_digit = message.len() - 2;
            message[last_digit] = if message[last_digit] == b'9' {
                b'0'
            } else {
                message[last_digit] + 1
            };

            let records = read(&message);

            assert_record(
                &records[0],
                b"",
                &message,
                &[LogDiagnostic::ChecksumMismatch],
            );
        }

        #[test]
        fn a_checksum_that_isnt_three_digits_is_malformed() {
            let message = b"8=FIX.4.4|9=5|35=0|10=1x3|";

            let records = read(message);

            assert_record(
                &records[0],
                b"",
                message,
                &[LogDiagnostic::MalformedChecksum],
            );
        }

        #[test]
        fn a_wrong_body_length_is_a_mismatch() {
            let message = with_trailer("8=FIX.4.4|9=2|35=0|58=hi|", b'|');

            let records = read(&message);

            assert_record(
                &records[0],
                b"",
                &message,
                &[LogDiagnostic::BodyLengthMismatch],
            );
        }

        #[test]
        fn a_body_length_past_the_bytes_fed_holds_records_until_the_end() {
            let first = with_trailer("8=FIX.4.4|9=999|35=0|", b'|');
            let second = message("35=0|", b'|');
            let mut reader = LogReader::default();
            reader.feed(&[first.as_slice(), b"\n", &second].concat());

            assert!(reader.next_record().is_none());
            let records: Vec<LogRecord> = reader.finish().collect();

            assert_eq!(records.len(), 2);
            assert_record(
                &records[0],
                b"",
                &first,
                &[LogDiagnostic::BodyLengthMismatch],
            );
            assert_record(&records[1], b"", &second, &[]);
        }

        #[test]
        fn a_missing_body_length() {
            let message = with_trailer("8=FIX.4.4|35=0|", b'|');

            let records = read(&message);

            assert_record(
                &records[0],
                b"",
                &message,
                &[LogDiagnostic::MissingBodyLength],
            );
        }

        #[test]
        fn a_malformed_body_length() {
            let message = with_trailer("8=FIX.4.4|9=x|35=0|", b'|');

            let records = read(&message);

            assert_record(
                &records[0],
                b"",
                &message,
                &[LogDiagnostic::MalformedBodyLength],
            );
        }

        #[test]
        fn a_missing_msg_type() {
            let message = with_trailer("8=FIX.4.4|9=5|49=A|", b'|');

            let records = read(&message);

            assert_record(&records[0], b"", &message, &[LogDiagnostic::MissingMsgType]);
        }
    }

    mod truncation {
        use super::*;

        #[test]
        fn a_message_cut_at_a_line_break() {
            let cut = b"8=FIX.4.4|9=20|35=0|58=hel";
            let second = message("35=0|", b'|');
            let log = [b"IN ".as_slice(), cut, b"\nOUT ", &second, b"\n"].concat();

            let records = read(&log);

            assert_eq!(records.len(), 2);
            assert_record(&records[0], b"IN ", cut, &[LogDiagnostic::Truncated]);
            assert_record(&records[1], b"OUT ", &second, &[]);
        }

        #[test]
        fn a_message_cut_by_the_next_one() {
            let cut = b"8=FIX.4.4|9=20|35=0|58=hel";
            let second = message("35=0|", b'|');
            let log = [cut.as_slice(), &second].concat();

            let records = read(&log);

            assert_eq!(records.len(), 2);
            assert_record(&records[0], b"", cut, &[LogDiagnostic::Truncated]);
            assert_record(&records[1], b"", &second, &[]);
        }

        #[test]
        fn a_message_cut_by_the_end_of_the_log() {
            let cut = b"8=FIX.4.4|9=20|35=0|58=hel";
            let mut reader = LogReader::default();
            reader.feed(cut);

            assert!(reader.next_record().is_none());
            let records: Vec<LogRecord> = reader.finish().collect();

            assert_eq!(records.len(), 1);
            assert_record(&records[0], b"", cut, &[LogDiagnostic::Truncated]);
        }

        #[test]
        fn a_message_cut_by_the_end_of_the_log_before_its_line_break() {
            let cut = b"8=FIX.4.4|9=20|35=0|58=hel";

            let records = read(&[cut.as_slice(), b"\r"].concat());

            assert_eq!(records.len(), 1);
            assert_record(&records[0], b"", cut, &[LogDiagnostic::Truncated]);
        }

        #[test]
        fn a_message_cut_before_msg_type() {
            let cut = b"8=FIX.4.4|9=20|";

            let records = read(cut);

            assert_record(
                &records[0],
                b"",
                cut,
                &[LogDiagnostic::MissingMsgType, LogDiagnostic::Truncated],
            );
        }
    }

    mod message_cap {
        use super::*;

        #[test]
        fn a_message_reaching_the_cap_is_cut_there() {
            let long = message(&format!("35=0|58={}|", "x".repeat(MAX_MESSAGE_LEN)), b'|');
            let second = message("35=0|", b'|');
            let log = [long.as_slice(), b"\n", &second].concat();

            let records = read(&log);

            assert_eq!(records.len(), 2);
            assert_record(
                &records[0],
                b"",
                &long[..MAX_MESSAGE_LEN],
                &[LogDiagnostic::Truncated],
            );
            let rest = [&long[MAX_MESSAGE_LEN..], b"\n"].concat();
            assert_record(&records[1], &rest, &second, &[]);
        }

        #[test]
        fn a_message_starting_across_the_cap_is_read() {
            let head = "8=FIX.4.4|9=20|35=0|58=";
            let cut = [head, &"z".repeat(MAX_MESSAGE_LEN - head.len() - 3)].concat();
            let second = message("35=0|", b'|');
            let log = [cut.as_bytes(), &second].concat();

            let records = read(&log);

            assert_eq!(records.len(), 2);
            assert_record(
                &records[0],
                b"",
                cut.as_bytes(),
                &[LogDiagnostic::Truncated],
            );
            assert_record(&records[1], b"", &second, &[]);
        }

        #[test]
        fn a_message_ending_at_the_cap_takes_its_line_break() {
            let long = message(
                &format!("35=0|58={}|", "x".repeat(MAX_MESSAGE_LEN - 36)),
                b'|',
            );
            assert_eq!(long.len(), MAX_MESSAGE_LEN);
            let second = message("35=0|", b'|');
            let log = [long.as_slice(), b"\nIN ", &second].concat();

            let records = read(&log);

            assert_eq!(records.len(), 2);
            assert_record(&records[0], b"", &long, &[]);
            assert_record(&records[1], b"IN ", &second, &[]);
        }

        #[test]
        fn a_body_length_past_the_cap_does_not_hold_records() {
            let first = with_trailer(&format!("8=FIX.4.4|9={MAX_MESSAGE_LEN}|35=0|"), b'|');
            let second = message("35=0|", b'|');
            let filler = "x".repeat(MAX_MESSAGE_LEN);
            let mut reader = LogReader::default();
            reader.feed(&[first.as_slice(), b"\n", &second, b"\n", filler.as_bytes()].concat());

            let records: Vec<LogRecord> = std::iter::from_fn(|| reader.next_record()).collect();

            assert_eq!(records.len(), 2);
            assert_record(
                &records[0],
                b"",
                &first,
                &[LogDiagnostic::BodyLengthMismatch],
            );
            assert_record(&records[1], b"", &second, &[]);
        }

        #[test]
        fn a_data_length_past_the_cap_does_not_hold_records() {
            let head = format!("8=FIX.4.4|9=5|35=0|95={MAX_MESSAGE_LEN}|96=a|");
            let first = with_trailer(&head, b'|');
            let second = message("35=0|", b'|');
            let filler = "x".repeat(MAX_MESSAGE_LEN);
            let mut reader = LogReader::default();
            reader.feed(&[first.as_slice(), b"\n", &second, b"\n", filler.as_bytes()].concat());

            let records: Vec<LogRecord> = std::iter::from_fn(|| reader.next_record()).collect();

            assert_eq!(records.len(), 2);
            assert_record(
                &records[0],
                b"",
                &first,
                &[LogDiagnostic::BodyLengthMismatch],
            );
            assert_record(&records[1], b"", &second, &[]);
        }
    }

    mod prefix_cap {
        use super::*;

        #[test]
        fn a_prefix_past_the_cap_keeps_the_bytes_nearest_its_message() {
            let kept = "x".repeat(MAX_PREFIX_LEN);
            let first = message("35=0|", b'|');
            let second = message("35=0|", b'|');
            let log = [
                b"dropped".as_slice(),
                kept.as_bytes(),
                &first,
                b"\nIN ",
                &second,
            ]
            .concat();

            let records = read(&log);

            assert_eq!(records.len(), 2);
            assert_record(
                &records[0],
                kept.as_bytes(),
                &first,
                &[LogDiagnostic::PrefixTruncated],
            );
            assert_record(&records[1], b"IN ", &second, &[]);
        }

        #[test]
        fn a_prefix_is_capped_before_its_message_arrives() {
            let lines = "no message here\n".repeat(MAX_PREFIX_LEN / 8);
            let message = message("35=0|", b'|');
            let mut reader = LogReader::default();

            reader.feed(lines.as_bytes());
            assert!(reader.next_record().is_none());
            assert_eq!(reader.buf.len(), MAX_PREFIX_LEN);
            reader.feed(&message);
            let records: Vec<LogRecord> = reader.finish().collect();

            let kept = &lines.as_bytes()[lines.len() - MAX_PREFIX_LEN..];
            assert_record(
                &records[0],
                kept,
                &message,
                &[LogDiagnostic::PrefixTruncated],
            );
        }
    }

    /// Feeding a log in two pieces, split at every byte, reads the same records
    /// as feeding it whole.
    #[test]
    fn splitting_the_log_anywhere_reads_the_same_records() {
        let log = [
            b"09:30:00 IN ".as_slice(),
            &message("35=A|34=1|", SOH),
            b"\r\n09:30:01 OUT ",
            &message("35=0|58=two\nlines|95=3|96=a|b|", b'|'),
            b"\nconnection reset 8=FIX\n",
            b"8=FIX.4.4|9=20|35=0|58=hel\n",
            &with_trailer("8=FIX.4.4|9=2|35=0|", b'|'),
            &message("35=0|", SOH),
        ]
        .concat();
        let summary = |records: Vec<LogRecord>| -> Vec<(Bytes, Bytes, Vec<LogDiagnostic>)> {
            records
                .into_iter()
                .map(|record| {
                    (
                        record.prefix,
                        record.message.bytes().clone(),
                        record.diagnostics,
                    )
                })
                .collect()
        };
        let whole = summary(read(&log));
        assert_eq!(whole.len(), 5);

        for split in 1..log.len() {
            let mut reader = LogReader::default();
            reader.feed(&log[..split]);
            let mut records: Vec<LogRecord> = std::iter::from_fn(|| reader.next_record()).collect();
            reader.feed(&log[split..]);
            records.extend(std::iter::from_fn(|| reader.next_record()));
            records.extend(reader.finish());

            assert_eq!(summary(records), whole, "split at {split}");
        }
    }
}
