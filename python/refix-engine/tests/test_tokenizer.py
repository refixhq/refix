import pickle

import pytest

import refix
from refix.errors import (
    GarbledError,
    GarbledReason,
    IncompleteError,
    TooLargeToIndexError,
    TrailingBytesError,
)


def to_wire(frame: str) -> bytes:
    """`|`-delimited readable string into SOH-delimited bytes."""
    return frame.replace("|", "\x01").encode()


def construct_valid_frame(begin: str, body: str) -> bytes:
    """A complete, correctly-checksummed frame with a correct BodyLength.

    `body` is `|`-delimited and includes its trailing `|`, e.g. "35=A|34=1|".
    """
    wire_body = to_wire(body)
    frame = to_wire(f"8={begin}|9={len(wire_body)}|") + wire_body
    return frame + to_wire(f"10={sum(frame) % 256:03d}|")


def tokenize_body(body: str) -> refix.RawMessage:
    """Tokenizes a `|`-delimited body wrapped in a valid FIX.4.4 frame."""
    return refix.Tokenizer().tokenize(construct_valid_frame("FIX.4.4", body))


def body_entries(message: refix.RawMessage) -> list[tuple[int, bytes]]:
    """The fields between the preamble (8, 9) and the trailer (10), so bodies
    can be asserted without hand-computing BodyLength and CheckSum.
    """
    entries = message.entries()
    tags = [tag for tag, _ in entries]
    assert tags[:2] == [8, 9], "frame must open with BeginString, BodyLength"
    assert tags[-1] == 10, "frame must close with CheckSum"
    return entries[2:-1]


class TestWellFormed:
    def test_valid_message(self):
        frame = to_wire(
            "8=FIX.4.4|9=41|35=0|49=A|56=B|34=1|52=20260730-10:00:00|10=123|"
        )

        message = refix.Tokenizer().tokenize(frame)

        assert message.bytes == frame
        assert message.entries() == [
            (8, b"FIX.4.4"),
            (9, b"41"),
            (35, b"0"),
            (49, b"A"),
            (56, b"B"),
            (34, b"1"),
            (52, b"20260730-10:00:00"),
            (10, b"123"),
        ]

    def test_empty_value(self):
        message = tokenize_body("35=0|58=|")
        assert body_entries(message) == [(35, b"0"), (58, b"")]

    def test_value_containing_equals(self):
        message = tokenize_body("35=0|58=px=1.23|")
        assert body_entries(message) == [(35, b"0"), (58, b"px=1.23")]

    def test_duplicate_tags_all_indexed(self):
        message = tokenize_body("35=0|58=first|58=second|")
        assert body_entries(message) == [(35, b"0"), (58, b"first"), (58, b"second")]

    def test_get_returns_first_occurrence(self):
        message = tokenize_body("35=0|58=first|58=second|")
        assert message.get(58) == b"first"

    def test_get_absent_tag(self):
        message = tokenize_body("35=0|")
        assert message.get(58) is None

    def test_preamble_and_trailer_are_ordinary_fields(self):
        message = tokenize_body("35=0|")
        assert message.get(8) == b"FIX.4.4"
        assert message.get(9) == b"5"
        assert message.get(35) == b"0"
        assert message.get(10) is not None


class TestErrors:
    def test_truncated_frame_is_incomplete(self):
        frame = construct_valid_frame("FIX.4.4", "35=0|")

        with pytest.raises(IncompleteError):
            refix.Tokenizer().tokenize(frame[:-1])

    def test_absurd_body_length_is_too_large_to_index(self):
        frame = to_wire("8=FIX.4.4|9=4294967295|35=0|")

        with pytest.raises(TooLargeToIndexError):
            refix.Tokenizer().tokenize(frame)

    def test_two_messages_are_trailing_bytes(self):
        frame = construct_valid_frame("FIX.4.4", "35=0|")

        with pytest.raises(TrailingBytesError) as excinfo:
            refix.Tokenizer().tokenize(frame + frame)

        assert excinfo.value.frame_len == len(frame)
        assert str(excinfo.value) == f"one message of {len(frame)} bytes, then trailing input"


class TestGarbledFrames:
    """One frame per garbled reason reachable through `tokenize`.

    `FRAME_TOO_LARGE` is the exception: it surfaces as `TooLargeToIndexError`.
    """

    FRAMES = [
        ("9=5|35=0|10=163|", GarbledReason.MISSING_BEGIN_STRING),
        ("8=FIX.4.4" + "A" * 40, GarbledReason.MALFORMED_BEGIN_STRING),
        ("8=FIX.4.4|35=0|10=163|", GarbledReason.MISSING_BODY_LENGTH),
        ("8=FIX.4.4|9=5x|35=0|10=163|", GarbledReason.MALFORMED_BODY_LENGTH),
        ("8=FIX.4.4|9=3|35=0|10=163|", GarbledReason.BODY_LENGTH_MISMATCH),
        ("8=FIX.4.4|9=5|49=A|10=185|", GarbledReason.MISSING_MSG_TYPE),
        ("8=FIX.4.4|9=5|35=0|10=1x3|", GarbledReason.MALFORMED_CHECKSUM),
        ("8=FIX.4.4|9=5|35=0|10=164|", GarbledReason.CHECKSUM_MISMATCH),
    ]

    @pytest.mark.parametrize(("frame", "reason"), FRAMES)
    def test_garbled_frame_raises_with_reason(self, frame: str, reason: GarbledReason):
        with pytest.raises(GarbledError) as excinfo:
            refix.Tokenizer().tokenize(to_wire(frame))

        assert excinfo.value.reason is reason
        assert str(excinfo.value) == f"garbled frame: {reason.value}"

    def test_garbled_error_pickles(self):
        with pytest.raises(GarbledError) as excinfo:
            refix.Tokenizer().tokenize(to_wire("8=FIX.4.4|9=5|35=0|10=164|"))

        clone = pickle.loads(pickle.dumps(excinfo.value))
        assert clone.reason is GarbledReason.CHECKSUM_MISMATCH


class TestSentinelRuns:
    """Tests for the MALFORMED_TAG sentinel value."""

    def test_run_without_equals(self):
        message = tokenize_body("35=0|junk|58=ok|")
        assert body_entries(message) == [
            (35, b"0"),
            (refix.MALFORMED_TAG, b"junk"),
            (58, b"ok"),
        ]

    def test_fields_after_a_fault_remain_readable(self):
        message = tokenize_body("35=0|junk|58=ok|")
        assert message.get(58) == b"ok"

    def test_non_numeric_tag(self):
        message = tokenize_body("35=0|abc=x|")
        assert body_entries(message) == [(35, b"0"), (refix.MALFORMED_TAG, b"x")]

    def test_empty_tag(self):
        message = tokenize_body("35=0|=x|")
        assert body_entries(message) == [(35, b"0"), (refix.MALFORMED_TAG, b"x")]

    def test_literal_tag_zero_is_a_sentinel(self):
        message = tokenize_body("35=0|0=x|")
        assert body_entries(message) == [(35, b"0"), (refix.MALFORMED_TAG, b"x")]

    def test_tag_overflowing_u32(self):
        message = tokenize_body("35=0|4294967296=x|")
        assert body_entries(message) == [(35, b"0"), (refix.MALFORMED_TAG, b"x")]

    def test_consecutive_runs(self):
        message = tokenize_body("35=0|junk|more|58=ok|")
        assert body_entries(message) == [
            (35, b"0"),
            (refix.MALFORMED_TAG, b"junk"),
            (refix.MALFORMED_TAG, b"more"),
            (58, b"ok"),
        ]
