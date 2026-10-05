import enum
import pickle

import pytest
from refix.enums import Unrecognized, from_value


class PartyRole(enum.IntEnum):
    EXECUTING_FIRM = 1
    CLIENT_ID = 3


class OrdType(enum.StrEnum):
    MARKET = "1"
    LIMIT = "2"


class TestFromValue:
    def test_a_listed_value_is_its_member(self):
        assert from_value(PartyRole, 3) is PartyRole.CLIENT_ID

    def test_an_unlisted_value_is_unrecognized(self):
        assert from_value(PartyRole, 99) == Unrecognized(99)

    def test_str_enums_read_the_same_way(self):
        assert from_value(OrdType, "2") is OrdType.LIMIT
        assert from_value(OrdType, "X") == Unrecognized("X")


class TestUnrecognized:
    def test_equal_values_are_equal_and_hash_alike(self):
        assert Unrecognized(99) == Unrecognized(99)
        assert hash(Unrecognized(99)) == hash(Unrecognized(99))

    def test_displays_as_its_value(self):
        assert str(Unrecognized("X")) == "X"

    def test_matches_with_its_value(self):
        match from_value(PartyRole, 99):
            case Unrecognized(value):
                assert value == 99
            case _:
                pytest.fail("expected an unrecognized value")

    def test_pickles(self):
        assert pickle.loads(pickle.dumps(Unrecognized(99))) == Unrecognized(99)
