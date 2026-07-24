const SOH: u8 = 0x01;

/// Longest BeginString field we'll frame: `8=` through the byte before its SOH.
const MAX_BEGIN_STRING_LEN: usize = 16;

/// Longest BodyLength value we'll frame, in digits.
const MAX_BODY_LENGTH_DIGITS: usize = 10;

/// `10=` + three checksum digits + SOH.
const CHECKSUM_FIELD_LEN: usize = 7;

/// The shortest prefix common to every BeginString (`FIX.4.x` and `FIXT.1.1`).
/// Used in the resync logic when scanning for the next frame start (i.e. the next BeginString).
const RESYNC_BEGIN_STRING_PREFIX: &[u8] = b"8=FIX";

pub struct FrameScanner {
    max_frame_size: usize,
}

impl FrameScanner {
    pub fn new(max_frame_size: usize) -> Self {
        Self { max_frame_size }
    }

    pub fn scan<'a>(&self, buf: &'a [u8]) -> ScanOutcome<'a> {
        match self.try_scan(buf) {
            Ok(frame) => ScanOutcome::Frame(frame),
            Err(halt) => halt.into(),
        }
    }

    fn try_scan<'a>(&self, buf: &'a [u8]) -> Result<Frame<'a>, Halt> {
        if buf.is_empty() || buf == b"8" {
            return Err(Halt::Incomplete);
        }
        if !buf.starts_with(b"8=") {
            return Err(structural_garble(GarbledReason::MissingBeginString, buf));
        }

        let (begin_string, next) = parse_begin_string(buf)?;
        let (body_length, body_start) = parse_body_length(buf, next)?;

        let frame_len = validated_frame_len(buf, body_start, body_length, self.max_frame_size)?;
        let frame = &buf[..frame_len];
        let checksum_start = body_start + body_length;

        verify_trailer_start(buf, checksum_start)?;
        verify_checksum(frame, checksum_start)?;

        let header = extract_header(frame, body_start, checksum_start, begin_string, body_length)?;
        Ok(Frame {
            bytes: frame,
            header,
        })
    }
}

/// Bytes to skip to resynchronise onto the next frame start.
///
/// Returns the offset of the next `8=FIX` after offset 0. If none is present,
/// preserves the longest partial `8=FIX` prefix hanging off the tail.
/// Failing that, skips the whole buffer.
fn find_next_begin_string(buf: &[u8]) -> usize {
    if let Some(skip) = buf[1..]
        .windows(RESYNC_BEGIN_STRING_PREFIX.len())
        .position(|b| b == RESYNC_BEGIN_STRING_PREFIX)
    {
        return skip + 1;
    }

    let max_k = (RESYNC_BEGIN_STRING_PREFIX.len() - 1).min(buf.len());
    for k in (1..=max_k).rev() {
        if buf[buf.len() - k..] == RESYNC_BEGIN_STRING_PREFIX[..k] {
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

/// Parses the `9=<BodyLength>␁` field starting at `next`.
///
/// On success returns `(body_length, body_start)`, where `body_start` is the
/// offset just past the BodyLength SOH.
fn parse_body_length(buf: &[u8], next: usize) -> Result<(usize, usize), Halt> {
    // Expect the `9=` tag. Fewer than 2 bytes → can't even test it yet.
    let tag = &buf[next..];
    if tag.len() < 2 {
        return Err(Halt::Incomplete);
    }
    if &tag[..2] != b"9=" {
        return Err(structural_garble(GarbledReason::MissingBodyLength, buf));
    }

    // Digits run from just after `9=` to the terminating SOH, bounded by the cap.
    let digits_start = next + 2;
    let search_end = buf.len().min(digits_start + MAX_BODY_LENGTH_DIGITS + 1);
    let soh_rel = match buf[digits_start..search_end].iter().position(|&b| b == SOH) {
        Some(rel) => rel,
        // Out-of-buffer first, cap-exceeded second — same ordering as BeginString.
        None if buf.len() <= digits_start + MAX_BODY_LENGTH_DIGITS => return Err(Halt::Incomplete),
        None => return Err(structural_garble(GarbledReason::MalformedBodyLength, buf)),
    };

    let digits = &buf[digits_start..digits_start + soh_rel];
    if digits.is_empty() || !digits.iter().all(|b| b.is_ascii_digit()) {
        return Err(structural_garble(GarbledReason::MalformedBodyLength, buf));
    }

    // At most MAX_BODY_LENGTH_DIGITS ASCII digits, already validated - no overflow.
    let body_length = digits
        .iter()
        .fold(0usize, |acc, &b| acc * 10 + (b - b'0') as usize);

    let body_start = digits_start + soh_rel + 1; // just past the BodyLength SOH
    Ok((body_length, body_start))
}

/// The total frame length implied by BodyLength, bounded and fully buffered.
///
/// `FrameTooLarge` if it exceeds `max_frame_size`; `Incomplete` if the frame
/// hasn't fully arrived. The frame's *content* — the `10=` trailer and checksum —
/// is not inspected here.
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

/// Verifies the `<CheckSum>␁` at `checksum_start`, the `10=` landing having
/// already been confirmed.
fn verify_checksum(frame: &[u8], checksum_start: usize) -> Result<(), Halt> {
    let digits = &frame[checksum_start + 3..checksum_start + 6];
    let trailing_soh = frame[checksum_start + 6];
    if trailing_soh != SOH || !digits.iter().all(|b| b.is_ascii_digit()) {
        return Err(post_structural_garble(GarbledReason::MalformedChecksum, frame));
    }

    let stated = u16::from(digits[0] - b'0') * 100
        + u16::from(digits[1] - b'0') * 10
        + u16::from(digits[2] - b'0');
    let computed = frame[..checksum_start]
        .iter()
        .fold(0u8, |acc, &b| acc.wrapping_add(b));

    if u16::from(computed) != stated {
        return Err(post_structural_garble(GarbledReason::ChecksumMismatch, frame));
    }

    Ok(())
}

/// Extracts the session-relevant header fields from the body region.
fn extract_header<'a>(
    frame: &'a [u8],
    body_start: usize,
    checksum_start: usize,
    begin_string: BeginString<'a>,
    body_length: usize,
) -> Result<StandardHeader<'a>, Halt> {
    let body = &frame[body_start..checksum_start];
    let is_fixt = matches!(begin_string, BeginString::Fixt11);

    let mut fields = body.split(|&b| b == SOH).filter(|f| !f.is_empty());

    // MsgType(35) must be the first field of the body.
    let msg_type = match fields.next().and_then(split_field) {
        Some((b"35", value)) => value,
        _ => {
            return Err(post_structural_garble(GarbledReason::MissingMsgType, frame));
        }
    };

    let mut header = StandardHeader {
        begin_string,
        body_length,
        msg_type,
        msg_seq_num: None,
        sender_comp_id: None,
        target_comp_id: None,
        poss_dup: None,
        sending_time: None,
        orig_sending_time: None,
        appl_ver_id: None,
    };

    // First occurrence of each tag wins; unrecognised tags are skipped.
    for field in fields {
        let Some((tag, value)) = split_field(field) else {
            continue;
        };
        match tag {
            b"34" if header.msg_seq_num.is_none() => header.msg_seq_num = parse_u64(value),
            b"49" if header.sender_comp_id.is_none() => header.sender_comp_id = Some(value),
            b"56" if header.target_comp_id.is_none() => header.target_comp_id = Some(value),
            b"43" if header.poss_dup.is_none() => header.poss_dup = parse_bool(value),
            b"52" if header.sending_time.is_none() => header.sending_time = Some(value),
            b"122" if header.orig_sending_time.is_none() => header.orig_sending_time = Some(value),
            b"1128" if is_fixt && header.appl_ver_id.is_none() => header.appl_ver_id = Some(value),
            _ => {}
        }
    }

    Ok(header)
}

/// Splits a `tag=value` field on its first `=`. `None` if there's no `=`.
fn split_field(field: &[u8]) -> Option<(&[u8], &[u8])> {
    let eq = field.iter().position(|&b| b == b'=')?;
    Some((&field[..eq], &field[eq + 1..]))
}

/// Parses ASCII digits to `u64`. `None` if empty, non-digit, or it overflows.
fn parse_u64(bytes: &[u8]) -> Option<u64> {
    if bytes.is_empty() {
        return None;
    }
    let mut n: u64 = 0;
    for &b in bytes {
        let digit = b.checked_sub(b'0').filter(|&d| d < 10)?;
        n = n.checked_mul(10)?.checked_add(u64::from(digit))?;
    }
    Some(n)
}

/// `PossDupFlag`: `Y`/`N` to bool, anything else `None`.
fn parse_bool(value: &[u8]) -> Option<bool> {
    match value {
        b"Y" => Some(true),
        b"N" => Some(false),
        _ => None,
    }
}

impl Default for FrameScanner {
    fn default() -> Self {
        Self {
            max_frame_size: 1 << 20,
        }
    }
}

pub enum ScanOutcome<'a> {
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
        /// Always ≥ 1: the caller advances by this many bytes, so a garbled frame never stalls the stream.
        skipped: usize,
    },
}

pub struct Frame<'a> {
    /// The full frame: 8= through the SOH after CheckSum.
    pub bytes: &'a [u8],
    /// The session-relevant fields in the header.
    pub header: StandardHeader<'a>,
}

/// A scan that ended without a frame.
enum Halt {
    Incomplete,
    Garbled {
        reason: GarbledReason,
        skipped: usize,
    },
}

impl<'a> From<Halt> for ScanOutcome<'a> {
    fn from(halt: Halt) -> Self {
        match halt {
            Halt::Incomplete => ScanOutcome::Incomplete,
            Halt::Garbled { reason, skipped } => ScanOutcome::Garbled { reason, skipped },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GarbledReason {
    /// It does not start with `8=`.
    MissingBeginString,
    /// `8=` present, but no SOH within MAX_BEGIN_STRING_LEN.
    MalformedBeginString,
    /// The second tag is not `9=`.
    MissingBodyLength,
    /// BodyLength is empty, contains a non-digit, or exceeds the digit cap.
    MalformedBodyLength,
    /// The BodyLength-implied offset is not a `10=` trailer at a field boundary
    /// (i.e. not preceded by SOH, or not `10=`).
    BodyLengthMismatch,
    /// The third tag is not `35=`.
    MissingMsgType,
    /// The CheckSum field is not three digits followed by SOH.
    MalformedChecksum,
    /// The calculated checksum does not match the value of CheckSum.
    ChecksumMismatch,
    /// The BodyLength-implied frame length exceeds max_frame_size.
    FrameTooLarge,
}

/// The session-relevant subset of a message's header.
///
/// Produced only as part of a [`Frame`], so its existence guarantees
/// the message was well-framed and checksum-verified, never garbled.
/// It makes no validity claims beyond that. Fields required for framing are
/// always present, whilst fields the session layer needs are optional.
/// The values of these fields are checked by the validation and session
/// layers, not here.
pub struct StandardHeader<'a> {
    /// `BeginString(8)`
    pub begin_string: BeginString<'a>,
    /// `BodyLength(9)`
    pub body_length: usize,
    /// `MsgType(35)`
    pub msg_type: &'a [u8],
    /// `MsgSeqNum(34)`: `None` when absent **or** unparseable - the session layer treats
    /// both the same way (gap filling is impossible, so logout is initiated).
    pub msg_seq_num: Option<u64>,
    /// `SenderCompID(49)`
    pub sender_comp_id: Option<&'a [u8]>,
    /// `TargetCompID(56)`
    pub target_comp_id: Option<&'a [u8]>,
    /// `PossDupFlag(43)`
    pub poss_dup: Option<bool>,
    /// `SendingTime(52)` as raw bytes.
    pub sending_time: Option<&'a [u8]>,
    /// `OrigSendingTime(122)` as raw bytes.
    pub orig_sending_time: Option<&'a [u8]>,
    /// `ApplVerID(1128)`, only extracted on FIXT sessions.
    pub appl_ver_id: Option<&'a [u8]>,
}

pub enum BeginString<'a> {
    Fix40,
    Fix41,
    Fix42,
    Fix43,
    Fix44,
    Fixt11,
    Other(&'a [u8]),
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `|`-delimited readable string into SOH-delimited bytes.
    fn to_wire(s: &str) -> Vec<u8> {
        s.replace('|', "\x01").into_bytes()
    }

    /// FIX checksum: sum of all bytes mod 256, as a zero-padded 3-digit string.
    fn calculate_checksum(bytes: &[u8]) -> String {
        let sum = bytes.iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
        format!("{sum:03}")
    }

    /// A complete, correctly-checksummed frame with a correct BodyLength.
    /// `body` is `|`-delimited and includes its trailing `|`, e.g. "35=A|34=1|".
    fn construct_valid_frame(begin: &str, body: &str) -> Vec<u8> {
        let body = to_wire(body);
        let mut bytes = to_wire(&format!("8={begin}|9={}|", body.len()));
        bytes.extend_from_slice(&body);
        let cs = calculate_checksum(&bytes);
        bytes.extend_from_slice(&to_wire(&format!("10={cs}|")));
        bytes
    }

    fn scan(buf: &[u8]) -> ScanOutcome<'_> {
        FrameScanner::default().scan(buf)
    }

    #[track_caller]
    fn assert_garbled(out: ScanOutcome, reason: GarbledReason, skipped: usize) {
        match out {
            ScanOutcome::Garbled {
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
    fn assert_incomplete(out: ScanOutcome) {
        assert!(
            matches!(out, ScanOutcome::Incomplete),
            "expected Incomplete"
        );
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
        fn malformed_begin_string_overruns_bound() {
            let mut buf = b"8=".to_vec();
            buf.extend([b'X'; 18]); // 20 bytes, no SOH within MAX_BEGIN_STRING_LEN
            assert_garbled(scan(&buf), GarbledReason::MalformedBeginString, 20);
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
        fn frame_too_large_rejected_before_waiting() {
            // frame_len of ~123 exceeds the cap of 50, so it rejects rather than waits.
            let buf = to_wire("8=FIX.4.4|9=100|");
            assert_garbled(
                FrameScanner::new(50).scan(&buf),
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
    }

    mod happy_path {
        use super::*;

        #[test]
        fn valid_frame_scans() {
            let buf = construct_valid_frame("FIX.4.4", "35=A|34=1|49=ME|56=YOU|");
            assert!(matches!(scan(&buf), ScanOutcome::Frame(_)));
        }
    }
}
