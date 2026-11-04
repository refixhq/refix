use super::MAX_MESSAGE_LEN;
use super::end::{End, find_end};
use crate::Tokenizer;
use crate::framing::SOH;

/// The message at the start of `bytes`, logged with `^A` for SOH, as a copy
/// with SOH restored, or `None` until its line break arrives.
///
/// The returned end is in `bytes`. The message takes its whole line unless
/// it ends before the line does.
pub(super) fn copy_message(
    tokenizer: &Tokenizer,
    bytes: &[u8],
    at_end: bool,
) -> Option<(End, Vec<u8>)> {
    let capped = &bytes[..bytes.len().min(MAX_MESSAGE_LEN)];
    let (line, line_consumed) = match capped.iter().position(|&byte| byte == b'\n') {
        Some(at) => {
            let line = &capped[..at];
            (line.strip_suffix(b"\r").unwrap_or(line), at + 1)
        }
        None if at_end || capped.len() == MAX_MESSAGE_LEN => (capped, capped.len()),
        None => return None,
    };

    let mut copy = Vec::with_capacity(line.len());
    let mut rest = line;
    while let Some(at) = rest.windows(2).position(|pair| pair == b"^A") {
        copy.extend_from_slice(&rest[..at]);
        copy.push(SOH);
        rest = &rest[at + 2..];
    }
    copy.extend_from_slice(rest);

    let end = find_end(tokenizer, &copy, SOH, None, true)?;
    let len = original_offset(line, &copy, end.len);
    let consumed = if len == line.len() {
        line_consumed
    } else {
        len
    };
    copy.truncate(end.len);
    Some((
        End {
            len,
            consumed,
            truncated: end.truncated,
        },
        copy,
    ))
}

/// The offset in `line` of the byte at `offset` in its `copy`.
fn original_offset(line: &[u8], copy: &[u8], offset: usize) -> usize {
    let mut at = 0;
    for &byte in &copy[..offset] {
        at += if byte == SOH && line[at] == b'^' {
            2
        } else {
            1
        };
    }
    at
}
