import enum
import io
import pickle
from datetime import UTC, date, datetime, time, timedelta, timezone
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


class TestWriteDate:
    def test_writes_a_date_that_reads_back(self):
        message = written("write_date", 75, date(2026, 11, 3))

        assert message.get_str(75) == "20261103"
        assert message.get_date(75) == date(2026, 11, 3)

    @pytest.mark.parametrize(
        ("value", "name"),
        [(datetime(2026, 11, 3, tzinfo=UTC), "datetime"), ("20261103", "str")],
    )
    def test_other_types_raise(self, value: object, name: str):
        with pytest.raises(TypeError, match=f"write_date takes a date, not {name}"):
            written("write_date", 75, value)


class TestWriteTime:
    def test_writes_milliseconds_and_cuts_the_rest(self):
        message = written("write_time", 273, time(9, 30, 36, 250_999))

        assert message.get_str(273) == "09:30:36.250"

    @pytest.mark.parametrize(
        ("precision", "text"),
        [(0, "09:30:36"), (1, "09:30:36.2"), (6, "09:30:36.250999")],
    )
    def test_writes_the_digits_asked_for(self, precision: int, text: str):
        writer = refix.FieldWriter()
        writer.write_time(273, time(9, 30, 36, 250_999), precision=precision)

        message = writer.finish()

        assert message.get_str(273) == text
        assert message.get_time(273) == time(9, 30, 36, 250_999).replace(
            microsecond=250_999 // 10 ** (6 - precision) * 10 ** (6 - precision)
        )

    def test_precision_past_nine_digits_raises(self):
        writer = refix.FieldWriter()

        with pytest.raises(ValueError, match="precision must be from 0 to 9 digits"):
            writer.write_time(273, time(9, 30, 36), precision=10)

    def test_time_with_a_time_zone_raises(self):
        with pytest.raises(WriteError) as excinfo:
            written("write_time", 273, time(9, 30, 36, tzinfo=UTC))

        assert excinfo.value.tag == 273
        assert str(excinfo.value) == (
            "datetime.time(9, 30, 36, tzinfo=datetime.timezone.utc) "
            "cannot be written to field 273"
        )

    def test_other_types_raise(self):
        with pytest.raises(TypeError, match="write_time takes a time, not str"):
            written("write_time", 273, "09:30:36")


class TestWriteTimeNs:
    def test_writes_nanoseconds(self):
        message = written("write_time_ns", 273, 34_236_250_999_999)

        assert message.get_str(273) == "09:30:36.250999999"
        assert message.get_time_ns(273) == 34_236_250_999_999

    def test_writes_the_digits_asked_for(self):
        writer = refix.FieldWriter()
        writer.write_time_ns(273, 34_236_250_999_999, precision=3)

        assert writer.finish().get_str(273) == "09:30:36.250"

    def test_writes_a_leap_second(self):
        writer = refix.FieldWriter()
        writer.write_time_ns(273, 86_400_500_000_000, precision=3)

        assert writer.finish().get_str(273) == "23:59:60.500"

    @pytest.mark.parametrize("value", [-1, 86_401_000_000_000])
    def test_value_outside_a_day_raises(self, value: int):
        with pytest.raises(WriteError) as excinfo:
            written("write_time_ns", 273, value)

        assert excinfo.value.tag == 273


class TestWriteUtcTimestamp:
    def test_writes_milliseconds_and_cuts_the_rest(self):
        stamp = datetime(2026, 11, 3, 9, 30, 36, 250_999, tzinfo=UTC)

        message = written("write_utc_timestamp", 60, stamp)

        assert message.get_str(60) == "20261103-09:30:36.250"
        assert message.get_utc_timestamp(60) == stamp.replace(microsecond=250_000)

    @pytest.mark.parametrize(
        ("precision", "text"),
        [
            (0, "20261103-09:30:36"),
            (6, "20261103-09:30:36.250999"),
            (9, "20261103-09:30:36.250999000"),
        ],
    )
    def test_writes_the_digits_asked_for(self, precision: int, text: str):
        writer = refix.FieldWriter()
        stamp = datetime(2026, 11, 3, 9, 30, 36, 250_999, tzinfo=UTC)
        writer.write_utc_timestamp(60, stamp, precision=precision)

        assert writer.finish().get_str(60) == text

    def test_converts_another_time_zone_to_utc(self):
        zone = timezone(timedelta(hours=2))
        stamp = datetime(2026, 11, 3, 11, 30, 36, tzinfo=zone)

        message = written("write_utc_timestamp", 60, stamp)

        assert message.get_str(60) == "20261103-09:30:36.000"

    def test_writes_a_time_before_1970(self):
        stamp = datetime(1969, 12, 31, 23, 59, 59, 999_000, tzinfo=UTC)

        message = written("write_utc_timestamp", 60, stamp)

        assert message.get_str(60) == "19691231-23:59:59.999"

    def test_datetime_without_a_time_zone_raises(self):
        naive = datetime(2026, 11, 3, 9, 30, 36)  # noqa: DTZ001

        with pytest.raises(WriteError) as excinfo:
            written("write_utc_timestamp", 60, naive)

        assert excinfo.value.tag == 60
        assert str(excinfo.value) == (
            "datetime.datetime(2026, 11, 3, 9, 30, 36) cannot be written to field 60"
        )

    @pytest.mark.parametrize("year", [1, 1677, 2263, 9999])
    def test_year_that_nanoseconds_cannot_hold_raises(self, year: int):
        with pytest.raises(WriteError):
            written("write_utc_timestamp", 60, datetime(year, 6, 1, tzinfo=UTC))

    def test_other_types_raise(self):
        message = "write_utc_timestamp takes a datetime, not date"
        with pytest.raises(TypeError, match=message):
            written("write_utc_timestamp", 60, date(2026, 11, 3))


class TestWriteUtcTimestampNs:
    def test_writes_nanoseconds(self):
        message = written("write_utc_timestamp_ns", 60, 1_793_370_591_123_456_789)

        assert message.get_str(60) == "20261030-14:29:51.123456789"
        assert message.get_utc_timestamp_ns(60) == 1_793_370_591_123_456_789

    def test_writes_the_digits_asked_for(self):
        writer = refix.FieldWriter()
        writer.write_utc_timestamp_ns(60, 1_793_370_591_123_456_789, precision=3)

        assert writer.finish().get_str(60) == "20261030-14:29:51.123"


class TestWriteMonthYear:
    @pytest.mark.parametrize(
        ("value", "text"),
        [
            (refix.MonthYear(2026, 12), "202612"),
            (refix.MonthYear(2026, 12, day=18), "20261218"),
            (refix.MonthYear(2026, 12, week=3), "202612w3"),
        ],
    )
    def test_writes_each_form(self, value: refix.MonthYear, text: str):
        message = written("write_month_year", 200, value)

        assert message.get_str(200) == text
        assert message.get_month_year(200) == value

    @pytest.mark.parametrize(
        "value",
        [
            refix.MonthYear(2026, 13),
            refix.MonthYear(2026, 2, day=30),
            refix.MonthYear(2026, 12, week=6),
            refix.MonthYear(2026, 12, day=18, week=3),
            refix.MonthYear(70_000, 12),
        ],
    )
    def test_value_that_names_no_month_raises(self, value: refix.MonthYear):
        with pytest.raises(WriteError) as excinfo:
            written("write_month_year", 200, value)

        assert excinfo.value.tag == 200
        assert str(excinfo.value) == f"{value!r} cannot be written to field 200"

    def test_other_types_raise(self):
        message = "write_month_year takes a MonthYear, not str"
        with pytest.raises(TypeError, match=message):
            written("write_month_year", 200, "202612")


class TestWriteData:
    def test_writes_the_length_before_the_data(self):
        writer = refix.FieldWriter()
        writer.write_data(95, 96, b"a\x01b=c")

        message = writer.finish()

        assert message.bytes == b"95=5\x0196=a\x01b=c\x01"
        assert message.get_int(95) == 5
        assert message.get(96) == b"a\x01b=c"

    def test_empty_value_raises_and_writes_nothing(self):
        writer = refix.FieldWriter()

        with pytest.raises(WriteError) as excinfo:
            writer.write_data(95, 96, b"")

        assert excinfo.value.tag == 96
        assert writer.finish().bytes == b""


class TestWriteFields:
    def test_writes_the_fields_of_a_written_message_after_its_own(self):
        instrument = refix.FieldWriter()
        instrument.write_str(55, "ACME")
        instrument.write_str(48, "US0000000001")
        writer = refix.FieldWriter()
        writer.write_str(11, "ORDER-1")
        writer.write_fields(instrument.finish())
        writer.write_int(38, 100)

        message = writer.finish()

        assert str(message) == "11=ORDER-1|55=ACME|48=US0000000001|38=100"
        assert message.get_int(38) == 100

    def test_writes_fields_read_from_a_log_with_the_separator(self):
        (record,) = refix.read_log(io.BytesIO(b"8=FIX.4.4|9=5|35=0|10=163|\n"))
        writer = refix.FieldWriter()
        writer.write_fields(record.message)

        message = writer.finish()

        assert message.bytes == b"8=FIX.4.4\x019=5\x0135=0\x0110=163\x01"


def header(seq: int) -> refix.RawMessage:
    writer = refix.FieldWriter()
    writer.write_str(49, "BUYSIDE")
    writer.write_str(56, "VENUE")
    writer.write_int(34, seq)
    return writer.finish()


def body() -> refix.RawMessage:
    writer = refix.FieldWriter()
    writer.write_str(11, "ORDER-1")
    return writer.finish()


class TestFrame:
    def test_frames_a_header_and_a_body(self):
        message = refix.frame("FIX.4.4", "D", header(7), body())

        assert str(message) == (
            "8=FIX.4.4|9=41|35=D|49=BUYSIDE|56=VENUE|34=7|11=ORDER-1|10=041"
        )
        assert message.get_str(35) == "D"

    def test_a_framed_message_reads_back_through_the_tokenizer(self):
        writer = refix.FieldWriter()
        writer.write_data(95, 96, b"a\x01b=c")
        message = refix.frame("FIX.4.4", "B", header(7), writer.finish())

        read = refix.Tokenizer().tokenize(message.bytes)

        assert read.entries() == message.entries()
        assert read.get(96) == b"a\x01b=c"

    def test_frames_a_message_with_no_body_fields(self):
        message = refix.frame("FIX.4.4", "0", header(8), refix.FieldWriter().finish())

        assert str(message) == "8=FIX.4.4|9=30|35=0|49=BUYSIDE|56=VENUE|34=8|10=154"

    @pytest.mark.parametrize("tag", [8, 9, 35, 10])
    def test_field_that_framing_writes_raises(self, tag: int):
        writer = refix.FieldWriter()
        writer.write_str(tag, "1")
        fields = writer.finish()

        with pytest.raises(WriteError) as in_body:
            refix.frame("FIX.4.4", "D", header(7), fields)
        with pytest.raises(WriteError) as in_header:
            refix.frame("FIX.4.4", "D", fields, body())

        assert in_body.value.tag == in_header.value.tag == tag
        assert str(in_body.value) == f"field {tag} is written by framing"

    def test_empty_msg_type_raises(self):
        with pytest.raises(WriteError) as excinfo:
            refix.frame("FIX.4.4", "", header(7), body())

        assert excinfo.value.tag == 35


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
