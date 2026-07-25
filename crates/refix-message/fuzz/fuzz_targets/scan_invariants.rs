#![no_main]

use libfuzzer_sys::fuzz_target;
use refix_message::scanner::{FrameScanner, ScanOutcome};

fuzz_target!(|data: &[u8]| {
    let scanner = FrameScanner::default();
    match scanner.scan(data) {
        ScanOutcome::Frame(frame) => {
            // The frame is a prefix of the input and structurally self-describing.
            assert!(data.starts_with(frame.bytes));
            assert!(frame.bytes.starts_with(b"8=FIX"));
            assert_eq!(frame.bytes.last(), Some(&0x01), "frame must end on SOH");
            assert!(frame.body_length < frame.bytes.len());

            // A frame is self-delimiting: scanning its own bytes yields it again.
            match scanner.scan(frame.bytes) {
                ScanOutcome::Frame(again) => assert_eq!(again.bytes, frame.bytes),
                other => panic!("frame did not rescan as itself: {other:?}"),
            }
        }
        ScanOutcome::Garbled { skipped, .. } => {
            // The advance contract.
            assert!(skipped >= 1, "skipped must make progress");
            assert!(skipped <= data.len(), "skipped past the end of the buffer");
        }
        ScanOutcome::Incomplete => {}
    }
});
