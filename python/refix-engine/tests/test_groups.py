import pytest
import refix


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
