import builtins
from collections.abc import Sequence

from refix.errors import GarbledReason


class RawMessage:
    @property
    def bytes(self) -> builtins.bytes: ...

    def get(self, tag: int) -> builtins.bytes | None: ...

    def entries(self) -> list[tuple[int, builtins.bytes]]: ...


class Tokenizer:
    def __init__(self, *, extra_length_tags: Sequence[int] = ...) -> None: ...

    def tokenize(self, data: bytes) -> RawMessage: ...


class Garble:
    @property
    def bytes(self) -> builtins.bytes: ...

    @property
    def reason(self) -> GarbledReason: ...


class MessageStream:
    def __init__(self, *, extra_length_tags: Sequence[int] = ...) -> None: ...

    def feed(self, data: bytes) -> None: ...

    def next_message(self) -> RawMessage | Garble | None: ...


MALFORMED_TAG: int


def version() -> str: ...
