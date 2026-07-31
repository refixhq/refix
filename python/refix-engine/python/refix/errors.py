import enum


class GarbledReason(enum.Enum):
    MISSING_BEGIN_STRING = "missing_begin_string"
    MALFORMED_BEGIN_STRING = "malformed_begin_string"
    MISSING_BODY_LENGTH = "missing_body_length"
    MALFORMED_BODY_LENGTH = "malformed_body_length"
    BODY_LENGTH_MISMATCH = "body_length_mismatch"
    MISSING_MSG_TYPE = "missing_msg_type"
    MALFORMED_CHECKSUM = "malformed_checksum"
    CHECKSUM_MISMATCH = "checksum_mismatch"
    FRAME_TOO_LARGE = "frame_too_large"


class TokenizeError(Exception):
    """The bytes handed to the tokenizer are not a valid FIX message."""


class GarbledError(TokenizeError):
    """The bytes fail the frame-level checks."""

    def __init__(self, reason: str | GarbledReason) -> None:
        super().__init__(reason)
        self.reason = GarbledReason(reason)

    def __str__(self) -> str:
        return f"garbled frame: {self.reason.value}"


class IncompleteError(TokenizeError):
    """The bytes end mid-message."""


class TrailingBytesError(TokenizeError):
    """One message was found, but the input continues past it."""

    def __init__(self, frame_len: int) -> None:
        super().__init__(frame_len)
        self.frame_len = frame_len

    def __str__(self) -> str:
        return f"one message of {self.frame_len} bytes, then trailing input"


class TooLargeToIndexError(TokenizeError):
    """The frame is too long for the index's offsets to address."""
