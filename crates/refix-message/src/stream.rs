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
