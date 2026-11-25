import runpy
from datetime import UTC, date, datetime, time
from decimal import Decimal
from io import BytesIO
from pathlib import Path
from typing import assert_never

import pytest
import refix
from refix import MonthYear, RawMessage, decode_log
from refix.enums import Unrecognized
from refix.errors import InvalidValueError
from test_tokenizer import construct_valid_frame, tokenize_body
from toy_generated import (
    CODEC,
    ExecInst,
    Header,
    Instrument,
    Logon,
    MassQuote,
    Message,
    NewOrderSingle,
    OrdType,
    Parties,
    PartyRole,
    from_raw,
)

TOY = str(Path(__file__).with_name("toy_generated.py"))


def order(body: str) -> NewOrderSingle:
    return NewOrderSingle.from_raw(tokenize_body(body))


class TestRuntimeVersion:
    @pytest.mark.parametrize("version", ["0.9.0", "0.1.0"])
    def test_an_older_runtime_is_refused(
        self, monkeypatch: pytest.MonkeyPatch, version: str
    ):
        monkeypatch.setattr(refix, "__version__", version)

        with pytest.raises(
            ImportError,
            match=f"toy needs refix 0.10 or newer, but refix {version} is installed",
        ):
            runpy.run_path(TOY, run_name="toy")

    @pytest.mark.parametrize("version", ["0.10.0", "0.11.2", "1.0.0"])
    def test_the_minimum_and_newer_runtimes_are_accepted(
        self, monkeypatch: pytest.MonkeyPatch, version: str
    ):
        monkeypatch.setattr(refix, "__version__", version)

        module = runpy.run_path(TOY, run_name="toy")

        assert "NewOrderSingle" in module


class TestReads:
    def test_typed_reads_over_a_tokenized_frame(self):
        new_order = order(
            "35=D|11=ORDER-1|38=200|44=101.5|64=20261103|200=202612|114=Y|"
            "273=14:29:51.123456789|40=1|"
        )

        assert NewOrderSingle.MSG_TYPE == b"D"
        assert new_order.raw.get(35) == NewOrderSingle.MSG_TYPE
        assert new_order.cl_ord_id == "ORDER-1"
        assert new_order.order_qty_data.order_qty == 200
        assert new_order.price == Decimal("101.5")
        assert new_order.settl_date == date(2026, 11, 3)
        assert new_order.maturity_month_year == MonthYear(2026, 12)
        assert new_order.locate_reqd is True
        assert new_order.md_entry_time == time(14, 29, 51, 123456)
        assert new_order.md_entry_time_ns == 52_191_123_456_789
        assert new_order.ord_type is OrdType.MARKET

    def test_an_absent_field_reads_as_none(self):
        new_order = order("35=D|11=ORDER-1|")

        assert new_order.order_qty_data.order_qty is None
        assert new_order.price is None
        assert new_order.settl_date is None
        assert new_order.maturity_month_year is None
        assert new_order.locate_reqd is None
        assert new_order.md_entry_time is None
        assert new_order.md_entry_time_ns is None
        assert new_order.ord_type is None
        assert new_order.exec_inst is None

    def test_a_malformed_value_raises(self):
        with pytest.raises(InvalidValueError) as excinfo:
            _ = order("35=D|38=12x3|").order_qty_data.order_qty

        assert excinfo.value.tag == 38

    def test_a_read_is_cached(self):
        new_order = order("35=D|11=ORDER-1|")

        assert new_order.cl_ord_id is new_order.cl_ord_id


class TestDispatch:
    def test_reads_each_message_as_its_msg_type_names(self):
        new_order = from_raw(tokenize_body("35=D|11=ORDER-1|"))

        assert isinstance(new_order, NewOrderSingle)
        assert new_order.cl_ord_id == "ORDER-1"
        assert isinstance(from_raw(tokenize_body("35=A|")), Logon)

    def test_an_unknown_msg_type_reads_as_the_raw_message(self):
        raw = tokenize_body("35=ZZ|58=hello|")

        assert from_raw(raw) is raw

    def test_a_match_handles_every_message(self):
        def describe(message: Message | RawMessage) -> str:
            match message:
                case NewOrderSingle():
                    return "order"
                case Logon():
                    return "logon"
                case MassQuote():
                    return "quote"
                case RawMessage():
                    return "unrecognized"
                case _:
                    assert_never(message)

        assert describe(from_raw(tokenize_body("35=D|"))) == "order"
        assert describe(from_raw(tokenize_body("35=A|"))) == "logon"
        assert describe(from_raw(tokenize_body("35=ZZ|"))) == "unrecognized"

    def test_decodes_a_log_through_the_codec(self):
        order = construct_valid_frame("FIX.4.4", "35=D|11=ORDER-1|")
        unknown = construct_valid_frame("FIX.4.4", "35=ZZ|")

        records = list(decode_log(BytesIO(b"junk\n" + order + unknown), CODEC))

        assert [record.prefix for record in records] == [b"junk\n", b""]
        assert isinstance(records[0].message, NewOrderSingle)
        assert records[0].message.cl_ord_id == "ORDER-1"
        assert isinstance(records[1].message, RawMessage)


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

    def test_a_multiple_value_field_without_listed_values_reads_text(self):
        assert order("35=D|1035=ADD AON|").desk_order_handling_inst == ("ADD", "AON")

    def test_badly_spaced_multiple_values_raise(self):
        with pytest.raises(InvalidValueError) as excinfo:
            _ = order("35=D|18=1  6|").exec_inst

        assert excinfo.value.tag == 18


class TestGroups:
    def test_typed_reads_over_groups(self):
        new_order = order(
            "35=D|11=ORDER-1|453=2|448=AL|452=1|802=2|523=S1|523=S2|448=BOB|452=03|38=200|"
        )

        parties = new_order.party_ids
        assert [party.party_id for party in parties] == ["AL", "BOB"]
        assert [party.party_role for party in parties] == [
            PartyRole.EXECUTING_FIRM,
            PartyRole.CLIENT_ID,
        ]
        assert [sub_id.party_sub_id for sub_id in parties[0].party_sub_ids] == [
            "S1",
            "S2",
        ]
        assert parties[1].party_sub_ids == ()
        assert new_order.order_qty_data.order_qty == 200

    def test_a_group_instance_is_an_object_of_the_groups_class(self):
        new_order = order("35=D|453=1|448=AL|")

        party: Parties = new_order.party_ids[0]
        assert type(party) is Parties

    def test_an_absent_group_reads_as_empty(self):
        assert order("35=D|11=ORDER-1|").party_ids == ()

    def test_a_malformed_group_raises(self):
        with pytest.raises(InvalidValueError) as excinfo:
            _ = order("35=D|453=2|448=AL|38=200|").party_ids

        assert excinfo.value.tag == 453

    def test_an_unrecognized_int_code_is_representable(self):
        party = order("35=D|453=1|448=AL|452=99|").party_ids[0]

        assert party.party_role == Unrecognized(99)

    def test_a_non_integer_int_code_raises(self):
        party = order("35=D|453=1|448=AL|452=X|").party_ids[0]

        with pytest.raises(InvalidValueError) as excinfo:
            _ = party.party_role

        assert excinfo.value.tag == 452

    def test_reads_a_group_declared_in_its_message(self):
        logon = Logon.from_raw(tokenize_body("35=A|384=2|372=D|372=8|"))

        assert [msg_type.ref_msg_type for msg_type in logon.msg_types] == ["D", "8"]

    def test_reads_a_group_declared_in_a_group_instance(self):
        quote = MassQuote.from_raw(
            tokenize_body(
                "35=i|117=Q1|296=2|302=SET-1|295=2|299=E1|299=E2|302=SET-2|295=1|299=E3|"
            )
        )

        entries = [
            (quote_set.quote_set_id, entry.quote_entry_id)
            for quote_set in quote.quote_sets
            for entry in quote_set.quote_entries
        ]
        assert entries == [("SET-1", "E1"), ("SET-1", "E2"), ("SET-2", "E3")]

    def test_an_absent_group_in_a_group_instance_reads_as_empty(self):
        quote = MassQuote.from_raw(tokenize_body("35=i|117=Q1|296=1|302=SET-1|"))

        assert quote.quote_sets[0].quote_entries == ()

    def test_a_header_field_after_a_group_ends_it(self):
        logon = Logon.from_raw(tokenize_body("35=A|384=1|372=D|49=SENDER|"))

        assert logon.msg_types[0].raw.get(49) is None

    def test_a_group_read_is_cached(self):
        new_order = order("35=D|453=1|448=AL|")

        assert new_order.party_ids is new_order.party_ids


class TestComponents:
    def test_reads_a_component_through_its_view(self):
        new_order = order("35=D|11=ORDER-1|55=ACME|1186=schema.xsd|38=200|")

        assert new_order.instrument.symbol == "ACME"
        assert new_order.order_qty_data.order_qty == 200
        assert new_order.instrument.security_xml.security_xml_schema == "schema.xsd"

    def test_a_view_is_an_object_of_its_component_class(self):
        new_order = order("35=D|11=ORDER-1|55=ACME|")

        instrument: Instrument = new_order.instrument
        assert isinstance(instrument, Instrument)
        assert new_order.instrument is instrument

    def test_a_view_reads_the_groups_of_its_component(self):
        new_order = order("35=D|11=ORDER-1|55=ACME|454=2|455=ALT-1|455=ALT-2|38=200|")

        alt_ids = new_order.instrument.security_alt_id
        assert [alt_id.security_alt_id for alt_id in alt_ids] == ["ALT-1", "ALT-2"]
        assert new_order.order_qty_data.order_qty == 200

    def test_a_group_instance_reaches_a_component_through_its_view(self):
        quote = MassQuote.from_raw(
            tokenize_body(
                "35=i|117=Q1|296=1|302=SET-1|295=2|"
                "299=E1|55=ACME|454=1|455=ALT-1|299=E2|55=INIT|"
            )
        )

        first, second = quote.quote_sets[0].quote_entries
        assert first.instrument.symbol == "ACME"
        assert second.instrument.symbol == "INIT"
        assert len(first.instrument.security_alt_id) == 1
        assert second.instrument.security_alt_id == ()

    def test_a_group_instance_and_its_view_read_a_field_they_both_list(self):
        quote = MassQuote.from_raw(
            tokenize_body("35=i|117=Q1|296=1|302=SET-1|295=1|299=E1|55=ACME|")
        )

        entry = quote.quote_sets[0].quote_entries[0]
        assert entry.symbol == "ACME"
        assert entry.instrument.symbol == "ACME"

    def test_a_view_prints_the_fields_of_its_component(self):
        new_order = order(
            "35=D|11=ORDER-1|55=ACME|454=1|455=ALT-1|1186=schema.xsd|38=200|"
        )

        assert (
            repr(new_order.instrument)
            == "Instrument(55=ACME|454=1|455=ALT-1|1186=schema.xsd)"
        )
        assert repr(new_order.instrument.security_xml) == "SecurityXML(1186=schema.xsd)"
        assert repr(new_order.order_qty_data) == "OrderQtyData(38=200)"

    def test_a_view_in_a_group_instance_prints_that_instance_alone(self):
        quote = MassQuote.from_raw(
            tokenize_body(
                "35=i|117=Q1|296=1|302=SET-1|295=2|299=E1|55=ACME|299=E2|55=INIT|"
            )
        )

        entries = quote.quote_sets[0].quote_entries
        assert repr(entries[1].instrument) == "Instrument(55=INIT)"


class TestPrinting:
    def test_a_message_prints_under_its_class_name(self):
        new_order = order("35=D|11=ORDER-1|")

        assert (
            repr(new_order) == "NewOrderSingle(8=FIX.4.4|9=16|35=D|11=ORDER-1|10=099)"
        )

    def test_a_group_instance_prints_its_own_fields(self):
        new_order = order("35=D|453=2|448=AL|802=1|523=S1|448=BOB|")

        first, second = new_order.party_ids
        assert repr(first) == "Parties(448=AL|802=1|523=S1)"
        assert repr(first.party_sub_ids[0]) == "PtysSubGrp(523=S1)"
        assert repr(second) == "Parties(448=BOB)"

    def test_a_group_a_message_declares_prints_its_path(self):
        logon = Logon.from_raw(tokenize_body("35=A|384=1|372=D|"))

        assert repr(logon.msg_types[0]) == "Logon.MsgTypes(372=D)"

    def test_a_group_in_a_group_instance_prints_its_path(self):
        quote = MassQuote.from_raw(tokenize_body("35=i|296=1|302=SET-1|295=1|299=E1|"))

        entries = quote.quote_sets[0].quote_entries
        entry: MassQuote.QuoteSets.QuoteEntries = entries[0]
        assert repr(entry) == "MassQuote.QuoteSets.QuoteEntries(299=E1)"


class TestHeader:
    def test_reads_the_header_and_trailer(self):
        new_order = order(
            "35=D|49=SENDER|56=TARGET|34=7|52=20261030-14:29:51.123456789|11=ORDER-1|"
            "93=4|89=SIGN|"
        )

        assert new_order.header.begin_string == "FIX.4.4"
        assert new_order.header.msg_seq_num == 7
        assert new_order.header.msg_type == "D"
        assert new_order.header.sender_comp_id == "SENDER"
        assert new_order.header.target_comp_id == "TARGET"
        assert new_order.header.sending_time == datetime(
            2026, 10, 30, 14, 29, 51, 123456, tzinfo=UTC
        )
        assert new_order.header.sending_time_ns == 1_793_370_591_123_456_789
        assert new_order.trailer.signature == b"SIGN"
        assert new_order.trailer.check_sum is not None

    def test_reads_a_header_without_the_message_type(self):
        header = Header.from_raw(tokenize_body("35=D|49=SENDER|11=ORDER-1|"))

        assert header.sender_comp_id == "SENDER"

    def test_a_message_header_ends_its_groups_where_the_body_starts(self):
        logon = Logon.from_raw(
            tokenize_body("35=A|627=2|628=HOP-1|628=HOP-2|384=1|372=D|")
        )

        hops = logon.header.hops
        assert [hop.hop_comp_id for hop in hops] == ["HOP-1", "HOP-2"]
        assert hops[1].raw.get(384) is None

    def test_a_header_from_raw_keeps_body_fields_in_its_last_group(self):
        header = Header.from_raw(
            tokenize_body("35=A|627=2|628=HOP-1|628=HOP-2|384=1|372=D|")
        )

        assert header.hops[1].raw.get(384) == b"1"
