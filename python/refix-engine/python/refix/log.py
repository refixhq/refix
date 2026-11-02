import os
from collections.abc import Iterator
from typing import BinaryIO

from refix._core import Dialect, Garble, MessageStream, RawMessage

_CHUNK_SIZE = 65536


def read_log(
    source: str | os.PathLike[str] | BinaryIO,
    dialect: Dialect | None = None,
) -> Iterator[RawMessage | Garble]:
    """Yields each message in a FIX message log, in order.

    Bytes between frames that are not FIX (timestamps, newlines, junk) are yielded as `Garble`.
    A truncated frame at the end of the log is ignored.
    """
    if isinstance(source, (str, os.PathLike)):
        with open(source, "rb") as f:
            yield from _drive(f, dialect)
    else:
        yield from _drive(source, dialect)


def _drive(file: BinaryIO, dialect: Dialect | None) -> Iterator[RawMessage | Garble]:
    stream = MessageStream(dialect)
    while chunk := file.read(_CHUNK_SIZE):
        stream.feed(chunk)
        while (outcome := stream.next_message()) is not None:
            yield outcome
