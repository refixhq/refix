import enum
import pickle
from decimal import Decimal

import pytest
import refix
from refix.errors import WriteError


class Side(enum.StrEnum):
    BUY = "1"


class PartyRole(enum.IntEnum):
    CLIENT_ID = 3


def written(write: str, tag: int, value: object) -> refix.RawMessage:
    """Returns the message of one field written with the method named `write`."""
    writer = refix.FieldWriter()
    getattr(writer, write)(tag, value)
    return writer.finish()


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


class TestWriteInt:
    @pytest.mark.parametrize("value", [0, 100, -5, 2**63 - 1, -(2**63)])
    def test_writes_an_integer_that_reads_back(self, value: int):
        message = written("write_int", 38, value)

        assert message.get_str(38) == str(value)
        assert message.get_int(38) == value

    def test_writes_an_int_enum_as_its_value(self):
        assert written("write_int", 452, PartyRole.CLIENT_ID).bytes == b"452=3\x01"

    def test_bool_raises(self):
        with pytest.raises(TypeError, match="write_int takes an int, not bool"):
            written("write_int", 43, True)

    def test_integer_past_64_bits_raises(self):
        with pytest.raises(OverflowError):
            written("write_int", 38, 2**63)


class TestWriteBool:
    def test_writes_y_and_n(self):
        writer = refix.FieldWriter()
        writer.write_bool(43, True)
        writer.write_bool(97, False)

        message = writer.finish()

        assert message.bytes == b"43=Y\x0197=N\x01"
        assert message.get_bool(43) is True
        assert message.get_bool(97) is False

    def test_int_raises(self):
        with pytest.raises(TypeError):
            written("write_bool", 43, 1)


class TestWriteDecimal:
    @pytest.mark.parametrize(
        ("value", "text"),
        [
            (Decimal("101.25"), "101.25"),
            (Decimal("1.50"), "1.50"),
            (Decimal("-0.005"), "-0.005"),
            (Decimal("1.5E+3"), "1500"),
            (Decimal("5E-7"), "0.0000005"),
            (Decimal(0), "0"),
        ],
    )
    def test_writes_a_decimal_with_its_scale(self, value: Decimal, text: str):
        message = written("write_decimal", 44, value)

        assert message.get_str(44) == text
        assert message.get_decimal(44) == value

    @pytest.mark.parametrize("value", [100, -5, 2**70])
    def test_writes_an_int(self, value: int):
        assert written("write_decimal", 38, value).get_str(38) == str(value)

    @pytest.mark.parametrize("value", [1.5, "1.5", True])
    def test_other_types_raise(self, value: object):
        with pytest.raises(TypeError):
            written("write_decimal", 44, value)

    @pytest.mark.parametrize(("value", "name"), [(1.5, "float"), (True, "bool")])
    def test_names_the_type_it_does_not_take(self, value: object, name: str):
        message = f"write_decimal takes a Decimal or an int, not {name}"
        with pytest.raises(TypeError, match=message):
            written("write_decimal", 44, value)

    @pytest.mark.parametrize("text", ["NaN", "sNaN", "Infinity", "-Infinity"])
    def test_values_that_are_not_numbers_raise(self, text: str):
        with pytest.raises(WriteError) as excinfo:
            written("write_decimal", 44, Decimal(text))

        assert excinfo.value.tag == 44
        assert str(excinfo.value) == f"{text} cannot be written to field 44"


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
