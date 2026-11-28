import enum
import pickle

import pytest
import refix
from refix.errors import WriteError


class Side(enum.StrEnum):
    BUY = "1"


class TestWrite:
    def test_writes_fields_in_the_order_of_the_calls(self):
        writer = refix.FieldWriter()
        writer.write_str(453, "2")
        writer.write_str(448, "AL")
        writer.write(448, b"BX")

        message = writer.finish()

        assert message.bytes == b"453=2\x01448=AL\x01448=BX\x01"
        assert message.entries() == [(453, b"2"), (448, b"AL"), (448, b"BX")]

    def test_written_fields_read_back_through_the_getters(self):
        writer = refix.FieldWriter()
        writer.write_str(11, "ORDER-1")
        writer.write(58, b"caf\xe9")

        message = writer.finish()

        assert message.get_str(11) == "ORDER-1"
        assert message.get(58) == b"caf\xe9"
        assert message.get(55) is None

    def test_writes_a_str_enum_as_its_value(self):
        writer = refix.FieldWriter()
        writer.write_str(54, Side.BUY)

        assert writer.finish().bytes == b"54=1\x01"

    @pytest.mark.parametrize("value", ["", "late\x0110=000"])
    def test_rejected_value_raises_and_writes_nothing(self, value: str):
        writer = refix.FieldWriter()
        writer.write_str(11, "ORDER-1")

        with pytest.raises(WriteError) as excinfo:
            writer.write_str(58, value)

        assert excinfo.value.tag == 58
        assert writer.finish().bytes == b"11=ORDER-1\x01"

    def test_names_what_is_wrong_with_the_value(self):
        writer = refix.FieldWriter()

        with pytest.raises(WriteError, match="empty value for field 58"):
            writer.write(58, b"")
        with pytest.raises(WriteError, match="separator in the value for field 58"):
            writer.write(58, b"a\x01b")


class TestFinish:
    def test_a_writer_with_nothing_written_gives_no_fields(self):
        message = refix.FieldWriter().finish()

        assert message.bytes == b""
        assert message.entries() == []

    def test_starts_again_with_no_fields(self):
        writer = refix.FieldWriter()
        writer.write_str(11, "ORDER-1")
        first = writer.finish()
        writer.write_str(11, "ORDER-2")

        assert writer.finish().bytes == b"11=ORDER-2\x01"
        assert first.bytes == b"11=ORDER-1\x01"


class TestWriteError:
    def test_is_a_value_error(self):
        assert issubclass(WriteError, ValueError)

    def test_pickles(self):
        error = WriteError(58, "empty value for field 58")

        clone = pickle.loads(pickle.dumps(error))

        assert clone.tag == 58
        assert str(clone) == "empty value for field 58"
