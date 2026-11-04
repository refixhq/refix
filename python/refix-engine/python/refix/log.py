import enum
import os
from collections.abc import Iterator
from dataclasses import dataclass
from typing import BinaryIO

from refix._core import Dialect, LogReader, RawMessage
from refix.codec import Codec

_CHUNK_SIZE = 65536


class LogDiagnostic(enum.Enum):
    """A problem found with a record of a log."""

    MISSING_BODY_LENGTH = "missing_body_length"
    MALFORMED_BODY_LENGTH = "malformed_body_length"
    BODY_LENGTH_MISMATCH = "body_length_mismatch"
    MISSING_MSG_TYPE = "missing_msg_type"
    MALFORMED_CHECKSUM = "malformed_checksum"
    CHECKSUM_MISMATCH = "checksum_mismatch"
    TRUNCATED = "truncated"
    PREFIX_TRUNCATED = "prefix_truncated"


@dataclass(frozen=True)
class LogRecord[T]:
    """A message read from a log, with the bytes before it."""

    prefix: bytes
    message: T
    diagnostics: tuple[LogDiagnostic, ...]


def read_log(
    source: str | os.PathLike[str] | BinaryIO,
    dialect: Dialect | None = None,
    *,
    separator: bytes | None = None,
) -> Iterator[LogRecord[RawMessage]]:
    """Yields each message in a FIX log as a record, in order.

    Each message's separator is detected unless `separator` gives one. Bytes after the last
    message are dropped.
    """
    if separator is not None and len(separator) != 1:
        raise ValueError("separator must be a single byte")
    separator_byte = None if separator is None else separator[0]
    if isinstance(source, (str, os.PathLike)):
        with open(source, "rb") as f:
            yield from _drive(f, dialect, separator_byte)
    else:
        yield from _drive(source, dialect, separator_byte)


def decode_log[T](
    source: str | os.PathLike[str] | BinaryIO,
    codec: Codec[T],
    *,
    separator: bytes | None = None,
) -> Iterator[LogRecord[T]]:
    """Yields each message in a FIX log as a record decoded by `codec`, in order."""
    for record in read_log(source, codec.dialect, separator=separator):
        yield LogRecord(record.prefix, codec.decode(record.message), record.diagnostics)


def _drive(
    file: BinaryIO, dialect: Dialect | None, separator: int | None
) -> Iterator[LogRecord[RawMessage]]:
    reader = LogReader(dialect, separator)
    while chunk := file.read(_CHUNK_SIZE):
        reader.feed(chunk)
        while (record := reader.next_record()) is not None:
            yield _record(*record)
    for record in reader.finish():
        yield _record(*record)


def _record(
    prefix: bytes, message: RawMessage, diagnostics: list[str]
) -> LogRecord[RawMessage]:
    return LogRecord(prefix, message, tuple(LogDiagnostic(d) for d in diagnostics))
