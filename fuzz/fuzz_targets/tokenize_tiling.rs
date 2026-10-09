#![no_main]

use bytes::Bytes;
use libfuzzer_sys::fuzz_target;
use refix_message::{TokenizeError, Tokenizer};

fuzz_target!(|data: &[u8]| {
    match Tokenizer::default().tokenize(Bytes::copy_from_slice(data)) {
        Ok(message) => {
            let bytes = message.bytes();
            assert_eq!(bytes.as_ref(), data, "message must own exactly the input");

            // The index tiles the frame: fields appear in order without
            // overlap, every value ends on SOH, and the last SOH ends the
            // frame. Offsets are recovered from the entry slices, which
            // borrow from the frame buffer.
            let base = bytes.as_ptr() as usize;
            let mut next_start = 0;
            for (_, value) in message.entries() {
                let start = value.as_ptr() as usize - base;
                let end = start + value.len();
                assert!(
                    start >= next_start,
                    "field starts before the previous ended"
                );
                assert!(end < bytes.len(), "value runs off the frame");
                assert_eq!(bytes[end], 0x01, "value must end on SOH");
                next_start = end + 1;
            }
            assert_eq!(
                next_start,
                bytes.len(),
                "index must cover the frame exactly"
            );
        }
        Err(TokenizeError::TrailingBytes { frame_len }) => {
            assert!(frame_len >= 1 && frame_len < data.len(), "bad frame_len");
        }
        Err(_) => {}
    }
});
