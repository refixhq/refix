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

        todo!()
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
/// resyncs by searching for the next BeginString. Not for checksum garbles: their
/// extent *is* trusted, so they skip the whole frame length instead.
fn structural_garble(reason: GarbledReason, buf: &[u8]) -> Halt {
    Halt::Garbled {
        reason,
        skipped: find_next_begin_string(buf),
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
        return Err(Halt::Garbled {
            reason: GarbledReason::MalformedChecksum,
            skipped: frame.len(),
        });
    }

    let stated = u16::from(digits[0] - b'0') * 100
        + u16::from(digits[1] - b'0') * 10
        + u16::from(digits[2] - b'0');
    let computed = frame[..checksum_start]
        .iter()
        .fold(0u8, |acc, &b| acc.wrapping_add(b));

    if u16::from(computed) != stated {
        return Err(Halt::Garbled {
            reason: GarbledReason::ChecksumMismatch,
            skipped: frame.len(),
        });
    }

    Ok(())
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
