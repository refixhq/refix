from collections.abc import Callable
from datetime import UTC, datetime, time
from typing import Any, Protocol, TypeVar

import refix
from test_tokenizer import construct_valid_frame

T = TypeVar("T")


class Benchmark(Protocol):
    """The callable interface of pytest-benchmark's `benchmark` fixture."""

    def __call__(self, target: Callable[..., T], *args: Any) -> T: ...


EXECUTION_REPORT_BODY = (
    "35=8|34=123|49=BROKER|56=CLIENT|52=20260731-10:00:00.123|"
    "37=ORD0001|11=CLT0001|17=EXEC0001|150=F|39=2|55=EURUSD|54=1|"
    "38=1000000|44=1.0925|32=1000000|31=1.0925|14=1000000|151=0|"
    "6=1.0925|60=20260731-10:00:00.120|"
)


MARKET_DATA_BODY = "35=W|55=EURUSD|268=1|269=0|270=1.0925|273=10:00:00.123|"


def execution_report() -> bytes:
    return construct_valid_frame("FIX.4.4", EXECUTION_REPORT_BODY)


def market_data() -> bytes:
    return construct_valid_frame("FIX.4.4", MARKET_DATA_BODY)


def test_bench_tokenize(benchmark: Benchmark) -> None:
    frame = execution_report()
    tokenizer = refix.Tokenizer()

    message = benchmark(tokenizer.tokenize, frame)

    assert message.get(35) == b"8"


def test_bench_get_hit(benchmark: Benchmark) -> None:
    message = refix.Tokenizer().tokenize(execution_report())

    value = benchmark(message.get, 44)

    assert value == b"1.0925"


def test_bench_get_miss(benchmark: Benchmark) -> None:
    message = refix.Tokenizer().tokenize(execution_report())

    value = benchmark(message.get, 9999)

    assert value is None


def test_bench_entries(benchmark: Benchmark) -> None:
    message = refix.Tokenizer().tokenize(execution_report())

    entries = benchmark(message.entries)

    assert len(entries) == 23


def test_bench_get_time(benchmark: Benchmark) -> None:
    message = refix.Tokenizer().tokenize(market_data())

    value = benchmark(message.get_time, 273)

    assert value == time(10, 0, 0, 123000)


def test_bench_get_time_ns(benchmark: Benchmark) -> None:
    message = refix.Tokenizer().tokenize(market_data())

    value = benchmark(message.get_time_ns, 273)

    assert value == 36_000_123_000_000


def test_bench_get_utc_timestamp(benchmark: Benchmark) -> None:
    message = refix.Tokenizer().tokenize(execution_report())

    value = benchmark(message.get_utc_timestamp, 52)

    assert value == datetime(2026, 7, 31, 10, 0, 0, 123000, tzinfo=UTC)


def test_bench_get_utc_timestamp_ns(benchmark: Benchmark) -> None:
    message = refix.Tokenizer().tokenize(execution_report())

    value = benchmark(message.get_utc_timestamp_ns, 52)

    assert value == 1_785_492_000_123_000_000
