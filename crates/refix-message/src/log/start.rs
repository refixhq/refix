use crate::framing::{BEGIN_STRING_PREFIX, MAX_BEGIN_STRING_LEN};
use std::ops::Range;

/// Longest tag we'll read after a BeginString, in digits.
const MAX_TAG_DIGITS: usize = 10;

/// Where a message starts in a log.
#[derive(Clone, Copy, Debug)]
pub(super) struct Start {
    /// Offset of its `8=`.
    pub(super) at: usize,
    /// What ends each of its fields.
    pub(super) separator: Separator,
}

/// How a logged message's fields end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Separator {
    Byte(u8),
    /// The two characters `cat -v` and `less` show SOH as.
    CaretA,
}

pub(super) enum NextStart {
    Found(Start),
    /// The candidate at this offset needs more bytes to decide.
    Pending(usize),
    NotFound,
}

/// The first message start at an offset in `range` of `bytes`.
///
/// A start is an `8=FIX` that doesn't follow a digit, then the rest of a
/// BeginString value, a separator, and a tag with its `=`. The separator is
/// detected unless `separator` is given, and only detection finds `^A`.
pub(super) fn find_start(
    bytes: &[u8],
    range: Range<usize>,
    separator: Option<u8>,
    at_end: bool,
) -> NextStart {
    for at in range {
        if bytes[at] != b'8' || at > 0 && bytes[at - 1].is_ascii_digit() {
            continue;
        }
        match check(&bytes[at..], separator) {
            Check::Start(separator) => return NextStart::Found(Start { at, separator }),
            Check::Pending if !at_end => return NextStart::Pending(at),
            Check::Pending | Check::NotStart => {}
        }
    }
    NextStart::NotFound
}

enum Check {
    Start(Separator),
    NotStart,
    Pending,
}

/// Whether `bytes` opens with a message start, and its separator if so.
fn check(bytes: &[u8], separator: Option<u8>) -> Check {
    let shared = bytes.len().min(BEGIN_STRING_PREFIX.len());
    if bytes[..shared] != BEGIN_STRING_PREFIX[..shared] {
        return Check::NotStart;
    }
    if shared < BEGIN_STRING_PREFIX.len() {
        return Check::Pending;
    }

    let value_end = BEGIN_STRING_PREFIX.len()
        + bytes[BEGIN_STRING_PREFIX.len()..]
            .iter()
            .take(MAX_BEGIN_STRING_LEN)
            .take_while(|&&byte| byte.is_ascii_alphanumeric() || byte == b'.')
            .count();
    if value_end > MAX_BEGIN_STRING_LEN {
        return Check::NotStart;
    }
    let Some(&found) = bytes.get(value_end) else {
        return Check::Pending;
    };
    let separator = match separator {
        Some(separator) if found != separator => return Check::NotStart,
        Some(separator) => Separator::Byte(separator),
        None if found == b'\n' || found == b'\r' => return Check::NotStart,
        None if found == b'^' => match bytes.get(value_end + 1) {
            None => return Check::Pending,
            Some(b'A') => Separator::CaretA,
            Some(_) => Separator::Byte(b'^'),
        },
        None => Separator::Byte(found),
    };
    let separator_len = match separator {
        Separator::Byte(_) => 1,
        Separator::CaretA => 2,
    };

    let tag = &bytes[value_end + separator_len..];
    let digits = tag
        .iter()
        .take(MAX_TAG_DIGITS + 1)
        .take_while(|byte| byte.is_ascii_digit())
        .count();
    if digits > MAX_TAG_DIGITS {
        return Check::NotStart;
    }
    match tag.get(digits) {
        None => Check::Pending,
        Some(b'=') if digits > 0 => Check::Start(separator),
        Some(_) => Check::NotStart,
    }
}
