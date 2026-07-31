import pytest

import refix
from refix.errors import TokenizeError


def to_wire(frame: str) -> bytes:
    return frame.replace("|", "\x01").encode()


def test_tokenize_valid_frame():
    frame = to_wire("8=FIX.4.4|9=25|35=0|49=SENDER|56=TARGET|10=177|")

    message = refix.Tokenizer().tokenize(frame)

    assert message.bytes == frame
    assert message.get(35) == b"0"
    assert message.get(49) == b"SENDER"
    assert message.get(58) is None


def test_tokenize_truncated_frame_raises():
    frame = to_wire("8=FIX.4.4|9=5|35=0|10=163|")

    with pytest.raises(TokenizeError):
        refix.Tokenizer().tokenize(frame[:-1])
