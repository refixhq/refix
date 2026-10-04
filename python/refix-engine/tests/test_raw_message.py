import pickle

import pytest
import refix
from refix.errors import InvalidValueError
from test_tokenizer import tokenize_body


def tokenize_bytes(body: bytes) -> refix.RawMessage:
    """Tokenizes a `|`-delimited body of raw bytes in a valid FIX.4.4 frame."""
    wire_body = body.replace(b"|", b"\x01")
    frame = f"8=FIX.4.4\x019={len(wire_body)}\x01".encode() + wire_body
    return refix.Tokenizer().tokenize(frame + f"10={sum(frame) % 256:03d}\x01".encode())


class TestGetStr:
    def test_reads_text(self):
        assert tokenize_body("35=D|58=hello|").get_str(58) == "hello"

    def test_absent_tag_is_none(self):
        assert tokenize_body("35=D|").get_str(58) is None

    def test_empty_value_is_empty_text(self):
        assert tokenize_body("35=D|58=|").get_str(58) == ""

    def test_invalid_utf8_raises(self):
        message = tokenize_bytes(b"35=D|58=caf\xe9|")

        with pytest.raises(InvalidValueError) as excinfo:
            message.get_str(58)

        assert excinfo.value.tag == 58


class TestGetInt:
    def test_reads_an_integer(self):
        assert tokenize_body("35=D|38=200|").get_int(38) == 200

    def test_reads_a_negative_integer(self):
        assert tokenize_body("35=D|38=-5|").get_int(38) == -5

    def test_absent_tag_is_none(self):
        assert tokenize_body("35=D|").get_int(38) is None

    def test_garbage_raises(self):
        with pytest.raises(InvalidValueError) as excinfo:
            tokenize_body("35=D|38=12x3|").get_int(38)

        assert excinfo.value.tag == 38


class TestInvalidValueError:
    def test_is_a_value_error(self):
        assert issubclass(InvalidValueError, ValueError)

    def test_names_the_field(self):
        assert str(InvalidValueError(38)) == "invalid value in field 38"

    def test_pickles(self):
        clone = pickle.loads(pickle.dumps(InvalidValueError(38)))
        assert clone.tag == 38
