from typing import assert_never

import pytest
from refix.enums import Unrecognized
from refix.errors import InvalidValueError
from test_tokenizer import tokenize_body
from toy_generated import ExecInst, Header, Logon, NewOrderSingle, OrdType, PartyRole


def order(body: str) -> NewOrderSingle:
    return NewOrderSingle(tokenize_body(body))


class TestReads:
    def test_typed_reads_over_a_tokenized_frame(self):
        new_order = order("35=D|11=ORDER-1|38=200|44=101.5|40=1|")

        assert NewOrderSingle.MSG_TYPE == b"D"
        assert new_order.raw.get(35) == NewOrderSingle.MSG_TYPE
        assert new_order.cl_ord_id == "ORDER-1"
        assert new_order.order_qty == 200
        assert new_order.price_raw == b"101.5"
        assert new_order.ord_type is OrdType.MARKET

    def test_an_absent_field_reads_as_none(self):
        new_order = order("35=D|11=ORDER-1|")

        assert new_order.order_qty is None
        assert new_order.price_raw is None
        assert new_order.ord_type is None
        assert new_order.exec_inst is None

    def test_a_malformed_value_raises(self):
        with pytest.raises(InvalidValueError) as excinfo:
            _ = order("35=D|38=12x3|").order_qty

        assert excinfo.value.tag == 38

    def test_a_read_is_cached(self):
        new_order = order("35=D|11=ORDER-1|")

        assert new_order.cl_ord_id is new_order.cl_ord_id


class TestEnums:
    def test_an_unrecognized_value_is_representable(self):
        assert order("35=D|40=X|").ord_type == Unrecognized("X")

    def test_a_match_handles_every_value(self):
        def describe(ord_type: OrdType | Unrecognized[str]) -> str:
            match ord_type:
                case OrdType.MARKET:
                    return "market"
                case OrdType.LIMIT:
                    return "limit"
                case Unrecognized(value):
                    return f"unrecognized {value}"
                case _:
                    assert_never(ord_type)

        assert describe(OrdType.LIMIT) == "limit"
        assert describe(Unrecognized("X")) == "unrecognized X"

    def test_a_multiple_value_field_reads_every_value(self):
        assert order("35=D|18=1 6 Z|").exec_inst == (
            ExecInst.NOT_HELD,
            ExecInst.PARTICIPATE_DONT_INITIATE,
            Unrecognized("Z"),
        )

    def test_badly_spaced_multiple_values_raise(self):
        with pytest.raises(InvalidValueError) as excinfo:
            _ = order("35=D|18=1  6|").exec_inst

        assert excinfo.value.tag == 18


class TestGroups:
    def test_typed_reads_over_groups(self):
        new_order = order(
            "35=D|11=ORDER-1|453=2|448=AL|452=1|802=2|523=S1|523=S2|448=BOB|452=03|38=200|"
        )

        parties = new_order.parties
        assert [party.party_id for party in parties] == ["AL", "BOB"]
        assert [party.party_role for party in parties] == [
            PartyRole.EXECUTING_FIRM,
            PartyRole.CLIENT_ID,
        ]
        assert [sub_id.party_sub_id for sub_id in parties[0].ptys_sub_grp] == [
            "S1",
            "S2",
        ]
        assert parties[1].ptys_sub_grp == ()
        assert new_order.order_qty == 200

    def test_an_absent_group_reads_as_empty(self):
        assert order("35=D|11=ORDER-1|").parties == ()

    def test_a_malformed_group_raises(self):
        with pytest.raises(InvalidValueError) as excinfo:
            _ = order("35=D|453=2|448=AL|38=200|").parties

        assert excinfo.value.tag == 453

    def test_an_unrecognized_int_code_is_representable(self):
        party = order("35=D|453=1|448=AL|452=99|").parties[0]

        assert party.party_role == Unrecognized(99)

    def test_a_non_integer_int_code_raises(self):
        party = order("35=D|453=1|448=AL|452=X|").parties[0]

        with pytest.raises(InvalidValueError) as excinfo:
            _ = party.party_role

        assert excinfo.value.tag == 452

    def test_reads_a_group_declared_in_its_message(self):
        logon = Logon(tokenize_body("35=A|384=2|372=D|372=8|"))

        assert [msg_type.ref_msg_type for msg_type in logon.msg_types] == ["D", "8"]

    def test_a_header_field_after_a_group_ends_it(self):
        logon = Logon(tokenize_body("35=A|384=1|372=D|49=SENDER|"))

        assert logon.msg_types[0].raw.get(49) is None

    def test_a_group_read_is_cached(self):
        new_order = order("35=D|453=1|448=AL|")

        assert new_order.parties is new_order.parties


class TestHeader:
    def test_reads_the_header_and_trailer(self):
        new_order = order("35=D|49=SENDER|56=TARGET|34=7|11=ORDER-1|")

        assert new_order.header.begin_string == "FIX.4.4"
        assert new_order.header.msg_seq_num == 7
        assert new_order.header.msg_type == "D"
        assert new_order.header.sender_comp_id == "SENDER"
        assert new_order.header.target_comp_id == "TARGET"
        assert new_order.trailer.check_sum is not None

    def test_reads_a_header_without_the_message_type(self):
        header = Header(tokenize_body("35=D|49=SENDER|11=ORDER-1|"))

        assert header.sender_comp_id == "SENDER"

    def test_a_message_header_ends_its_groups_where_the_body_starts(self):
        logon = Logon(tokenize_body("35=A|627=2|628=HOP-1|628=HOP-2|384=1|372=D|"))

        hops = logon.header.hops
        assert [hop.hop_comp_id for hop in hops] == ["HOP-1", "HOP-2"]
        assert hops[1].raw.get(384) is None

    def test_a_header_from_raw_keeps_body_fields_in_its_last_group(self):
        header = Header(tokenize_body("35=A|627=2|628=HOP-1|628=HOP-2|384=1|372=D|"))

        assert header.hops[1].raw.get(384) == b"1"
