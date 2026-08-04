import os
from collections.abc import Iterator, Sequence
from typing import BinaryIO

from refix._core import Garble, MessageStream, RawMessage

_CHUNK_SIZE = 65536


def read_log(
    source: str | os.PathLike[str] | BinaryIO,
    *,
    extra_length_tags: Sequence[int] = (),
) -> Iterator[RawMessage | Garble]:
    """Yields each message in a FIX message log, in order.

    Bytes between frames that are not FIX (timestamps, newlines, junk) are yielded as `Garble`.
    A truncated frame at the end of the log is ignored.
    """
    if isinstance(source, (str, os.PathLike)):
        with open(source, "rb") as f:
            yield from _drive(f, extra_length_tags)
    else:
        yield from _drive(source, extra_length_tags)


def _drive(
    file: BinaryIO, extra_length_tags: Sequence[int]
) -> Iterator[RawMessage | Garble]:
    stream = MessageStream(extra_length_tags=extra_length_tags)
    while chunk := file.read(_CHUNK_SIZE):
        stream.feed(chunk)
        while (outcome := stream.next_message()) is not None:
            yield outcome
