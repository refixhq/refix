from dataclasses import dataclass


@dataclass(frozen=True)
class MonthYear:
    """A FIX MonthYear, such as a contract's maturity, with an optional day or week."""

    year: int
    month: int
    day: int | None = None
    week: int | None = None
