import os
from collections.abc import Iterator
from typing import BinaryIO

from refix._core import Dialect, Garble, MessageStream, RawMessage
from refix.codec import Codec

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


def decode_log[T](
    source: str | os.PathLike[str] | BinaryIO,
    codec: Codec[T],
) -> Iterator[T | Garble]:
    """Yields each message in a FIX message log, decoded by `codec`, in order.

    Garbles and a truncated tail are handled as in `read_log`.
    """
    for outcome in read_log(source, codec.dialect):
        yield outcome if isinstance(outcome, Garble) else codec.decode(outcome)


def _drive(file: BinaryIO, dialect: Dialect | None) -> Iterator[RawMessage | Garble]:
    stream = MessageStream(dialect)
    while chunk := file.read(_CHUNK_SIZE):
        stream.feed(chunk)
        while (outcome := stream.next_message()) is not None:
            yield outcome
