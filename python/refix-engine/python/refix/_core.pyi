import builtins
from collections.abc import Sequence


class RawMessage:
    @property
    def bytes(self) -> builtins.bytes: ...

    def get(self, tag: int) -> builtins.bytes | None: ...

    def entries(self) -> list[tuple[int, builtins.bytes]]: ...


class Tokenizer:
    def __init__(self, *, extra_length_tags: Sequence[int] = ...) -> None: ...

    def tokenize(self, data: bytes) -> RawMessage: ...


class MessageStream:
    def __init__(self, *, extra_length_tags: Sequence[int] = ...) -> None: ...


MALFORMED_TAG: int


def version() -> str: ...
