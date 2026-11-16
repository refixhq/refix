use crate::framing::GarbledReason;

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
