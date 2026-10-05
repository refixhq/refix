import enum
from dataclasses import dataclass


@dataclass(frozen=True)
class Unrecognized[T]:
    """A value the dictionary does not list for its field."""

    value: T

    def __str__(self) -> str:
        return str(self.value)


def from_value[E: enum.Enum, T](enum_type: type[E], value: T) -> E | Unrecognized[T]:
    """The member of `enum_type` with `value`, or `Unrecognized(value)`."""
    try:
        return enum_type(value)
    except ValueError:
        return Unrecognized(value)
