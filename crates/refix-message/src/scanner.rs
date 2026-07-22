const SOH: u8 = 0x01;

/// Longest BeginString field we'll frame: `8=` through the byte before its SOH.
/// `8=FIXT.1.1` (10 bytes) is the longest real one; the slack leaves room for a
/// plausible-but-unrecognised version (→ `Other`) without ever mistaking a run
/// of garbage after `8=` for a slow-arriving field.
const MAX_BEGIN_STRING_LEN: usize = 16;

pub struct FrameScanner {
    max_frame_size: usize,
}

impl FrameScanner {
    pub fn new(max_frame_size: usize) -> Self {
        Self { max_frame_size }
    }

    pub fn scan<'a>(&self, _buf: &'a [u8]) -> ScanOutcome<'a> {
        todo!()
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
        reason: GarbledReason,
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
