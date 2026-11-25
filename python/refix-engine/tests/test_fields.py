import copy
import enum
from datetime import UTC, date, datetime, time
from decimal import Decimal
from typing import Any

import pytest
import refix
from refix import fields
from refix.enums import Unrecognized
from refix.errors import InvalidValueError
from test_raw_message import tokenize_bytes
from test_tokenizer import tokenize_body

PARTIES = refix.GroupTable(453, 448, (448, 452), ())
KNOWN = refix.KnownTags((35, 55, 448, 452, 453))


class Side(enum.StrEnum):
    BUY = "1"
    SELL = "2"


class PartyRole(enum.IntEnum):
    EXECUTING_FIRM = 1
    CLIENT_ID = 3


class QuoteCondition(enum.StrEnum):
    OPEN = "A"
    CLOSED = "B"


class PtysSubGrp(fields.GroupInstance):
    party_sub_id = fields.Str(523)


class Parties(fields.GroupInstance):
    party_id = fields.Str(448)
    party_role = fields.Int(452, PartyRole)
    party_sub_ids = fields.Group(802, PtysSubGrp)


class Header(fields.Component):
    class Hops(fields.GroupInstance):
        hop_comp_id = fields.Str(628)

    sender_comp_id = fields.Str(49)
    hops = fields.Group(627, Hops)


class Trailer(fields.Component):
    signature_length = fields.Int(93)


class UnderlyingInstrument(fields.Component):
    underlying_symbol = fields.Str(311)
    underlying_security_id = fields.Str(309)


class UndInstrmtGrp(fields.GroupInstance):
    underlying_instrument = fields.View(UnderlyingInstrument)


class NewOrderSingle(fields.Message):
    header = fields.View(Header)
    exec_inst = fields.MultipleValues(18)
    order_qty = fields.Int(38)
    poss_dup_flag = fields.Bool(43)
    price = fields.Decimal(44)
    sending_time = fields.UtcTimestamp(52)
    side = fields.Str(54, Side)
    sending_time_ns = fields.UtcTimestampNs(52)
    symbol = fields.Str(55)
    text = fields.Str(58)
    trade_date = fields.Date(75)
    raw_data = fields.Bytes(96)
    maturity_month_year = fields.MonthYear(200)
    md_entry_time = fields.Time(273)
    md_entry_time_ns = fields.TimeNs(273)
    quote_condition = fields.MultipleValues(276, QuoteCondition)
    party_role = fields.Int(452, PartyRole)
    party_ids = fields.Group(453, Parties)
    underlyings = fields.Group(711, UndInstrmtGrp)
    trailer = fields.View(Trailer)


class CustomOrder(NewOrderSingle):
    desk = fields.Str(9001)


class Instrument(fields.Component):
    symbol = fields.Str(55)


class Empty(fields.GroupInstance):
    pass


class EmptyHolder(fields.Message):
    empties = fields.Group(453, Empty)


class Plain:
    symbol = fields.Str(55)
    party_ids = fields.Group(453, Parties)
    header = fields.View(Header)


def order() -> refix.RawMessage:
    return tokenize_body("35=D|453=2|448=AL|452=1|448=BOB|55=ACME|")


def parties() -> tuple[refix.Scope, ...]:
    instances = order().get_group(PARTIES, KNOWN)
    assert instances is not None
    return instances


class TestMessage:
    def test_keeps_its_raw_message(self):
        raw = order()

        assert NewOrderSingle(raw).raw is raw

    def test_from_raw_returns_the_subclass(self):
        raw = order()

        message = NewOrderSingle.from_raw(raw)

        assert type(message) is NewOrderSingle
        assert message.raw is raw

    def test_the_repr_names_the_subclass(self):
        message = NewOrderSingle(tokenize_body("35=D|58=hello|"))

        assert repr(message) == "NewOrderSingle(8=FIX.4.4|9=14|35=D|58=hello|10=166)"

    def test_a_scope_raises(self):
        scope: Any = parties()[0]

        with pytest.raises(TypeError):
            NewOrderSingle(scope)

    def test_raw_by_keyword_raises(self):
        message: Any = NewOrderSingle

        with pytest.raises(TypeError):
            message(raw=order())

    def test_from_raw_by_keyword_raises(self):
        message: Any = NewOrderSingle

        with pytest.raises(TypeError):
            message.from_raw(raw=order())


class TestComponent:
    def test_reads_from_a_raw_message(self):
        raw = order()

        assert Instrument(raw).raw is raw

    def test_reads_from_a_scope(self):
        scope = parties()[0]

        assert Instrument(scope).raw is scope

    def test_from_raw_returns_the_subclass(self):
        assert type(Instrument.from_raw(order())) is Instrument

    def test_the_repr_shows_only_the_fields_it_declares(self):
        assert repr(Instrument(order())) == "Instrument(55=ACME)"

    def test_the_repr_shows_the_fields_of_its_groups(self):
        raw = tokenize_body("35=D|49=AL|627=1|628=HOP|55=ACME|")

        assert repr(Header(raw)) == "Header(49=AL|627=1|628=HOP)"

    def test_the_repr_of_a_view_shows_the_same_fields(self):
        raw = tokenize_body("35=D|49=AL|627=1|628=HOP|55=ACME|")

        assert repr(NewOrderSingle(raw).header) == "Header(49=AL|627=1|628=HOP)"

    def test_the_repr_stays_within_a_scope(self):
        assert repr(Instrument(parties()[1])) == "Instrument()"

    def test_any_other_object_raises(self):
        tag: Any = 55

        with pytest.raises(TypeError, match="'int' object is not a RawMessage"):
            Instrument(tag)

    def test_raw_by_keyword_raises(self):
        component: Any = Instrument

        with pytest.raises(TypeError):
            component(raw=order())


class TestGroupInstance:
    def test_keeps_its_scope(self):
        scope = parties()[0]

        assert Parties(scope).raw is scope

    def test_from_raw_returns_the_subclass(self):
        assert type(Parties.from_raw(parties()[0])) is Parties

    def test_the_repr_uses_the_qualified_name(self):
        raw = tokenize_body("35=D|627=1|628=HOP|55=ACME|")

        (hop,) = NewOrderSingle(raw).header.hops

        assert repr(hop) == "Header.Hops(628=HOP)"

    def test_a_raw_message_raises(self):
        raw: Any = order()

        with pytest.raises(TypeError):
            Parties(raw)

    def test_raw_by_keyword_raises(self):
        instance: Any = Parties

        with pytest.raises(TypeError):
            instance(raw=parties()[0])


class TestCopy:
    def test_a_copy_of_a_message_is_the_message(self):
        message = NewOrderSingle(order())

        assert copy.copy(message) is message

    def test_a_deep_copy_of_a_message_is_the_message(self):
        message = NewOrderSingle(order())

        assert copy.deepcopy(message) is message

    def test_a_copy_of_a_component_is_the_component(self):
        component = Instrument(order())

        assert copy.copy(component) is component
        assert copy.deepcopy(component) is component

    def test_a_copy_of_a_group_instance_is_the_instance(self):
        instance = Parties(parties()[0])

        assert copy.copy(instance) is instance
        assert copy.deepcopy(instance) is instance

    def test_a_deep_copy_of_a_list_keeps_its_messages(self):
        message = NewOrderSingle(order())

        (copied,) = copy.deepcopy([message])

        assert copied is message


class TestStr:
    def test_reads_text_from_a_message(self):
        assert NewOrderSingle(order()).symbol == "ACME"

    def test_reads_text_from_a_component(self):
        assert Instrument(order()).symbol == "ACME"

    def test_reads_text_from_a_group_instance(self):
        ids = [Parties(scope).party_id for scope in parties()]

        assert ids == ["AL", "BOB"]

    def test_absent_field_is_none(self):
        assert NewOrderSingle(order()).text is None

    def test_a_second_read_returns_the_same_object(self):
        message = NewOrderSingle(order())

        assert message.symbol is message.symbol

    def test_invalid_utf8_raises(self):
        message = NewOrderSingle(tokenize_bytes(b"35=D|58=caf\xe9|"))

        with pytest.raises(InvalidValueError) as excinfo:
            _ = message.text

        assert excinfo.value.tag == 58

    def test_class_access_returns_the_declaration(self):
        assert isinstance(NewOrderSingle.symbol, fields.Str)

    def test_the_repr_shows_the_tag(self):
        assert repr(NewOrderSingle.symbol) == "Str(55)"

    def test_assignment_raises(self):
        message: Any = NewOrderSingle(order())

        with pytest.raises(AttributeError, match="cannot change 'symbol'"):
            message.symbol = "ACME"

    def test_tag_by_keyword_raises(self):
        declaration: Any = fields.Str

        with pytest.raises(TypeError):
            declaration(tag=55)

    def test_a_class_without_a_base_raises(self):
        with pytest.raises(TypeError, match="'Plain' is not a Message"):
            _ = Plain().symbol


class TestInt:
    def test_reads_an_integer(self):
        message = NewOrderSingle(tokenize_body("35=D|38=200|"))

        assert message.order_qty == 200

    def test_absent_field_is_none(self):
        assert NewOrderSingle(order()).order_qty is None

    def test_a_non_integer_raises(self):
        message = NewOrderSingle(tokenize_body("35=D|38=1.5|"))

        with pytest.raises(InvalidValueError) as excinfo:
            _ = message.order_qty

        assert excinfo.value.tag == 38

    def test_the_repr_shows_the_tag(self):
        assert repr(NewOrderSingle.order_qty) == "Int(38)"

    def test_tag_by_keyword_raises(self):
        declaration: Any = fields.Int

        with pytest.raises(TypeError):
            declaration(tag=38)


class TestDecimal:
    def test_reads_a_decimal(self):
        message = NewOrderSingle(tokenize_body("35=D|44=101.25|"))

        assert message.price == Decimal("101.25")

    def test_keeps_the_scale(self):
        message = NewOrderSingle(tokenize_body("35=D|44=1.50|"))

        assert str(message.price) == "1.50"

    def test_absent_field_is_none(self):
        assert NewOrderSingle(order()).price is None

    def test_a_non_decimal_raises(self):
        message = NewOrderSingle(tokenize_body("35=D|44=1e5|"))

        with pytest.raises(InvalidValueError) as excinfo:
            _ = message.price

        assert excinfo.value.tag == 44

    def test_the_repr_shows_the_tag(self):
        assert repr(NewOrderSingle.price) == "Decimal(44)"

    def test_tag_by_keyword_raises(self):
        declaration: Any = fields.Decimal

        with pytest.raises(TypeError):
            declaration(tag=44)


class TestBool:
    def test_reads_y_as_true(self):
        message = NewOrderSingle(tokenize_body("35=D|43=Y|"))

        assert message.poss_dup_flag is True

    def test_reads_n_as_false(self):
        message = NewOrderSingle(tokenize_body("35=D|43=N|"))

        assert message.poss_dup_flag is False

    def test_absent_field_is_none(self):
        assert NewOrderSingle(order()).poss_dup_flag is None

    def test_anything_but_y_or_n_raises(self):
        message = NewOrderSingle(tokenize_body("35=D|43=true|"))

        with pytest.raises(InvalidValueError) as excinfo:
            _ = message.poss_dup_flag

        assert excinfo.value.tag == 43

    def test_the_repr_shows_the_tag(self):
        assert repr(NewOrderSingle.poss_dup_flag) == "Bool(43)"

    def test_tag_by_keyword_raises(self):
        declaration: Any = fields.Bool

        with pytest.raises(TypeError):
            declaration(tag=43)


class TestBytes:
    def test_reads_bytes(self):
        message = NewOrderSingle(tokenize_body("35=D|96=abc|"))

        assert message.raw_data == b"abc"

    def test_reads_bytes_that_are_not_text(self):
        message = NewOrderSingle(tokenize_bytes(b"35=D|96=\xff\xfe|"))

        assert message.raw_data == b"\xff\xfe"

    def test_absent_field_is_none(self):
        assert NewOrderSingle(order()).raw_data is None

    def test_the_repr_shows_the_tag(self):
        assert repr(NewOrderSingle.raw_data) == "Bytes(96)"

    def test_tag_by_keyword_raises(self):
        declaration: Any = fields.Bytes

        with pytest.raises(TypeError):
            declaration(tag=96)


class TestDate:
    def test_reads_a_date(self):
        message = NewOrderSingle(tokenize_body("35=D|75=20261030|"))

        assert message.trade_date == date(2026, 10, 30)

    def test_absent_field_is_none(self):
        assert NewOrderSingle(order()).trade_date is None

    def test_an_invalid_date_raises(self):
        message = NewOrderSingle(tokenize_body("35=D|75=2026-10-30|"))

        with pytest.raises(InvalidValueError) as excinfo:
            _ = message.trade_date

        assert excinfo.value.tag == 75

    def test_the_repr_shows_the_tag(self):
        assert repr(NewOrderSingle.trade_date) == "Date(75)"

    def test_tag_by_keyword_raises(self):
        declaration: Any = fields.Date

        with pytest.raises(TypeError):
            declaration(tag=75)


class TestTime:
    def test_reads_a_time(self):
        message = NewOrderSingle(tokenize_body("35=D|273=14:29:51.123|"))

        assert message.md_entry_time == time(14, 29, 51, 123000)

    def test_truncates_to_microseconds(self):
        message = NewOrderSingle(tokenize_body("35=D|273=14:29:51.123456789|"))

        assert message.md_entry_time == time(14, 29, 51, 123456)

    def test_absent_field_is_none(self):
        assert NewOrderSingle(order()).md_entry_time is None

    def test_an_invalid_time_raises(self):
        message = NewOrderSingle(tokenize_body("35=D|273=24:00:00|"))

        with pytest.raises(InvalidValueError) as excinfo:
            _ = message.md_entry_time

        assert excinfo.value.tag == 273

    def test_the_repr_shows_the_tag(self):
        assert repr(NewOrderSingle.md_entry_time) == "Time(273)"

    def test_tag_by_keyword_raises(self):
        declaration: Any = fields.Time

        with pytest.raises(TypeError):
            declaration(tag=273)


class TestTimeNs:
    def test_reads_every_digit(self):
        message = NewOrderSingle(tokenize_body("35=D|273=14:29:51.123456789|"))

        assert message.md_entry_time_ns == 52_191_123_456_789

    def test_absent_field_is_none(self):
        assert NewOrderSingle(order()).md_entry_time_ns is None

    def test_an_invalid_time_raises(self):
        message = NewOrderSingle(tokenize_body("35=D|273=24:00:00|"))

        with pytest.raises(InvalidValueError) as excinfo:
            _ = message.md_entry_time_ns

        assert excinfo.value.tag == 273

    def test_the_repr_shows_the_tag(self):
        assert repr(NewOrderSingle.md_entry_time_ns) == "TimeNs(273)"

    def test_tag_by_keyword_raises(self):
        declaration: Any = fields.TimeNs

        with pytest.raises(TypeError):
            declaration(tag=273)


class TestUtcTimestamp:
    def test_reads_a_timestamp(self):
        message = NewOrderSingle(tokenize_body("35=D|52=20261030-14:29:51.123|"))

        assert message.sending_time == datetime(
            2026, 10, 30, 14, 29, 51, 123000, tzinfo=UTC
        )

    def test_is_in_utc(self):
        message = NewOrderSingle(tokenize_body("35=D|52=20261030-14:29:51|"))

        assert message.sending_time is not None
        assert message.sending_time.tzinfo is UTC

    def test_truncates_to_microseconds(self):
        message = NewOrderSingle(tokenize_body("35=D|52=20261030-14:29:51.123456789|"))

        assert message.sending_time == datetime(
            2026, 10, 30, 14, 29, 51, 123456, tzinfo=UTC
        )

    def test_absent_field_is_none(self):
        assert NewOrderSingle(order()).sending_time is None

    def test_an_invalid_timestamp_raises(self):
        message = NewOrderSingle(tokenize_body("35=D|52=20261030|"))

        with pytest.raises(InvalidValueError) as excinfo:
            _ = message.sending_time

        assert excinfo.value.tag == 52

    def test_the_repr_shows_the_tag(self):
        assert repr(NewOrderSingle.sending_time) == "UtcTimestamp(52)"

    def test_tag_by_keyword_raises(self):
        declaration: Any = fields.UtcTimestamp

        with pytest.raises(TypeError):
            declaration(tag=52)


class TestUtcTimestampNs:
    def test_reads_every_digit(self):
        message = NewOrderSingle(tokenize_body("35=D|52=20261030-14:29:51.123456789|"))

        assert message.sending_time_ns == 1_793_370_591_123_456_789

    def test_absent_field_is_none(self):
        assert NewOrderSingle(order()).sending_time_ns is None

    def test_an_invalid_timestamp_raises(self):
        message = NewOrderSingle(tokenize_body("35=D|52=20261030|"))

        with pytest.raises(InvalidValueError) as excinfo:
            _ = message.sending_time_ns

        assert excinfo.value.tag == 52

    def test_the_repr_shows_the_tag(self):
        assert repr(NewOrderSingle.sending_time_ns) == "UtcTimestampNs(52)"

    def test_tag_by_keyword_raises(self):
        declaration: Any = fields.UtcTimestampNs

        with pytest.raises(TypeError):
            declaration(tag=52)


class TestMonthYear:
    def test_reads_a_month(self):
        message = NewOrderSingle(tokenize_body("35=D|200=202612|"))

        assert message.maturity_month_year == refix.MonthYear(2026, 12)

    def test_reads_a_day(self):
        message = NewOrderSingle(tokenize_body("35=D|200=20261218|"))

        assert message.maturity_month_year == refix.MonthYear(2026, 12, day=18)

    def test_absent_field_is_none(self):
        assert NewOrderSingle(order()).maturity_month_year is None

    def test_an_invalid_value_raises(self):
        message = NewOrderSingle(tokenize_body("35=D|200=2026-12|"))

        with pytest.raises(InvalidValueError) as excinfo:
            _ = message.maturity_month_year

        assert excinfo.value.tag == 200

    def test_the_repr_shows_the_tag(self):
        assert repr(NewOrderSingle.maturity_month_year) == "MonthYear(200)"

    def test_tag_by_keyword_raises(self):
        declaration: Any = fields.MonthYear

        with pytest.raises(TypeError):
            declaration(tag=200)


class TestMultipleValues:
    def test_reads_values_in_wire_order(self):
        message = NewOrderSingle(tokenize_body("35=D|18=1 6 1|"))

        assert message.exec_inst == ("1", "6", "1")

    def test_reads_a_single_value(self):
        message = NewOrderSingle(tokenize_body("35=D|18=G|"))

        assert message.exec_inst == ("G",)

    def test_absent_field_is_none(self):
        assert NewOrderSingle(order()).exec_inst is None

    def test_a_double_space_raises(self):
        message = NewOrderSingle(tokenize_body("35=D|18=1  6|"))

        with pytest.raises(InvalidValueError) as excinfo:
            _ = message.exec_inst

        assert excinfo.value.tag == 18

    def test_the_repr_shows_the_tag(self):
        assert repr(NewOrderSingle.exec_inst) == "MultipleValues(18)"

    def test_tag_by_keyword_raises(self):
        declaration: Any = fields.MultipleValues

        with pytest.raises(TypeError):
            declaration(tag=18)


class TestCodedStr:
    def test_reads_a_member(self):
        message = NewOrderSingle(tokenize_body("35=D|54=1|"))

        assert message.side is Side.BUY

    def test_an_unlisted_value_is_unrecognized(self):
        message = NewOrderSingle(tokenize_body("35=D|54=X|"))

        assert message.side == Unrecognized("X")

    def test_absent_field_is_none(self):
        assert NewOrderSingle(order()).side is None

    def test_the_missing_hook_of_the_enum_is_not_used(self):
        class Lenient(enum.StrEnum):
            BUY = "B"

            @classmethod
            def _missing_(cls, value: object) -> "Lenient":
                return cls.BUY

        class Order(fields.Message):
            side = fields.Str(54, Lenient)

        message = Order(tokenize_body("35=D|54=b|"))

        assert message.side == Unrecognized("b")

    def test_the_repr_shows_the_enum(self):
        assert repr(NewOrderSingle.side) == "Str(54, Side)"

    def test_an_int_enum_raises(self):
        declaration: Any = fields.Str

        with pytest.raises(TypeError, match="'PartyRole' is not a subclass of StrEnum"):
            declaration(54, PartyRole)

    def test_a_plain_enum_raises(self):
        class Plain(enum.Enum):
            BUY = "1"

        declaration: Any = fields.Str

        with pytest.raises(TypeError, match="is not a subclass of StrEnum"):
            declaration(54, Plain)

    def test_anything_but_a_class_raises(self):
        declaration: Any = fields.Str

        with pytest.raises(TypeError):
            declaration(54, "Side")

    def test_the_enum_by_keyword_raises(self):
        declaration: Any = fields.Str

        with pytest.raises(TypeError):
            declaration(54, enum=Side)


class TestCodedInt:
    def test_reads_a_member(self):
        message = NewOrderSingle(tokenize_body("35=D|452=3|"))

        assert message.party_role is PartyRole.CLIENT_ID

    def test_an_unlisted_value_is_unrecognized(self):
        message = NewOrderSingle(tokenize_body("35=D|452=99|"))

        assert message.party_role == Unrecognized(99)

    def test_absent_field_is_none(self):
        assert NewOrderSingle(tokenize_body("35=D|")).party_role is None

    def test_a_non_integer_raises(self):
        message = NewOrderSingle(tokenize_body("35=D|452=A|"))

        with pytest.raises(InvalidValueError) as excinfo:
            _ = message.party_role

        assert excinfo.value.tag == 452

    def test_the_repr_shows_the_enum(self):
        assert repr(NewOrderSingle.party_role) == "Int(452, PartyRole)"

    def test_a_str_enum_raises(self):
        declaration: Any = fields.Int

        with pytest.raises(TypeError, match="'Side' is not a subclass of IntEnum"):
            declaration(452, Side)

    def test_the_enum_by_keyword_raises(self):
        declaration: Any = fields.Int

        with pytest.raises(TypeError):
            declaration(452, enum=PartyRole)


class TestCodedMultipleValues:
    def test_reads_members_in_wire_order(self):
        message = NewOrderSingle(tokenize_body("35=D|276=B A B|"))

        assert message.quote_condition == (
            QuoteCondition.CLOSED,
            QuoteCondition.OPEN,
            QuoteCondition.CLOSED,
        )

    def test_an_unlisted_value_is_unrecognized(self):
        message = NewOrderSingle(tokenize_body("35=D|276=A Z|"))

        assert message.quote_condition == (QuoteCondition.OPEN, Unrecognized("Z"))

    def test_absent_field_is_none(self):
        assert NewOrderSingle(order()).quote_condition is None

    def test_a_double_space_raises(self):
        message = NewOrderSingle(tokenize_body("35=D|276=A  B|"))

        with pytest.raises(InvalidValueError) as excinfo:
            _ = message.quote_condition

        assert excinfo.value.tag == 276

    def test_the_repr_shows_the_enum(self):
        expected = "MultipleValues(276, QuoteCondition)"

        assert repr(NewOrderSingle.quote_condition) == expected

    def test_an_int_enum_raises(self):
        declaration: Any = fields.MultipleValues

        with pytest.raises(TypeError, match="'PartyRole' is not a subclass of StrEnum"):
            declaration(276, PartyRole)

    def test_the_enum_by_keyword_raises(self):
        declaration: Any = fields.MultipleValues

        with pytest.raises(TypeError):
            declaration(276, enum=QuoteCondition)


class TestGroup:
    def test_reads_each_instance(self):
        parties = NewOrderSingle(order()).party_ids

        assert [type(party) for party in parties] == [Parties] * 2
        assert [party.party_id for party in parties] == ["AL", "BOB"]

    def test_absent_group_is_empty(self):
        assert NewOrderSingle(tokenize_body("35=D|55=ACME|")).party_ids == ()

    def test_a_count_of_zero_is_empty(self):
        assert NewOrderSingle(tokenize_body("35=D|453=0|55=ACME|")).party_ids == ()

    def test_reads_a_group_in_a_group(self):
        raw = tokenize_body(
            "35=D|453=2|448=AL|802=2|523=desk|523=floor|448=BOB|55=ACME|"
        )

        first, second = NewOrderSingle(raw).party_ids

        assert [sub.party_sub_id for sub in first.party_sub_ids] == ["desk", "floor"]
        assert second.party_sub_ids == ()

    def test_ends_at_a_field_of_the_message(self):
        raw = tokenize_body("35=D|453=1|448=AL|55=ACME|")

        (party,) = NewOrderSingle(raw).party_ids

        assert str(party.raw) == "448=AL"

    def test_an_undeclared_tag_stays_in_the_instance(self):
        raw = tokenize_body("35=D|453=1|448=AL|9001=x|55=ACME|")

        (party,) = NewOrderSingle(raw).party_ids

        assert str(party.raw) == "448=AL|9001=x"

    def test_ends_at_a_field_a_subclass_declares(self):
        raw = tokenize_body("35=D|453=1|448=AL|9001=x|55=ACME|")

        (party,) = CustomOrder(raw).party_ids

        assert str(party.raw) == "448=AL"

    def test_a_wrong_count_raises(self):
        message = NewOrderSingle(tokenize_body("35=D|453=3|448=AL|448=BOB|55=ACME|"))

        with pytest.raises(InvalidValueError) as excinfo:
            _ = message.party_ids

        assert excinfo.value.tag == 453

    def test_a_second_read_returns_the_same_object(self):
        message = NewOrderSingle(order())

        assert message.party_ids is message.party_ids

    def test_class_access_returns_the_declaration(self):
        assert isinstance(NewOrderSingle.party_ids, fields.Group)

    def test_the_repr_shows_the_tag_and_the_instance_class(self):
        assert repr(NewOrderSingle.party_ids) == "Group(453, Parties)"

    def test_a_class_that_is_not_a_group_instance_raises(self):
        declaration: Any = fields.Group

        with pytest.raises(
            TypeError, match="'Instrument' is not a subclass of GroupInstance"
        ):
            declaration(453, Instrument)

    def test_an_instance_class_without_fields_raises(self):
        message = EmptyHolder(order())

        with pytest.raises(TypeError, match="'Empty' declares no fields"):
            _ = message.empties

    def test_the_instance_class_by_keyword_raises(self):
        declaration: Any = fields.Group

        with pytest.raises(TypeError):
            declaration(453, instance=Parties)

    def test_a_class_without_a_base_raises(self):
        with pytest.raises(TypeError, match="'Plain' is not a Message"):
            _ = Plain().party_ids


class TestView:
    def test_reads_a_field_of_its_owner(self):
        message = NewOrderSingle(tokenize_body("35=D|49=AL|55=ACME|"))

        assert message.header.sender_comp_id == "AL"

    def test_is_an_object_of_the_component_class(self):
        assert type(NewOrderSingle(order()).header) is Header

    def test_reads_from_what_its_owner_reads_from(self):
        message = NewOrderSingle(order())

        assert message.header.raw is message.raw

    def test_a_second_read_returns_the_same_object(self):
        message = NewOrderSingle(order())

        assert message.header is message.header

    def test_a_group_in_a_view_ends_at_a_field_of_the_owner(self):
        raw = tokenize_body("35=D|627=1|628=HOP|55=ACME|")

        (hop,) = NewOrderSingle(raw).header.hops

        assert str(hop.raw) == "628=HOP"

    def test_a_group_in_a_view_of_a_view_ends_at_a_field_of_the_owner(self):
        class Routing(fields.Component):
            header = fields.View(Header)

        class Routed(fields.Message):
            routing = fields.View(Routing)
            symbol = fields.Str(55)

        raw = tokenize_body("35=D|627=1|628=HOP|55=ACME|")

        (hop,) = Routed(raw).routing.header.hops

        assert str(hop.raw) == "628=HOP"

    def test_a_group_in_a_component_made_directly_ends_at_its_own_fields(self):
        raw = tokenize_body("35=D|627=1|628=HOP|55=ACME|49=AL|")

        (hop,) = Header(raw).hops

        assert str(hop.raw) == "628=HOP|55=ACME"

    def test_a_group_ends_at_a_field_of_a_view(self):
        raw = tokenize_body("35=D|453=1|448=AL|93=3|89=abc|")

        (party,) = NewOrderSingle(raw).party_ids

        assert str(party.raw) == "448=AL"

    def test_a_group_instance_starts_at_the_first_field_of_its_view(self):
        raw = tokenize_body("35=D|711=2|311=ACME|309=XS1|311=INIT|55=ACME|")

        underlyings = NewOrderSingle(raw).underlyings

        symbols = [
            underlying.underlying_instrument.underlying_symbol
            for underlying in underlyings
        ]
        assert symbols == ["ACME", "INIT"]
        assert [str(underlying.raw) for underlying in underlyings] == [
            "311=ACME|309=XS1",
            "311=INIT",
        ]

    def test_class_access_returns_the_declaration(self):
        assert isinstance(NewOrderSingle.header, fields.View)

    def test_the_repr_shows_the_component_class(self):
        assert repr(NewOrderSingle.header) == "View(Header)"

    def test_a_class_that_is_not_a_component_raises(self):
        declaration: Any = fields.View

        with pytest.raises(TypeError, match="'Empty' is not a subclass of Component"):
            declaration(Empty)

    def test_the_component_class_by_keyword_raises(self):
        declaration: Any = fields.View

        with pytest.raises(TypeError):
            declaration(component=Header)

    def test_a_class_without_a_base_raises(self):
        with pytest.raises(TypeError, match="'Plain' is not a Message"):
            _ = Plain().header
