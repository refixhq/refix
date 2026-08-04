use crate::framing::{GarbledReason, Outcome as FramingOutcome, Scanner};
use crate::{RawMessage, Tokenizer};
use bytes::{Bytes, BytesMut};

/// Reads a stream of FIX messages from incrementally fed bytes.
#[derive(Default)]
pub struct MessageStream {
    tokenizer: Tokenizer,
    scanner: Scanner,
    buf: BytesMut,
}

impl MessageStream {
    /// Creates a stream that tokenises with `tokenizer`.
    pub fn new(tokenizer: Tokenizer) -> Self {
        Self {
            tokenizer,
            scanner: Scanner::default(),
            buf: BytesMut::new(),
        }
    }

    /// Appends bytes to the stream's buffer.
    pub fn feed(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    /// Consumes and returns the next outcome at the front of the buffer.
    pub fn next_message(&mut self) -> Outcome {
        match self.scanner.scan(&self.buf) {
            FramingOutcome::Frame(frame) => {
                let len = frame.bytes.len();
                let bytes = self.buf.split_to(len).freeze();
                let fields = self.tokenizer.tokenize_fields(&bytes);
                Outcome::Message(RawMessage::new(bytes, fields))
            }
            FramingOutcome::Incomplete => Outcome::Incomplete,
            FramingOutcome::Garbled { reason, skipped } => Outcome::Garbled {
                reason,
                bytes: self.buf.split_to(skipped).freeze(),
            },
        }
    }
}

/// One step of reading the stream.
pub enum Outcome {
    /// A complete, checksum-verified message, consumed from the buffer.
    Message(RawMessage),
    /// Bytes that cannot begin a message, consumed from the buffer.
    Garbled { reason: GarbledReason, bytes: Bytes },
    /// The buffer holds no complete message, feed more bytes.
    Incomplete,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Tokenizer;
    use crate::test_utils::construct_valid_frame;

    /// Collects outcomes until the stream reports `Incomplete`.
    fn drain(stream: &mut MessageStream) -> Vec<Outcome> {
        let mut outcomes = Vec::new();
        loop {
            match stream.next_message() {
                Outcome::Incomplete => return outcomes,
                outcome => outcomes.push(outcome),
            }
        }
    }

    #[track_caller]
    fn expect_message(outcome: &Outcome) -> &RawMessage {
        match outcome {
            Outcome::Message(message) => message,
            Outcome::Garbled { .. } => panic!("expected a message, got a garble"),
            Outcome::Incomplete => panic!("expected a message, got Incomplete"),
        }
    }

    #[track_caller]
    fn expect_garble(outcome: &Outcome) -> (GarbledReason, &Bytes) {
        match outcome {
            Outcome::Garbled { reason, bytes } => (*reason, bytes),
            Outcome::Message(_) => panic!("expected a garble, got a message"),
            Outcome::Incomplete => panic!("expected a garble, got Incomplete"),
        }
    }

    #[test]
    fn empty_stream_is_incomplete() {
        let mut stream = MessageStream::default();
        assert!(matches!(stream.next_message(), Outcome::Incomplete));
    }

    #[test]
    fn single_message() {
        let frame = construct_valid_frame("FIX.4.4", "35=0|58=hi|");
        let mut stream = MessageStream::default();
        stream.feed(&frame);

        let outcomes = drain(&mut stream);

        assert_eq!(outcomes.len(), 1);
        assert_eq!(expect_message(&outcomes[0]).bytes(), &frame);
    }

    #[test]
    fn several_messages_in_one_feed() {
        let first = construct_valid_frame("FIX.4.4", "35=0|58=first|");
        let second = construct_valid_frame("FIX.4.4", "35=0|58=second|");
        let mut stream = MessageStream::default();
        stream.feed(&first);
        stream.feed(&second);

        let outcomes = drain(&mut stream);

        assert_eq!(outcomes.len(), 2);
        assert_eq!(expect_message(&outcomes[0]).bytes(), &first);
        assert_eq!(expect_message(&outcomes[1]).bytes(), &second);
    }

    /// Splitting a frame at every byte boundary covers every boundary class:
    /// mid-preamble, mid-tag, mid-body and mid-data-field. The data field's
    /// extent must survive the split.
    #[test]
    fn message_completes_across_feeds_at_every_split() {
        let frame = construct_valid_frame("FIX.4.4", "35=0|95=3|96=a|b|58=ok|");
        for split in 1..frame.len() {
            let mut stream = MessageStream::default();
            stream.feed(&frame[..split]);
            assert!(
                matches!(stream.next_message(), Outcome::Incomplete),
                "prefix of {split} bytes must be incomplete",
            );

            stream.feed(&frame[split..]);
            let outcomes = drain(&mut stream);
            assert_eq!(outcomes.len(), 1, "split at {split}");
            let message = expect_message(&outcomes[0]);
            assert_eq!(message.bytes(), &frame, "split at {split}");
            assert_eq!(message.get(96), Some(b"a\x01b".as_slice()));
        }
    }

    #[test]
    fn dialect_extras_flow_through() {
        let frame = construct_valid_frame("FIX.4.4", "35=0|5001=3|5002=a|b|");
        let mut stream = MessageStream::new(Tokenizer::with_extra_length_tags([5001]));
        stream.feed(&frame);

        let outcomes = drain(&mut stream);

        assert_eq!(
            expect_message(&outcomes[0]).get(5002),
            Some(b"a\x01b".as_slice())
        );
    }

    #[test]
    fn garbage_between_messages() {
        let first = construct_valid_frame("FIX.4.4", "35=0|58=first|");
        let second = construct_valid_frame("FIX.4.4", "35=0|58=second|");
        let mut stream = MessageStream::default();
        stream.feed(&first);
        stream.feed(b"YY8=Z");
        stream.feed(&second);

        let outcomes = drain(&mut stream);

        assert_eq!(outcomes.len(), 3);
        assert_eq!(expect_message(&outcomes[0]).bytes(), &first);
        let (reason, bytes) = expect_garble(&outcomes[1]);
        assert_eq!(reason, GarbledReason::MissingBeginString);
        assert_eq!(bytes.as_ref(), b"YY8=Z");
        assert_eq!(expect_message(&outcomes[2]).bytes(), &second);
    }

    #[test]
    fn leading_garbage_is_skipped() {
        let frame = construct_valid_frame("FIX.4.4", "35=0|");
        let mut stream = MessageStream::default();
        stream.feed(b"junk");
        stream.feed(&frame);

        let outcomes = drain(&mut stream);

        assert_eq!(outcomes.len(), 2);
        let (reason, bytes) = expect_garble(&outcomes[0]);
        assert_eq!(reason, GarbledReason::MissingBeginString);
        assert_eq!(bytes.as_ref(), b"junk");
        assert_eq!(expect_message(&outcomes[1]).bytes(), &frame);
    }

    #[test]
    fn garbled_frame_does_not_stall_the_stream() {
        let mut bad = construct_valid_frame("FIX.4.4", "35=0|58=bad|");
        let last_digit = bad.len() - 2;
        bad[last_digit] = if bad[last_digit] == b'0' { b'1' } else { b'0' };
        let good = construct_valid_frame("FIX.4.4", "35=0|58=good|");
        let mut stream = MessageStream::default();
        stream.feed(&bad);
        stream.feed(&good);

        let outcomes = drain(&mut stream);

        let (reason, _) = expect_garble(&outcomes[0]);
        assert_eq!(reason, GarbledReason::ChecksumMismatch);
        let last = outcomes.last().unwrap();
        assert_eq!(expect_message(last).bytes(), &good);
    }

    /// An oversized frame is a garble like any other, not a terminal state.
    #[test]
    fn oversized_frame_is_skipped_not_fatal() {
        let frame = construct_valid_frame("FIX.4.4", "35=0|");
        let mut stream = MessageStream::default();
        stream.feed(b"8=FIX.4.4\x019=1048577\x01");
        stream.feed(&frame);

        let outcomes = drain(&mut stream);

        let (reason, _) = expect_garble(&outcomes[0]);
        assert_eq!(reason, GarbledReason::FrameTooLarge);
        let last = outcomes.last().unwrap();
        assert_eq!(expect_message(last).bytes(), &frame);
    }
}
