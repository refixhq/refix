//! Framing: delimiting one checksum-verified FIX message from a byte stream.
//!
//! A FIX connection is an unbroken stream with no external length prefix, so the
//! only way to know where a message starts is to know where the previous one
//! ended. [`Scanner::scan`] answers that one message at a time, borrowing
//! from the caller's buffer and allocating nothing.
//!
//! Each call returns one of three outcomes, and each implies a different action:
//! a [`Frame`] to consume, [`Outcome::Incomplete`] to wait on, or
//! [`Outcome::Garbled`] to skip. Driving it is a loop:
//!
//! ```
//! use refix_message::framing::{Scanner, Outcome};
//!
//! // Two heartbeats back to back, SOH-delimited.
//! let stream = b"8=FIX.4.4\x019=10\x0135=0\x0134=2\x0110=166\x01\
//!                8=FIX.4.4\x019=10\x0135=0\x0134=2\x0110=166\x01";
//!
//! let scanner = Scanner::default();
//! let mut buf = &stream[..];
//! let mut seen = Vec::new();
//! loop {
//!     let advance = match scanner.scan(buf) {
//!         Outcome::Frame(frame) => {
//!             seen.push(frame.msg_type.to_vec());
//!             frame.bytes.len()
//!         }
//!         Outcome::Garbled { skipped, .. } => skipped,
//!         Outcome::Incomplete => break, // read more from the socket
//!     };
//!     buf = &buf[advance..];
//! }
//! assert_eq!(seen, vec![b"0".to_vec(), b"0".to_vec()]);
//! ```
//!
//! # Scope
//!
//! Framing reads only the fields whose position the frame structure fixes:
//! `BeginString`, `BodyLength`, `MsgType` and `CheckSum`, each checked
//! positionally at its known offset. Every other header field has no guaranteed
//! wire position and may sit behind a length-prefixed data field, so reading it
//! needs the order-independent, data-field-aware decoding of the layer above.

pub const SOH: u8 = 0x01;

/// Longest BeginString field we'll frame: `8=` through the byte before its SOH.
const MAX_BEGIN_STRING_LEN: usize = 16;

/// Longest BodyLength value we'll frame, in digits.
const MAX_BODY_LENGTH_DIGITS: usize = 10;

/// `10=` + three checksum digits + SOH.
const CHECKSUM_FIELD_LEN: usize = 7;

/// The shortest prefix common to every BeginString: the FIX spec fixes every
/// one to the form `FIX.x.y` or `FIXT.x.y`, so `8=FIX` marks the start of any frame.
const BEGIN_STRING_PREFIX: &[u8] = b"8=FIX";

/// A stateless framing scanner: each [`scan`](Scanner::scan) call attempts
/// to delimit one checksum-verified frame from the start of a buffer.
///
/// The [`Default`] scanner caps frames at 1 MiB; use [`Scanner::new`] to
/// choose a different cap.
pub struct Scanner {
    max_frame_size: usize,
}

impl Scanner {
    /// Creates a scanner that reports frames longer than `max_frame_size`
    /// bytes as [`GarbledReason::FrameTooLarge`].
    pub fn new(max_frame_size: usize) -> Self {
        Self { max_frame_size }
    }

    /// Scans one frame from the start of `buf`.
    ///
    /// `buf` must begin at a presumed message boundary: the start of the
    /// stream, or the position reached by advancing past a previous outcome.
    /// After [`Outcome::Frame`], advance by `frame.bytes.len()`; after
    /// [`Outcome::Garbled`], advance by `skipped`; on
    /// [`Outcome::Incomplete`], read more bytes and scan again from the
    /// same position.
    pub fn scan<'a>(&self, buf: &'a [u8]) -> Outcome<'a> {
        match self.try_scan(buf) {
            Ok(frame) => Outcome::Frame(frame),
            Err(halt) => halt.into(),
        }
    }

    fn try_scan<'a>(&self, buf: &'a [u8]) -> Result<Frame<'a>, Halt> {
        if !buf.starts_with(BEGIN_STRING_PREFIX) {
            // If `buf` is still a prefix of `8=FIX`, more bytes
            // could complete it; otherwise it's garbage to resync past.
            return if BEGIN_STRING_PREFIX.starts_with(buf) {
                Err(Halt::Incomplete)
            } else {
                Err(structural_garble(GarbledReason::MissingBeginString, buf))
            };
        }

        let (begin_string, body_length_start) = parse_begin_string(buf)?;
        let (body_length, body_start) = parse_body_length(buf, body_length_start)?;

        let frame_len = validated_frame_len(buf, body_start, body_length, self.max_frame_size)?;
        let frame = &buf[..frame_len];
        let checksum_start = body_start + body_length;

        verify_trailer_start(buf, checksum_start)?;
        verify_checksum(frame, checksum_start)?;

        let msg_type = parse_msg_type(frame, body_start, body_length)?;
        Ok(Frame {
            bytes: frame,
            begin_string,
            body_length,
            msg_type,
        })
    }
}

/// Bytes to skip to resynchronise onto the next frame start.
///
/// Returns the offset of the next `8=FIX` after offset 0. If none is present,
/// preserves the longest partial `8=FIX` prefix hanging off the tail.
/// Failing that, skips the whole buffer.
fn find_next_begin_string(buf: &[u8]) -> usize {
    debug_assert!(
        !BEGIN_STRING_PREFIX.starts_with(buf),
        "a partial `8=FIX` has no resync distance; it is Incomplete",
    );

    if let Some(skip) = buf[1..]
        .windows(BEGIN_STRING_PREFIX.len())
        .position(|b| b == BEGIN_STRING_PREFIX)
    {
        return skip + 1;
    }

    let max_k = (BEGIN_STRING_PREFIX.len() - 1).min(buf.len());
    for k in (1..=max_k).rev() {
        if buf[buf.len() - k..] == BEGIN_STRING_PREFIX[..k] {
            return buf.len() - k;
        }
    }

    buf.len()
}

/// Builds a structural garble - one whose extent isn't trustworthy, so the skip
/// resyncs by searching for the next BeginString. For garbles found once the
/// frame is checksum-verified, use [`post_structural_garble`] instead.
fn structural_garble(reason: GarbledReason, buf: &[u8]) -> Halt {
    Halt::Garbled {
        reason,
        skipped: find_next_begin_string(buf),
    }
}

/// Builds a post-structural garble - one found after the frame's extent is
/// trusted (checksum-verified), so the skip is the whole frame, not a resync.
fn post_structural_garble(reason: GarbledReason, frame: &[u8]) -> Halt {
    Halt::Garbled {
        reason,
        skipped: frame.len(),
    }
}

/// Parses `BeginString(8)`, the first field of a message.
fn parse_begin_string(buf: &'_ [u8]) -> Result<(BeginString<'_>, usize), Halt> {
    let search_end = buf.len().min(MAX_BEGIN_STRING_LEN + 1);
    let soh = match buf[2..search_end].iter().position(|&b| b == SOH) {
        Some(rel) => rel + 2,
        None if buf.len() <= MAX_BEGIN_STRING_LEN => return Err(Halt::Incomplete),
        None => return Err(structural_garble(GarbledReason::MalformedBeginString, buf)),
    };

    let value = &buf[2..soh];
    let begin_string = match value {
        b"FIX.4.0" => BeginString::Fix40,
        b"FIX.4.1" => BeginString::Fix41,
        b"FIX.4.2" => BeginString::Fix42,
        b"FIX.4.3" => BeginString::Fix43,
        b"FIX.4.4" => BeginString::Fix44,
        b"FIXT.1.1" => BeginString::Fixt11,
        other => BeginString::Other(other),
    };

    Ok((begin_string, soh + 1)) // soh + 1 = start of the 9= field
}

/// Parses `BodyLength(9)`, the second field of a message (starting at `body_length_start`).
///
/// On success returns `(body_length, body_start)`, where `body_start` is the
/// offset just past the BodyLength SOH.
///
/// Throughout this module, `body_start` and `body_length` describe the region
/// BodyLength measures: from its own terminating SOH up to and including the SOH
/// before `10=`. That region holds most of the StandardHeader, so it is not the
/// FIX "body" of application fields.
fn parse_body_length(buf: &[u8], body_length_start: usize) -> Result<(usize, usize), Halt> {
    // Expect the `9=` tag. With fewer than 2 bytes we can't even test it yet.
    let tag = &buf[body_length_start..];
    if tag.len() < 2 {
        return Err(Halt::Incomplete);
    }
    if &tag[..2] != b"9=" {
        return Err(structural_garble(GarbledReason::MissingBodyLength, buf));
    }

    // Digits run from just after `9=` to the terminating SOH, bounded by the cap.
    let digits_start = body_length_start + 2;
    let search_end = buf.len().min(digits_start + MAX_BODY_LENGTH_DIGITS + 1);
    let soh_rel = match buf[digits_start..search_end].iter().position(|&b| b == SOH) {
        Some(rel) => rel,
        // Out-of-buffer first, cap-exceeded second - same ordering as BeginString.
        None if buf.len() <= digits_start + MAX_BODY_LENGTH_DIGITS => return Err(Halt::Incomplete),
        None => return Err(structural_garble(GarbledReason::MalformedBodyLength, buf)),
    };

    let digits = &buf[digits_start..digits_start + soh_rel];
    if digits.is_empty() || !digits.iter().all(|b| b.is_ascii_digit()) {
        return Err(structural_garble(GarbledReason::MalformedBodyLength, buf));
    }

    let body_length = digits
        .iter()
        .try_fold(0usize, |acc, &b| {
            acc.checked_mul(10)?.checked_add(usize::from(b - b'0'))
        })
        .ok_or_else(|| structural_garble(GarbledReason::MalformedBodyLength, buf))?;

    let body_start = digits_start + soh_rel + 1; // just past the BodyLength SOH
    Ok((body_length, body_start))
}

/// The total frame length implied by BodyLength, bounded and fully buffered.
///
/// `FrameTooLarge` if it exceeds `max_frame_size`; `Incomplete` if the frame
/// hasn't fully arrived. The frame's *content* - the `10=` trailer and
/// checksum - is not inspected here.
fn validated_frame_len(
    buf: &[u8],
    body_start: usize,
    body_length: usize,
    max_frame_size: usize,
) -> Result<usize, Halt> {
    let frame_len = body_start
        .saturating_add(body_length)
        .saturating_add(CHECKSUM_FIELD_LEN);
    if frame_len > max_frame_size {
        return Err(structural_garble(GarbledReason::FrameTooLarge, buf));
    }
    if buf.len() < frame_len {
        return Err(Halt::Incomplete);
    }
    Ok(frame_len)
}

/// Confirms BodyLength's jump landed on the CheckSum field: `10=` at a field
/// boundary (SOH just before). A miss means BodyLength was wrong, so resync over
/// the whole buffer.
fn verify_trailer_start(buf: &[u8], checksum_start: usize) -> Result<(), Halt> {
    if buf[checksum_start - 1] != SOH || &buf[checksum_start..checksum_start + 3] != b"10=" {
        return Err(structural_garble(GarbledReason::BodyLengthMismatch, buf));
    }
    Ok(())
}

/// Verifies the `<CheckSum><SOH>` at `checksum_start`, the `10=` landing
/// having already been confirmed.
fn verify_checksum(frame: &[u8], checksum_start: usize) -> Result<(), Halt> {
    let digits = &frame[checksum_start + 3..checksum_start + 6];
    let trailing_soh = frame[checksum_start + 6];
    if trailing_soh != SOH || !digits.iter().all(|b| b.is_ascii_digit()) {
        return Err(post_structural_garble(
            GarbledReason::MalformedChecksum,
            frame,
        ));
    }

    let stated = u16::from(digits[0] - b'0') * 100
        + u16::from(digits[1] - b'0') * 10
        + u16::from(digits[2] - b'0');
    let computed = frame[..checksum_start]
        .iter()
        .fold(0u8, |acc, &b| acc.wrapping_add(b));

    if u16::from(computed) != stated {
        return Err(post_structural_garble(
            GarbledReason::ChecksumMismatch,
            frame,
        ));
    }

    Ok(())
}

/// Parses `MsgType(35)`, the third field of a message.
fn parse_msg_type(frame: &[u8], body_start: usize, body_length: usize) -> Result<&[u8], Halt> {
    let region = &frame[body_start..body_start + body_length];
    let Some(after_tag) = region.strip_prefix(b"35=") else {
        return Err(post_structural_garble(GarbledReason::MissingMsgType, frame));
    };
    // The region's final byte is the SOH before `10=`,
    // so a terminator is always present once the tag matched.
    let Some(soh) = after_tag.iter().position(|&b| b == SOH) else {
        return Err(post_structural_garble(GarbledReason::MissingMsgType, frame));
    };
    Ok(&after_tag[..soh])
}

impl Default for Scanner {
    fn default() -> Self {
        Self {
            max_frame_size: 1 << 20,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome<'a> {
    /// A well-framed message, checksum verified.
    Frame(Frame<'a>),
    /// The bytes don't represent a complete message.
    Incomplete,
    /// The message is garbled.
    Garbled {
        /// The reason for the supplied bytes being garbled.
        reason: GarbledReason,
        /// Number of bytes to skip.
        ///
        /// Always at least 1: the caller advances by this many bytes, so a
        /// garbled frame never stalls the stream.
        skipped: usize,
    },
}

/// One well-framed, checksum-verified message.
///
/// Carries the raw frame bytes plus the three preamble fields whose position is
/// guaranteed by the frame structure itself (`8`/`9`/`35`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame<'a> {
    /// The full frame: 8= through the SOH after CheckSum.
    pub bytes: &'a [u8],
    /// `BeginString(8)`
    pub begin_string: BeginString<'a>,
    /// `BodyLength(9)`
    pub body_length: usize,
    /// `MsgType(35)`
    pub msg_type: &'a [u8],
}

/// A scan that ended without a frame.
enum Halt {
    Incomplete,
    Garbled {
        reason: GarbledReason,
        skipped: usize,
    },
}

impl<'a> From<Halt> for Outcome<'a> {
    fn from(halt: Halt) -> Self {
        match halt {
            Halt::Incomplete => Outcome::Incomplete,
            Halt::Garbled { reason, skipped } => Outcome::Garbled { reason, skipped },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GarbledReason {
    /// It does not start with `8=FIX` (so it is not a frame start).
    MissingBeginString,
    /// `8=FIX` present, but no SOH within MAX_BEGIN_STRING_LEN.
    MalformedBeginString,
    /// The second tag is not `9=`.
    MissingBodyLength,
    /// BodyLength is empty, contains a non-digit, exceeds the digit cap, or is
    /// too large to represent.
    MalformedBodyLength,
    /// The BodyLength-implied offset is not a `10=` trailer at a field boundary
    /// (i.e. not preceded by SOH, or not `10=`).
    BodyLengthMismatch,
    /// The third field is not `35=` (it must directly follow BodyLength).
    MissingMsgType,
    /// The CheckSum field is not three digits followed by SOH.
    MalformedChecksum,
    /// The calculated checksum does not match the value of CheckSum.
    ChecksumMismatch,
    /// The BodyLength-implied frame length exceeds max_frame_size.
    FrameTooLarge,
}

/// The value of `BeginString(8)`, with recognised protocol versions decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeginString<'a> {
    Fix40,
    Fix41,
    Fix42,
    Fix43,
    Fix44,
    Fixt11,
    /// An unrecognised BeginString, preserved as raw bytes.
    Other(&'a [u8]),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{construct_valid_frame, to_wire};

    fn scan(buf: &[u8]) -> Outcome<'_> {
        Scanner::default().scan(buf)
    }

    #[track_caller]
    fn assert_garbled(out: Outcome, reason: GarbledReason, skipped: usize) {
        match out {
            Outcome::Garbled {
                reason: r,
                skipped: s,
            } => {
                assert_eq!(r, reason, "reason");
                assert_eq!(s, skipped, "skipped");
            }
            _ => panic!("expected Garbled({reason:?}), got a different outcome"),
        }
    }

    #[track_caller]
    fn assert_incomplete(out: Outcome) {
        assert!(matches!(out, Outcome::Incomplete), "expected Incomplete");
    }

    /// Scans a buffer expected to be a well-formed frame and returns it.
    #[track_caller]
    fn frame(buf: &[u8]) -> Frame<'_> {
        match scan(buf) {
            Outcome::Frame(frame) => frame,
            _ => panic!("expected Frame"),
        }
    }

    /// The resync primitive on its own: pure, fiddly, and stable enough to test directly.
    mod resync {
        use super::*;

        #[test]
        fn finds_needle_after_offset_zero() {
            assert_eq!(find_next_begin_string(b"XX8=FIX"), 2);
        }

        #[test]
        fn finds_needle_at_offset_one() {
            assert_eq!(find_next_begin_string(b"X8=FIX"), 1);
        }

        #[test]
        fn no_match_skips_whole_buffer() {
            assert_eq!(find_next_begin_string(b"XYZW"), 4);
        }

        #[test]
        fn preserves_longest_partial_tail() {
            // Ends in `8=FI`, a 4-byte partial the next read may complete.
            assert_eq!(find_next_begin_string(b"garbage8=FI"), 7);
        }

        #[test]
        fn preserves_short_partial_tail() {
            assert_eq!(find_next_begin_string(b"xx8="), 2);
        }

        #[test]
        fn skip_is_always_in_range() {
            // Every in-domain buffer (i.e. not itself a partial `8=FIX`, which is
            // Incomplete) must yield a skip that both progresses and stays inside
            // the buffer. Short ones matter most: that is where a tail match could
            // otherwise cover the whole buffer and report 0.
            for buf in [
                &b"x"[..],
                b"9",
                b"=8",
                b"junk",
                b"8=FIXX",
                b"xx8=",
                b"x8=FI",
                b"garbage8=FI",
                b"XX8=FIX",
                b"8=NOTFIX",
            ] {
                let skipped = find_next_begin_string(buf);
                assert!(
                    (1..=buf.len()).contains(&skipped),
                    "skip of {skipped} out of range 1..={} for {:?}",
                    buf.len(),
                    String::from_utf8_lossy(buf),
                );
            }
        }
    }

    mod incomplete {
        use super::*;

        #[test]
        fn empty() {
            assert_incomplete(scan(b""));
        }

        #[test]
        fn lone_8() {
            assert_incomplete(scan(b"8"));
        }

        #[test]
        fn begin_string_without_soh() {
            assert_incomplete(scan(&to_wire("8=FIX.4.4")));
        }

        #[test]
        fn body_not_yet_arrived() {
            // BodyLength claims 100 bytes; only a handful are here.
            assert_incomplete(scan(&to_wire("8=FIX.4.4|9=100|35=A")));
        }
    }

    /// Extent untrustworthy: the skip resyncs by searching for the next BeginString.
    mod structural {
        use super::*;

        #[test]
        fn missing_begin_string() {
            assert_garbled(scan(&to_wire("junk")), GarbledReason::MissingBeginString, 4);
        }

        #[test]
        fn non_fix_prefix_is_missing_begin_string() {
            // `8=` but not `8=FIX`: not a frame start at all, so resync past it.
            assert_garbled(
                scan(&to_wire("8=NOTFIX|9=5|")),
                GarbledReason::MissingBeginString,
                13,
            );
        }

        #[test]
        fn malformed_begin_string_overruns_bound() {
            // Starts with `8=FIX` but the BeginString runs past the bound with no SOH.
            let mut buf = b"8=FIX".to_vec();
            buf.extend([b'X'; 18]); // 23 bytes, no SOH within MAX_BEGIN_STRING_LEN
            assert_garbled(scan(&buf), GarbledReason::MalformedBeginString, 23);
        }

        #[test]
        fn missing_body_length() {
            assert_garbled(
                scan(&to_wire("8=FIX.4.4|35=A|")),
                GarbledReason::MissingBodyLength,
                15,
            );
        }

        #[test]
        fn malformed_body_length_non_digit() {
            assert_garbled(
                scan(&to_wire("8=FIX.4.4|9=XY|")),
                GarbledReason::MalformedBodyLength,
                15,
            );
        }

        #[test]
        fn malformed_body_length_overruns_cap() {
            // 15 digits, no SOH: exceeds MAX_BODY_LENGTH_DIGITS.
            assert_garbled(
                scan(&to_wire("8=FIX.4.4|9=123456789012345")),
                GarbledReason::MalformedBodyLength,
                27,
            );
        }

        #[test]
        fn max_digit_body_length_does_not_panic() {
            // The largest cap-legal BodyLength. It fits usize on a 64-bit target
            // (rejected as FrameTooLarge) but overruns it on a 32-bit one (rejected
            // as malformed); either way it must garble, never panic or wrap.
            let buf = to_wire("8=FIX.4.4|9=9999999999|35=A|");
            assert!(matches!(scan(&buf), Outcome::Garbled { .. }));
        }

        #[test]
        fn frame_too_large_rejected_before_waiting() {
            // frame_len of ~123 exceeds the cap of 50, so it rejects rather than waits.
            let buf = to_wire("8=FIX.4.4|9=100|");
            assert_garbled(
                Scanner::new(50).scan(&buf),
                GarbledReason::FrameTooLarge,
                16,
            );
        }

        #[test]
        fn body_length_mismatch() {
            // BodyLength says 3, real body is longer, so the jump lands mid-field.
            assert_garbled(
                scan(&to_wire("8=FIX.4.4|9=3|35=A|10=000|")),
                GarbledReason::BodyLengthMismatch,
                26,
            );
        }

        #[test]
        fn resync_lands_on_next_message() {
            // Garbage, then a real message start 6 bytes in: the skip must land
            // exactly on the next BeginString, not the end of the buffer.
            let mut buf = to_wire("junk|X");
            buf.extend_from_slice(&construct_valid_frame("FIX.4.4", "35=A|"));
            assert_garbled(scan(&buf), GarbledReason::MissingBeginString, 6);
        }
    }

    /// Extent trusted: the skip is the whole frame, never a resync.
    mod post_structural {
        use super::*;

        #[test]
        fn checksum_mismatch() {
            let mut buf = construct_valid_frame("FIX.4.4", "35=A|34=1|");
            let len = buf.len();
            let last_digit = &mut buf[len - 2]; // 3 digits then trailing SOH
            *last_digit = if *last_digit == b'0' { b'1' } else { b'0' };
            assert_garbled(scan(&buf), GarbledReason::ChecksumMismatch, len);
        }

        #[test]
        fn malformed_checksum_non_digit() {
            let mut buf = construct_valid_frame("FIX.4.4", "35=A|34=1|");
            let len = buf.len();
            buf[len - 2] = b'x'; // non-digit in the checksum value
            assert_garbled(scan(&buf), GarbledReason::MalformedChecksum, len);
        }

        #[test]
        fn missing_msg_type() {
            let buf = construct_valid_frame("FIX.4.4", "34=1|");
            let len = buf.len();
            assert_garbled(scan(&buf), GarbledReason::MissingMsgType, len);
        }

        #[test]
        fn missing_msg_type_on_empty_body() {
            let buf = construct_valid_frame("FIX.4.4", "");
            let len = buf.len();
            assert_garbled(scan(&buf), GarbledReason::MissingMsgType, len);
        }

        #[test]
        fn empty_first_field() {
            // An empty field where MsgType belongs, so `35=` is only the fourth
            // field: `8=FIX.4.4|9=6||35=A|10=182|`. BodyLength and CheckSum are
            // both correct, so framing reaches the MsgType check, and requiring
            // `35=` to sit exactly at the start of the region is what rejects it.
            let buf = construct_valid_frame("FIX.4.4", "|35=A|");
            let len = buf.len();
            assert_garbled(scan(&buf), GarbledReason::MissingMsgType, len);
        }

        #[test]
        fn skip_does_not_resync_into_body() {
            // The body contains `8=FIX` inside a Text field. A checksum failure
            // trusts the frame extent, so it must skip the whole frame rather
            // than resync onto the embedded bytes.
            let mut buf = construct_valid_frame("FIX.4.4", "35=A|58=8=FIX.4.4|");
            let len = buf.len();
            buf[len - 2] = b'x';
            assert_garbled(scan(&buf), GarbledReason::MalformedChecksum, len);
        }
    }

    mod happy_path {
        use super::*;

        #[test]
        fn valid_frame_scans() {
            let buf = construct_valid_frame("FIX.4.4", "35=A|34=1|49=ME|56=YOU|");
            assert!(matches!(scan(&buf), Outcome::Frame(_)));
        }
    }

    mod preamble {
        use super::*;

        #[test]
        fn msg_type() {
            let buf = construct_valid_frame("FIX.4.4", "35=D|");
            assert_eq!(frame(&buf).msg_type, b"D".as_slice());
        }

        #[test]
        fn begin_string_and_body_length() {
            let buf = construct_valid_frame("FIX.4.4", "35=A|");
            let h = frame(&buf);
            assert_eq!(h.begin_string, BeginString::Fix44);
            assert_eq!(h.body_length, 5); // "35=A" + SOH
        }

        #[test]
        fn unknown_begin_string_is_other() {
            let buf = construct_valid_frame("FIX.5.0", "35=A|");
            assert_eq!(frame(&buf).begin_string, BeginString::Other(b"FIX.5.0"));
        }

        #[test]
        fn fixt_begin_string() {
            let buf = construct_valid_frame("FIXT.1.1", "35=A|");
            assert_eq!(frame(&buf).begin_string, BeginString::Fixt11);
        }

        #[test]
        fn msg_type_read_regardless_of_trailing_fields() {
            // Only MsgType is surfaced; the fields after it are the tokenizer's
            // concern and must not affect framing or the preamble.
            let buf = construct_valid_frame("FIX.4.4", "35=A|34=42|49=ME|56=YOU|");
            assert_eq!(frame(&buf).msg_type, b"A".as_slice());
        }
    }

    mod truncation {
        use super::*;

        #[test]
        fn every_prefix_of_a_valid_frame_is_incomplete() {
            let frames = [
                construct_valid_frame("FIX.4.4", "35=A|"),
                construct_valid_frame("FIX.4.4", "35=D|34=2|49=SENDER|56=TARGET|"),
                construct_valid_frame("FIXT.1.1", "35=A|1128=9|"),
                construct_valid_frame("FIX.4.2", "35=0|"),
            ];
            for full in &frames {
                for len in 0..full.len() {
                    assert!(
                        matches!(scan(&full[..len]), Outcome::Incomplete),
                        "prefix of length {len} (of a {}-byte frame) should be Incomplete",
                        full.len(),
                    );
                }
            }
        }
    }

    mod chunking {
        use super::*;

        /// Drives the scanner over `stream` delivered in `chunk_size`-byte pieces,
        /// returning the frame byte-slices produced, in order. Models a caller that
        /// accumulates bytes, consumes complete frames/garbles, waits on Incomplete.
        fn drive(stream: &[u8], chunk_size: usize) -> Vec<Vec<u8>> {
            let scanner = Scanner::default();
            let mut frames = Vec::new();
            let mut buf: Vec<u8> = Vec::new();
            for chunk in stream.chunks(chunk_size) {
                buf.extend_from_slice(chunk);
                loop {
                    let advance = match scanner.scan(&buf) {
                        Outcome::Frame(f) => {
                            frames.push(f.bytes.to_vec());
                            f.bytes.len()
                        }
                        Outcome::Garbled { skipped, .. } => skipped,
                        Outcome::Incomplete => break,
                    };
                    buf.drain(..advance);
                }
            }
            frames
        }

        #[test]
        fn clean_stream_is_chunk_invariant() {
            let f1 = construct_valid_frame("FIX.4.4", "35=A|34=1|");
            let f2 = construct_valid_frame("FIXT.1.1", "35=D|1128=9|");
            let mut stream = f1.clone();
            stream.extend_from_slice(&f2);

            let whole = drive(&stream, stream.len());
            assert_eq!(whole, vec![f1, f2]);
            for chunk_size in 1..=stream.len() {
                assert_eq!(
                    drive(&stream, chunk_size),
                    whole,
                    "chunk_size = {chunk_size}"
                );
            }
        }

        #[test]
        fn garbage_before_frame_is_chunk_invariant() {
            // Regression: leading non-`8=` garbage, then `8=<non-FIX>` with no SOH
            // before the real frame's BeginString SOH, then a valid frame. Under a
            // weak `8=` start anchor, byte-by-byte delivery latched onto the `8=Z`,
            // absorbed the real frame into a bogus BeginString, and skipped it on
            // the checksum failure - losing a frame the whole-buffer scan kept. The
            // `8=FIX` anchor makes both paths skip the garbage identically.
            let f = construct_valid_frame("FIX.4.4", "35=0|");
            let mut stream = b"YY8=Z".to_vec();
            stream.extend_from_slice(&f);

            let whole = drive(&stream, stream.len());
            assert_eq!(whole, vec![f]);
            for chunk_size in 1..=stream.len() {
                assert_eq!(
                    drive(&stream, chunk_size),
                    whole,
                    "chunk_size = {chunk_size}"
                );
            }
        }
    }
}
