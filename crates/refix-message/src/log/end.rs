use super::CHECKSUM;
use super::start::{NextStart, find_start};
use crate::Tokenizer;
use crate::framing::MAX_BODY_LENGTH_DIGITS;
use crate::tokenizer::{next_field, parse_u32};

/// Where a message ends in a log.
#[derive(Clone, Copy, Debug)]
pub(super) struct End {
    /// The message's length.
    pub(super) len: usize,
    /// The bytes its record takes, with any line break after the message.
    pub(super) consumed: usize,
    /// Whether it ends before its CheckSum field.
    pub(super) truncated: bool,
}

/// Where the message at the start of `bytes` ends, or `None` until more bytes arrive.
pub(super) fn find_end(
    tokenizer: &Tokenizer,
    bytes: &[u8],
    separator: u8,
    separator_override: Option<u8>,
    at_end: bool,
) -> Option<End> {
    match by_body_length(bytes, separator, at_end) {
        BodyLengthEnd::Found(end) => Some(end),
        BodyLengthEnd::NotFound => walk(tokenizer, bytes, separator, separator_override, at_end),
        BodyLengthEnd::Pending => None,
    }
}

enum BodyLengthEnd {
    Found(End),
    NotFound,
    Pending,
}

/// The end BodyLength gives, if it points at a CheckSum field.
fn by_body_length(bytes: &[u8], separator: u8, at_end: bool) -> BodyLengthEnd {
    let pending = || {
        if at_end {
            BodyLengthEnd::NotFound
        } else {
            BodyLengthEnd::Pending
        }
    };

    // The start check found BeginString's separator and a whole tag after it.
    let tag_start = bytes
        .iter()
        .position(|&byte| byte == separator)
        .map_or(bytes.len(), |at| at + 1);
    let Some(rest) = bytes[tag_start..].strip_prefix(b"9=") else {
        return BodyLengthEnd::NotFound;
    };
    let digits = rest
        .iter()
        .take(MAX_BODY_LENGTH_DIGITS + 1)
        .take_while(|byte| byte.is_ascii_digit())
        .count();
    if digits > MAX_BODY_LENGTH_DIGITS {
        return BodyLengthEnd::NotFound;
    }
    match rest.get(digits) {
        None => return pending(),
        Some(&byte) if byte == separator => {}
        Some(_) => return BodyLengthEnd::NotFound,
    }
    let Some(body_length) = parse_u32(&rest[..digits]) else {
        return BodyLengthEnd::NotFound;
    };

    let body_start = tag_start + 2 + digits + 1;
    let trailer_start = body_start + body_length as usize;
    if bytes.len() < trailer_start + 3 {
        return pending();
    }
    if bytes[trailer_start - 1] != separator || &bytes[trailer_start..trailer_start + 3] != b"10=" {
        return BodyLengthEnd::NotFound;
    }

    let value_start = trailer_start + 3;
    let value_end = bytes[value_start..]
        .iter()
        .position(|&byte| byte == separator || byte == b'\n')
        .map(|at| value_start + at);
    match value_end {
        Some(at) if bytes[at] == separator => match after_message(bytes, at + 1, at_end) {
            Some(end) => BodyLengthEnd::Found(end),
            None => BodyLengthEnd::Pending,
        },
        Some(at) => BodyLengthEnd::Found(at_line_break(bytes, at, false)),
        None if at_end => BodyLengthEnd::Found(End {
            len: bytes.len(),
            consumed: bytes.len(),
            truncated: false,
        }),
        None => BodyLengthEnd::Pending,
    }
}

/// Walks the fields of the message at the start of `bytes` to its CheckSum field.
fn walk(
    tokenizer: &Tokenizer,
    bytes: &[u8],
    separator: u8,
    separator_override: Option<u8>,
    at_end: bool,
) -> Option<End> {
    let mut pos = 0;
    let mut data_len = None;
    loop {
        let next = next_field(bytes, pos, data_len, separator);
        if let (Some((field, _)), Some(len)) = (next, data_len)
            && !at_end
            && field.value_start as usize + len >= bytes.len()
        {
            // The data value may run past the bytes so far.
            return None;
        }

        // A data value read by its length may hold anything.
        let checked_end = match next {
            Some((field, _))
                if data_len == Some((field.value_end - field.value_start) as usize) =>
            {
                field.value_start as usize
            }
            Some((field, _)) => field.value_end as usize,
            None => bytes.len(),
        };
        let from = pos.max(1);
        let line_break = bytes[from..checked_end]
            .iter()
            .position(|&byte| byte == b'\n')
            .map(|at| from + at);
        let search_end = line_break.unwrap_or(checked_end);
        match find_start(bytes, from..search_end, separator_override, at_end) {
            NextStart::Found(start) => {
                return Some(End {
                    len: start.at,
                    consumed: start.at,
                    truncated: true,
                });
            }
            NextStart::Pending(_) => return None,
            NextStart::NotFound => {}
        }
        if let Some(at) = line_break {
            let is_checksum = matches!(next, Some((field, _)) if field.tag == CHECKSUM);
            return Some(at_line_break(bytes, at, !is_checksum));
        }

        let Some((field, after)) = next else {
            // The rest holds no `=` or separator yet.
            return at_end.then_some(End {
                len: bytes.len(),
                consumed: bytes.len(),
                truncated: true,
            });
        };
        if field.value_end as usize == bytes.len() {
            // The value runs to the end of the bytes so far.
            return at_end.then_some(End {
                len: bytes.len(),
                consumed: bytes.len(),
                truncated: field.tag != CHECKSUM,
            });
        }
        if field.tag == CHECKSUM {
            return after_message(bytes, after, at_end);
        }
        data_len = tokenizer.pending_data_len(bytes, field);
        pos = after;
    }
}

/// The end of a message whose CheckSum field ends at `len`, taking one line break after it.
fn after_message(bytes: &[u8], len: usize, at_end: bool) -> Option<End> {
    let consumed = match &bytes[len..] {
        [] | [b'\r'] if !at_end => return None,
        [b'\n', ..] => len + 1,
        [b'\r', b'\n', ..] => len + 2,
        _ => len,
    };
    Some(End {
        len,
        consumed,
        truncated: false,
    })
}

/// The end of a message at the line break at `at`, which its record takes.
fn at_line_break(bytes: &[u8], at: usize, truncated: bool) -> End {
    let len = if bytes[at - 1] == b'\r' { at - 1 } else { at };
    End {
        len,
        consumed: at + 1,
        truncated,
    }
}
