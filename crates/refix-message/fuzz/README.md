# Fuzzing refix-message

Three targets, run with `cargo-fuzz` (`cargo install cargo-fuzz`) on a nightly toolchain. From the repo root:

```sh
just fuzz scan_invariants seeds
just fuzz chunking_equivalence seeds_chunking
just fuzz tokenize_tiling seeds_tokenize
```

To invoke `cargo fuzz` directly, run from `crates/refix-message` and create the corpus directory first, e.g.

```sh
mkdir -p fuzz/corpus/scan_invariants
cargo +nightly fuzz run scan_invariants \
    fuzz/corpus/scan_invariants fuzz/seeds -- -dict=fuzz/dict.txt -max_total_time=60
```

## Targets

- `scan_invariants`: a returned frame is a prefix of the input, starts with `8=FIX`, ends on SOH and rescans to
  itself; a garble reports a skip in `1..=len` so the caller always progresses.
- `chunking_equivalence`: the first input byte is a chunk size; driving the scanner over the rest in chunks of that
  size yields the same frames as scanning it whole.
- `tokenize_tiling`: the whole tokenizer; a returned message's index must tile the frame - fields in wire order
  without overlap, every value ending on SOH, the last SOH ending the frame. Exercises the data field extents.

`dict.txt` gives the mutator the wire tokens; without it almost every input dies at the BeginString anchor.

`corpus/`, `artifacts/`, `target/` and `coverage/` are generated and ignored. Minimise a crash with
`cargo fuzz tmin` and turn it into a unit test in the module that owns the bug rather than committing the artifact.

## Seeds

SOH is shown as `|`; the seed files hold real SOH bytes.

`seeds/`, for `scan_invariants`:

- `embedded_begin_string`: `8=FIX.4.4|9=18|35=A|58=8=FIX.4.4|10=179|`
  a BeginString inside a field value is not a boundary -> `Frame(35=A)`
- `fixt_logon`: `8=FIXT.1.1|9=24|35=A|34=1|1128=9|1137=9|10=143|`
  FIXT.1.1 recognised -> `Frame(35=A)`
- `garbage_then_frame`: `YY8=Z8=FIX.4.4|9=5|35=0|10=163|`
  resync past junk and a false anchor -> `Garbled(MissingBeginString, skip 5)`, `Frame(35=0)`
- `heartbeat`: `8=FIX.4.4|9=10|35=0|34=2|10=166|`
  smallest realistic frame -> `Frame(35=0)`
- `logon`: `8=FIX.4.4|9=35|35=A|34=1|49=ME|56=YOU|98=0|108=30|10=185|`
  baseline happy path -> `Frame(35=A)`
- `new_order`: `8=FIX.4.2|9=64|35=D|34=3|49=ME|56=YOU|11=ord1|55=AAPL|54=1|38=100|40=2|44=1.23|10=209|`
  longer application message -> `Frame(35=D)`
- `partial_tail`: `8=FIX.4.4|9=10|35=0|34=9|10=173|8=FI`
  partial BeginString buffered for a later read -> `Frame(35=0)`, `Incomplete`
- `three_frames`: `8=FIX.4.4|9=10|35=0|34=4|10=168|` three times -> `Frame(35=0)` x3
- `truncated`: `8=FIX.4.4|9=16|35=A|34=1|49=ME|1`
  frame cut short -> `Incomplete`
- `two_frames`: `8=FIX.4.4|9=10|35=0|34=3|10=167|` twice -> `Frame(35=0)` x2
- `unknown_version`: `8=FIX.9.9|9=10|35=0|34=1|10=175|`
  an unknown BeginString frames as `Other` -> `Frame(35=0)`

`seeds_chunking/`, for `chunking_equivalence`: the streams above prefixed with one chunk-size byte, named `_cs<N>`.
Chunk-sensitive streams (resync, partial tails, multiple frames, truncation) carry sizes 1, 2, 3, 7 and 255; the rest
size 7 only.

`seeds_tokenize/`, for `tokenize_tiling`, all correctly checksummed so their mutants reach the tokenizer:

- `raw_data`: `8=FIX.4.4|9=23|35=0|95=3|96=a|b|58=ok|10=168|`
  data value with embedded SOH; the extent, not scanning, delimits it -> 96 indexed as `a|b`
- `raw_data_short_length`: `8=FIX.4.4|9=23|35=0|95=1|96=a|b|58=ok|10=166|`
  understated length, one digit away from every fallback path -> 96 as `a`, then a sentinel run `b`
- `xml_data`: `8=FIX.4.4|9=20|35=0|212=4|213=<x/>|10=204|`
  a second length pair -> 213 indexed as `<x/>`
