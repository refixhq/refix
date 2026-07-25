#![no_main]

use libfuzzer_sys::fuzz_target;
use refix_message::scanner::{FrameScanner, ScanOutcome};

/// Drives the scanner over `stream` delivered in `chunk_size`-byte pieces,
/// returning the frames produced in order.
fn drive(stream: &[u8], chunk_size: usize) -> Vec<Vec<u8>> {
    let scanner = FrameScanner::default();
    let mut frames = Vec::new();
    let mut buf: Vec<u8> = Vec::new();
    for chunk in stream.chunks(chunk_size) {
        buf.extend_from_slice(chunk);
        loop {
            let advance = match scanner.scan(&buf) {
                ScanOutcome::Frame(f) => {
                    frames.push(f.bytes.to_vec());
                    f.bytes.len()
                }
                ScanOutcome::Garbled { skipped, .. } => skipped,
                ScanOutcome::Incomplete => break,
            };
            assert!(
                advance >= 1 && advance <= buf.len(),
                "bad advance {advance}"
            );
            buf.drain(..advance);
        }
    }
    frames
}

fuzz_target!(|data: &[u8]| {
    // First byte picks the chunk size; the rest is the stream.
    let Some((&first, stream)) = data.split_first() else {
        return;
    };
    let chunk_size = (first as usize).max(1);

    let chunked = drive(stream, chunk_size);
    let whole = drive(stream, stream.len().max(1));
    assert_eq!(
        chunked, whole,
        "chunk_size {chunk_size} changed the frame sequence"
    );
});
