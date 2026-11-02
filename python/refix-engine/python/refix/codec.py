from collections.abc import Callable
from dataclasses import dataclass

from refix._core import Dialect, RawMessage


@dataclass(frozen=True)
class Codec[T]:
    """A dialect and the decoder generated with it."""

    dialect: Dialect
    decode: Callable[[RawMessage], T]
