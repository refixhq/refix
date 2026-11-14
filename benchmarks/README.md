# Benchmarks

ReFIX against the Python FIX libraries [simplefix](https://pypi.org/project/simplefix/) and
[QuickFIX](https://pypi.org/project/quickfix/), on reading messages.

## Running

```sh
just sync
just pycompare
```

QuickFIX compiles its C++ engine on install, so it is opt-in. Its cases are skipped without it.

```sh
uv sync --project python/refix-engine --group quickfix
just pycompare
```

## Results

Measured on two machines:

- A MacBook Pro with an Apple M3 Max, running macOS 26.3 and Python 3.12.13.
- An AWS c7a.xlarge with an AMD EPYC 9R14, running Ubuntu 26.04 and Python 3.12.16.

| Library   | Version |
|-----------|---------|
| refix     | 0.7.1   |
| simplefix | 1.0.17  |
| quickfix  | 1.16.0  |

### Read one message

Parse a 30-field ExecutionReport and read ClOrdID, OrderQty, Price and TransactTime. Raw
values are returned as the library stores them. Typed values are a `str`, an `int`, a
`Decimal` and a `datetime`.

Median in microseconds, with the ratio to ReFIX.

Apple M3 Max:

|           | Raw values  | Typed values |
|-----------|-------------|--------------|
| ReFIX     | 0.50        | 0.88         |
| QuickFIX  | 3.37 (6.8x) | 6.54 (7.5x)  |
| simplefix | 40.50 (81x) | 44.04 (50x)  |

AMD EPYC 9R14:

|           | Raw values  | Typed values |
|-----------|-------------|--------------|
| ReFIX     | 0.73        | 1.35         |
| QuickFIX  | 5.86 (8.0x) | 10.98 (8.1x) |
| simplefix | 64.94 (89x) | 70.56 (52x)  |