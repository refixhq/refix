const SOH: u8 = 0x01;

/// Longest BeginString field we'll frame: `8=` through the byte before its SOH.
/// `8=FIXT.1.1` (10 bytes) is the longest real one; the slack leaves room for a
/// plausible-but-unrecognised version (→ `Other`) without ever mistaking a run
/// of garbage after `8=` for a slow-arriving field.
const MAX_BEGIN_STRING_LEN: usize = 16;

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
        if buf.is_empty() || buf == b"8" {
            return ScanOutcome::Incomplete;
        }

        if !buf.starts_with(b"8=") {
            return ScanOutcome::Garbled {
                reason: GarbledReason::MissingBeginString,
                skipped: find_next_begin_string(buf),
            };
        }

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

pub enum GarbledReason {
    /// It does not start with `8=`.
    MissingBeginString,
    /// `8=` present, but no SOH within MAX_BEGIN_STRING_LEN.
    MalformedBeginString,
    /// The second tag is not `9=`.
    MissingBodyLength,
    /// BodyLength is empty, contains a non-digit, or exceeds the digit cap.
    MalformedBodyLength,
    /// The bytes at the BodyLength-implied offset are not `10=`.
    BodyLengthMismatch,
    /// The third tag is not `35=`.
    MissingMsgType,
    /// CheckSum is present but its value is not exactly three digits.
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
    /// `MsgSeqNum(34)`: `None` when absent **or** unparseable — the session layer treats
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
