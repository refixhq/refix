from typing import Any

import pytest
import refix
from refix import fields
from test_tokenizer import tokenize_body

PARTIES = refix.GroupTable(453, 448, (448, 452), ())
KNOWN = refix.KnownTags((35, 55, 448, 452, 453))


class NewOrderSingle(fields.Message):
    pass


class Instrument(fields.Component):
    pass


class Parties:
    class Instance(fields.GroupInstance):
        pass


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
