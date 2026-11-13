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

Measured on a MacBook Pro with an Apple M3 Max, running macOS 26.3 and Python 3.12.13.

| Library   | Version |
|-----------|---------|
| refix     | 0.7.0   |
| simplefix | 1.0.17  |
| quickfix  | 1.16.0  |

### Read one message

Parse a 30-field ExecutionReport and read ClOrdID, OrderQty, Price and TransactTime. Raw
values are returned as the library stores them. Typed values are a `str`, an `int`, a
`Decimal` and a `datetime`.

Median in microseconds, with the ratio to ReFIX.

|           | Raw values  | Typed values |
|-----------|-------------|--------------|
| ReFIX     | 0.53        | 0.92         |
| QuickFIX  | 3.33 (6.2x) | 6.50 (7.1x)  |
| simplefix | 40.46 (76x) | 44.17 (48x)  |