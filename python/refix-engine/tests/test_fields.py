from typing import Any

import pytest
import refix
from refix import fields
from refix.errors import InvalidValueError
from test_raw_message import tokenize_bytes
from test_tokenizer import tokenize_body

PARTIES = refix.GroupTable(453, 448, (448, 452), ())
KNOWN = refix.KnownTags((35, 55, 448, 452, 453))


class NewOrderSingle(fields.Message):
    symbol = fields.Str(55)
    text = fields.Str(58)


class Instrument(fields.Component):
    symbol = fields.Str(55)


class Parties:
    class Instance(fields.GroupInstance):
        party_id = fields.Str(448)


class Plain:
    symbol = fields.Str(55)


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

    def test_the_repr_shows_the_scope(self):
        assert repr(Instrument(parties()[1])) == "Instrument(448=BOB)"

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

        assert Parties.Instance(scope).raw is scope

    def test_from_raw_returns_the_subclass(self):
        assert type(Parties.Instance.from_raw(parties()[0])) is Parties.Instance

    def test_the_repr_uses_the_qualified_name(self):
        instance = Parties.Instance(parties()[0])

        assert repr(instance) == "Parties.Instance(448=AL|452=1)"

    def test_a_raw_message_raises(self):
        raw: Any = order()

        with pytest.raises(TypeError):
            Parties.Instance(raw)

    def test_raw_by_keyword_raises(self):
        instance: Any = Parties.Instance

        with pytest.raises(TypeError):
            instance(raw=parties()[0])


class TestStr:
    def test_reads_text_from_a_message(self):
        assert NewOrderSingle(order()).symbol == "ACME"

    def test_reads_text_from_a_component(self):
        assert Instrument(order()).symbol == "ACME"

    def test_reads_text_from_a_group_instance(self):
        ids = [Parties.Instance(scope).party_id for scope in parties()]

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
