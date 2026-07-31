import builtins


class RawMessage:
    @property
    def bytes(self) -> builtins.bytes: ...

    def get(self, tag: int) -> builtins.bytes | None: ...

    def entries(self) -> list[tuple[int, builtins.bytes]]: ...


class Tokenizer:
    def tokenize(self, data: bytes) -> RawMessage: ...


MALFORMED_TAG: int


def version() -> str: ...
