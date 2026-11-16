use crate::framing::GarbledReason;
use std::fmt;

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

impl fmt::Display for TokenizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenizeError::Garbled(reason) => write!(f, "garbled frame: {reason}"),
            TokenizeError::Incomplete => f.write_str("incomplete message"),
            TokenizeError::TrailingBytes { frame_len } => {
                write!(f, "one message of {frame_len} bytes, then trailing input")
            }
            TokenizeError::TooLargeToIndex => f.write_str("frame too large to index"),
        }
    }
}

impl std::error::Error for TokenizeError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn displays_each_error() {
        let cases = [
            (
                TokenizeError::Garbled(GarbledReason::ChecksumMismatch),
                "garbled frame: CheckSum mismatch",
            ),
            (TokenizeError::Incomplete, "incomplete message"),
            (
                TokenizeError::TrailingBytes { frame_len: 82 },
                "one message of 82 bytes, then trailing input",
            ),
            (TokenizeError::TooLargeToIndex, "frame too large to index"),
        ];
        for (error, expected) in cases {
            assert_eq!(error.to_string(), expected);
        }
    }

    #[test]
    fn converts_to_a_boxed_error() {
        fn fail() -> Result<(), Box<dyn std::error::Error>> {
            Err(TokenizeError::Incomplete)?
        }

        assert_eq!(fail().unwrap_err().to_string(), "incomplete message");
    }
}
