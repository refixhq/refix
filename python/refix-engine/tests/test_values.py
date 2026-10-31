import pickle

from refix import MonthYear


class TestMonthYear:
    def test_equal_values_are_equal_and_hash_alike(self):
        assert MonthYear(2026, 12, day=18) == MonthYear(2026, 12, day=18)
        assert hash(MonthYear(2026, 12, day=18)) == hash(MonthYear(2026, 12, day=18))

    def test_tells_the_forms_apart(self):
        assert MonthYear(2026, 12) != MonthYear(2026, 12, day=1)
        assert MonthYear(2026, 12, day=1) != MonthYear(2026, 12, week=1)

    def test_pickles(self):
        value = MonthYear(2026, 12, week=3)
        assert pickle.loads(pickle.dumps(value)) == value
