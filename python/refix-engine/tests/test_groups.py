import gc
from datetime import date
from decimal import Decimal

import pytest
import refix
from refix.errors import InvalidValueError
from test_tokenizer import tokenize_body

PARTY_SUB_IDS = refix.GroupTable(802, 523, (523,), ())
PARTIES = refix.GroupTable(453, 448, (448, 452, 802), (PARTY_SUB_IDS,))
PARTIES_KNOWN = refix.KnownTags((448, 452, 523, 802))
KNOWN = refix.KnownTags((35, 54, 448, 452, 453, 523, 802))


def parties(body: str) -> tuple[refix.Scope, ...] | None:
    return tokenize_body(body).get_group(PARTIES, KNOWN)


class TestGroupTable:
    def test_builds_a_valid_table_with_a_nested_one(self):
        sub_ids = refix.GroupTable(802, 523, (523,), ())
        refix.GroupTable(453, 448, (448, 452, 802), (sub_ids,))

    def test_unsorted_members_raise(self):
        with pytest.raises(ValueError, match="sorted and unique"):
            refix.GroupTable(453, 448, (452, 448), ())

    def test_a_delimiter_outside_the_members_raises(self):
        with pytest.raises(ValueError, match="the delimiter 448"):
            refix.GroupTable(453, 448, (452,), ())

    def test_a_nested_count_tag_outside_the_members_raises(self):
        sub_ids = refix.GroupTable(802, 523, (523,), ())

        with pytest.raises(ValueError, match="the nested count tag 802"):
            refix.GroupTable(453, 448, (448,), (sub_ids,))


class TestKnownTags:
    def test_builds_from_sorted_unique_tags(self):
        refix.KnownTags((35, 448, 453))

    @pytest.mark.parametrize("tags", [(448, 35), (35, 35)])
    def test_unsorted_or_duplicate_tags_raise(self, tags: tuple[int, ...]):
        with pytest.raises(ValueError, match="sorted and unique"):
            refix.KnownTags(tags)


class TestGetGroup:
    def test_an_absent_group_is_none(self):
        assert parties("35=D|54=1|") is None

    def test_a_count_of_zero_is_an_empty_group(self):
        assert parties("35=D|453=0|54=1|") == ()

    def test_reads_each_instance_within_its_bounds(self):
        instances = parties("35=D|453=2|448=AL|452=3|448=BOB|54=1|")

        assert instances is not None
        assert [party.get_str(448) for party in instances] == ["AL", "BOB"]
        assert [party.get_int(452) for party in instances] == [3, None]
        assert instances[1].get(448) == b"BOB"

    def test_a_malformed_group_raises_on_the_count_tag(self):
        with pytest.raises(InvalidValueError) as excinfo:
            parties("35=D|453=2|448=AL|54=1|")

        assert excinfo.value.tag == 453

    def test_a_nested_group_is_read_from_its_instance(self):
        instances = parties("35=D|453=1|448=AL|802=2|523=S1|523=S2|54=1|")

        assert instances is not None
        sub_ids = instances[0].get_group(PARTY_SUB_IDS, PARTIES_KNOWN)
        assert sub_ids is not None
        assert [sub_id.get_str(523) for sub_id in sub_ids] == ["S1", "S2"]

    def test_an_instance_reads_multiple_values(self):
        table = refix.GroupTable(453, 448, (448, 523), ())
        message = tokenize_body("35=D|453=1|448=AL|523=S1 S2|54=1|")

        instances = message.get_group(table, KNOWN)

        assert instances is not None
        assert instances[0].get_multiple_values(523) == ("S1", "S2")

    def test_an_instance_keeps_its_message_alive(self):
        instances = parties("35=D|453=1|448=AL|54=1|")
        gc.collect()

        assert instances is not None
        assert instances[0].get_str(448) == "AL"

    def test_an_instance_reads_decimals(self):
        entries = refix.GroupTable(268, 269, (269, 270), ())

        message = tokenize_body("35=W|268=2|269=0|270=101.25|269=1|270=101.50|")

        instances = message.get_group(entries, refix.KnownTags((35, 268, 269, 270)))

        assert instances is not None
        assert [entry.get_decimal(270) for entry in instances] == [
            Decimal("101.25"),
            Decimal("101.50"),
        ]

    def test_an_instance_reads_booleans(self):
        allocs = refix.GroupTable(78, 79, (79, 208), ())

        message = tokenize_body("35=J|78=2|79=A1|208=Y|79=A2|208=N|")

        instances = message.get_group(allocs, refix.KnownTags((35, 78, 79, 208)))

        assert instances is not None
        assert [alloc.get_bool(208) for alloc in instances] == [True, False]

    def test_an_instance_reads_dates(self):
        entries = refix.GroupTable(268, 269, (269, 272), ())

        message = tokenize_body("35=W|268=2|269=0|272=20261030|269=1|272=20261031|")

        instances = message.get_group(entries, refix.KnownTags((35, 268, 269, 272)))

        assert instances is not None
        assert [entry.get_date(272) for entry in instances] == [
            date(2026, 10, 30),
            date(2026, 10, 31),
        ]

    def test_an_instance_reads_month_years(self):
        legs = refix.GroupTable(555, 600, (600, 610), ())

        message = tokenize_body("35=AB|555=2|600=ES|610=202612|600=ES|610=202703|")

        instances = message.get_group(legs, refix.KnownTags((35, 555, 600, 610)))

        assert instances is not None
        assert [leg.get_month_year(610) for leg in instances] == [
            refix.MonthYear(2026, 12),
            refix.MonthYear(2027, 3),
        ]

    def test_a_scope_cannot_be_built_directly(self):
        with pytest.raises(TypeError):
            refix.Scope()
