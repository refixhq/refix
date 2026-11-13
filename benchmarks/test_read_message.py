from collections.abc import Callable
from datetime import UTC, datetime
from decimal import Decimal

import pytest
import refix
import simplefix

import messages

Fields = tuple[str | None, int | None, Decimal | None, datetime | None]
Reader = Callable[[bytes], Fields]
RawFields = tuple[bytes | str | None, ...]
RawReader = Callable[[bytes], RawFields]

EXPECTED: Fields = (
    "CLT0001",
    1000000,
    Decimal("1.0925"),
    datetime(2026, 7, 31, 10, 0, 0, 120000, tzinfo=UTC),
)
EXPECTED_RAW = (b"CLT0001", b"1000000", b"1.0925", b"20260731-10:00:00.120")


def refix_reader() -> Reader:
    tokenizer = refix.Tokenizer()

    def read(frame: bytes) -> Fields:
        message = tokenizer.tokenize(frame)
        return (
            message.get_str(11),
            message.get_int(38),
            message.get_decimal(44),
            message.get_utc_timestamp(60),
        )

    return read


def refix_raw_reader() -> RawReader:
    tokenizer = refix.Tokenizer()

    def read(frame: bytes) -> RawFields:
        message = tokenizer.tokenize(frame)
        return (
            message.get(11),
            message.get(38),
            message.get(44),
            message.get(60),
        )

    return read


def simplefix_reader() -> Reader:
    parser = simplefix.FixParser()

    def read(frame: bytes) -> Fields:
        parser.append_buffer(frame)
        message = parser.get_message()
        timestamp = message.get(60).decode()
        return (
            message.get(11).decode(),
            int(message.get(38)),
            Decimal(message.get(44).decode()),
            datetime.strptime(timestamp, "%Y%m%d-%H:%M:%S.%f").replace(tzinfo=UTC),
        )

    return read


def simplefix_raw_reader() -> RawReader:
    parser = simplefix.FixParser()

    def read(frame: bytes) -> RawFields:
        parser.append_buffer(frame)
        message = parser.get_message()
        return (
            message.get(11),
            message.get(38),
            message.get(44),
            message.get(60),
        )

    return read


def quickfix_reader() -> Reader:
    quickfix = pytest.importorskip("quickfix")

    def read(frame: bytes) -> Fields:
        message = quickfix.Message(frame.decode())
        timestamp = message.getField(60)
        return (
            message.getField(11),
            int(message.getField(38)),
            Decimal(message.getField(44)),
            datetime.strptime(timestamp, "%Y%m%d-%H:%M:%S.%f").replace(tzinfo=UTC),
        )

    return read


def quickfix_raw_reader() -> RawReader:
    quickfix = pytest.importorskip("quickfix")

    def read(frame: bytes) -> RawFields:
        message = quickfix.Message(frame.decode())
        return (
            message.getField(11),
            message.getField(38),
            message.getField(44),
            message.getField(60),
        )

    return read


@pytest.mark.benchmark(group="read one message, typed values")
@pytest.mark.parametrize(
    "reader",
    [refix_reader, simplefix_reader, quickfix_reader],
    ids=["refix", "simplefix", "quickfix"],
)
def test_read_message(benchmark, reader):
    read = reader()
    frame = messages.execution_report()

    fields = benchmark(read, frame)

    assert fields == EXPECTED


@pytest.mark.benchmark(group="read one message, raw values")
@pytest.mark.parametrize(
    "reader",
    [refix_raw_reader, simplefix_raw_reader, quickfix_raw_reader],
    ids=["refix", "simplefix", "quickfix"],
)
def test_read_message_raw(benchmark, reader):
    read = reader()
    frame = messages.execution_report()

    fields = benchmark(read, frame)

    assert tuple(as_bytes(field) for field in fields) == EXPECTED_RAW


def as_bytes(value: bytes | str | None) -> bytes | None:
    return value.encode() if isinstance(value, str) else value
