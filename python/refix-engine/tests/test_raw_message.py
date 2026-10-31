import pickle
from datetime import date
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
