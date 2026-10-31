import pickle
from datetime import UTC, date, datetime, time
from decimal import Decimal

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


class TestGetBool:
    def test_reads_y_and_n(self):
        message = tokenize_body("35=D|43=Y|97=N|")

        assert message.get_bool(43) is True
        assert message.get_bool(97) is False

    def test_absent_tag_is_none(self):
        assert tokenize_body("35=D|").get_bool(43) is None

    @pytest.mark.parametrize("value", ["", "y", "1", "YES", "true"])
    def test_anything_but_y_or_n_raises(self, value: str):
        message = tokenize_body(f"35=D|43={value}|")

        with pytest.raises(InvalidValueError) as excinfo:
            message.get_bool(43)

        assert excinfo.value.tag == 43


class TestGetMultipleValues:
    def test_reads_values_in_wire_order_with_duplicates(self):
        message = tokenize_body("35=D|18=1 6 1|")
        assert message.get_multiple_values(18) == ("1", "6", "1")

    def test_reads_a_single_value(self):
        assert tokenize_body("35=D|18=G|").get_multiple_values(18) == ("G",)

    def test_absent_tag_is_none(self):
        assert tokenize_body("35=D|").get_multiple_values(18) is None

    @pytest.mark.parametrize("value", ["", " 1", "1 ", "1  6"])
    def test_anything_but_single_spaces_between_values_raises(self, value: str):
        message = tokenize_body(f"35=D|18={value}|")

        with pytest.raises(InvalidValueError) as excinfo:
            message.get_multiple_values(18)

        assert excinfo.value.tag == 18

    def test_invalid_utf8_raises(self):
        with pytest.raises(InvalidValueError):
            tokenize_bytes(b"35=D|18=1 \xe9|").get_multiple_values(18)


class TestInvalidValueError:
    def test_is_a_value_error(self):
        assert issubclass(InvalidValueError, ValueError)

    def test_names_the_field(self):
        assert str(InvalidValueError(38)) == "invalid value in field 38"

    def test_pickles(self):
        clone = pickle.loads(pickle.dumps(InvalidValueError(38)))
        assert clone.tag == 38


class TestGetDecimal:
    def test_reads_a_price(self):
        assert tokenize_body("35=D|44=101.25|").get_decimal(44) == Decimal("101.25")

    def test_keeps_the_scale(self):
        assert str(tokenize_body("35=D|44=1.50|").get_decimal(44)) == "1.50"

    def test_reads_a_negative_value(self):
        assert tokenize_body("35=D|44=-.5|").get_decimal(44) == Decimal("-0.5")

    def test_absent_tag_is_none(self):
        assert tokenize_body("35=D|").get_decimal(44) is None

    @pytest.mark.parametrize(
        "value", ["", "1e5", "NaN", "Infinity", " 1", "+5", "1_000"]
    )
    def test_text_only_python_accepts_raises(self, value: str):
        message = tokenize_body(f"35=D|44={value}|")

        with pytest.raises(InvalidValueError) as excinfo:
            message.get_decimal(44)

        assert excinfo.value.tag == 44


class TestGetDate:
    def test_reads_a_trade_date(self):
        assert tokenize_body("35=8|75=20261030|").get_date(75) == date(2026, 10, 30)

    def test_reads_the_last_date_fix_can_write(self):
        assert tokenize_body("35=8|75=99991231|").get_date(75) == date(9999, 12, 31)

    def test_absent_tag_is_none(self):
        assert tokenize_body("35=8|").get_date(75) is None

    def test_year_zero_raises_though_fix_allows_it(self):
        with pytest.raises(InvalidValueError) as excinfo:
            tokenize_body("35=8|75=00000101|").get_date(75)

        assert excinfo.value.tag == 75

    @pytest.mark.parametrize("value", ["", "2026-10-30", "20261301", "20230229"])
    def test_invalid_dates_raise(self, value: str):
        message = tokenize_body(f"35=8|75={value}|")

        with pytest.raises(InvalidValueError) as excinfo:
            message.get_date(75)

        assert excinfo.value.tag == 75


class TestGetMonthYear:
    def test_reads_a_month(self):
        message = tokenize_body("35=D|200=202612|")
        assert message.get_month_year(200) == refix.MonthYear(2026, 12)

    def test_reads_a_day(self):
        message = tokenize_body("35=D|200=20261218|")
        assert message.get_month_year(200) == refix.MonthYear(2026, 12, day=18)

    def test_reads_a_week(self):
        message = tokenize_body("35=D|200=202612w3|")
        assert message.get_month_year(200) == refix.MonthYear(2026, 12, week=3)

    def test_absent_tag_is_none(self):
        assert tokenize_body("35=D|").get_month_year(200) is None

    @pytest.mark.parametrize(
        "value", ["", "2026-12", "202613", "20260231", "202612w6", "202612W3"]
    )
    def test_invalid_values_raise(self, value: str):
        message = tokenize_body(f"35=D|200={value}|")

        with pytest.raises(InvalidValueError) as excinfo:
            message.get_month_year(200)

        assert excinfo.value.tag == 200


class TestGetTime:
    def test_reads_an_entry_time(self):
        message = tokenize_body("35=W|273=14:29:51.123|")
        assert message.get_time(273) == time(14, 29, 51, 123000)

    def test_truncates_to_microseconds(self):
        message = tokenize_body("35=W|273=14:29:51.123456789|")
        assert message.get_time(273) == time(14, 29, 51, 123456)

    def test_is_naive(self):
        value = tokenize_body("35=W|273=14:29:51|").get_time(273)

        assert value is not None
        assert value.tzinfo is None

    def test_clamps_a_leap_second_to_the_last_microsecond(self):
        message = tokenize_body("35=W|273=23:59:60.5|")
        assert message.get_time(273) == time(23, 59, 59, 999999)

    def test_absent_tag_is_none(self):
        assert tokenize_body("35=W|").get_time(273) is None

    @pytest.mark.parametrize("value", ["", "24:00:00", "14:29", "14:29:51Z"])
    def test_invalid_times_raise(self, value: str):
        message = tokenize_body(f"35=W|273={value}|")

        with pytest.raises(InvalidValueError) as excinfo:
            message.get_time(273)

        assert excinfo.value.tag == 273


class TestGetTimeNs:
    def test_reads_every_digit(self):
        message = tokenize_body("35=W|273=14:29:51.123456789|")
        assert message.get_time_ns(273) == 52_191_123_456_789

    def test_keeps_a_leap_second(self):
        message = tokenize_body("35=W|273=23:59:60.5|")
        assert message.get_time_ns(273) == 86_400_500_000_000

    def test_absent_tag_is_none(self):
        assert tokenize_body("35=W|").get_time_ns(273) is None

    def test_invalid_time_raises(self):
        with pytest.raises(InvalidValueError) as excinfo:
            tokenize_body("35=W|273=24:00:00|").get_time_ns(273)

        assert excinfo.value.tag == 273


class TestGetUtcTimestamp:
    def test_reads_a_sending_time(self):
        message = tokenize_body("35=D|52=20261030-14:29:51.123|")
        assert message.get_utc_timestamp(52) == datetime(
            2026, 10, 30, 14, 29, 51, 123000, tzinfo=UTC
        )

    def test_is_in_utc(self):
        value = tokenize_body("35=D|52=20261030-14:29:51|").get_utc_timestamp(52)

        assert value is not None
        assert value.tzinfo is UTC

    def test_truncates_to_microseconds(self):
        message = tokenize_body("35=D|52=20261030-14:29:51.123456789|")
        assert message.get_utc_timestamp(52) == datetime(
            2026, 10, 30, 14, 29, 51, 123456, tzinfo=UTC
        )

    def test_reads_a_time_before_the_epoch(self):
        message = tokenize_body("35=D|52=19691231-23:59:59.999|")
        assert message.get_utc_timestamp(52) == datetime(
            1969, 12, 31, 23, 59, 59, 999000, tzinfo=UTC
        )

    def test_folds_a_leap_second_into_the_next_day(self):
        message = tokenize_body("35=D|52=20161231-23:59:60|")
        assert message.get_utc_timestamp(52) == datetime(2017, 1, 1, tzinfo=UTC)

    def test_reads_the_ends_of_the_i64_range(self):
        last = tokenize_body("35=D|52=22620411-23:47:16.854775807|")
        first = tokenize_body("35=D|52=16770921-00:12:43.145224192|")

        assert last.get_utc_timestamp(52) == datetime(
            2262, 4, 11, 23, 47, 16, 854775, tzinfo=UTC
        )
        assert first.get_utc_timestamp(52) == datetime(
            1677, 9, 21, 0, 12, 43, 145224, tzinfo=UTC
        )

    def test_absent_tag_is_none(self):
        assert tokenize_body("35=D|").get_utc_timestamp(52) is None

    @pytest.mark.parametrize(
        "value", ["", "20261030", "2026-10-30T14:29:51Z", "20261030-24:00:00"]
    )
    def test_invalid_timestamps_raise(self, value: str):
        message = tokenize_body(f"35=D|52={value}|")

        with pytest.raises(InvalidValueError) as excinfo:
            message.get_utc_timestamp(52)

        assert excinfo.value.tag == 52


class TestGetUtcTimestampNs:
    def test_reads_every_digit(self):
        message = tokenize_body("35=D|52=20261030-14:29:51.123456789|")
        assert message.get_utc_timestamp_ns(52) == 1_793_370_591_123_456_789

    def test_reads_a_time_before_the_epoch(self):
        message = tokenize_body("35=D|52=19691231-23:59:59.999|")
        assert message.get_utc_timestamp_ns(52) == -1_000_000

    def test_absent_tag_is_none(self):
        assert tokenize_body("35=D|").get_utc_timestamp_ns(52) is None

    def test_invalid_timestamp_raises(self):
        with pytest.raises(InvalidValueError) as excinfo:
            tokenize_body("35=D|52=20261030|").get_utc_timestamp_ns(52)

        assert excinfo.value.tag == 52
