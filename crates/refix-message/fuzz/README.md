# Fuzzing the framing scanner

Two fuzzer targets, run with `cargo-fuzz` (`cargo install cargo-fuzz`) on a nightly toolchain. Run from the crate root,
`crates/refix-message`, so the paths below resolve.

```sh
cargo +nightly fuzz run scan_invariants \
    fuzz/corpus/scan_invariants fuzz/seeds -- -dict=fuzz/dict.txt -max_total_time=60

cargo +nightly fuzz run chunking_equivalence \
    fuzz/corpus/chunking_equivalence fuzz/seeds_chunking -- -dict=fuzz/dict.txt -max_total_time=60
```

## Targets

- `scan_invariants` asserts the per-call contract: a returned frame is a prefix of the input, starts with `8=FIX`, ends
  on SOH, and rescans to itself; a garble reports a skip in `1..=len` so the caller always progresses.
- `chunking_equivalence` consumes the first input byte as a chunk size and asserts that driving the scanner over the
  rest in chunks of that size yields the same frame sequence as scanning it whole. Framing must not depend on how the
  byte stream happens to be split across reads.

`dict.txt` gives the mutator the wire tokens (`8=FIX`, `9=`, `35=`, `10=`, SOH, whole small fields). Without it almost
every input dies at the BeginString anchor and never reaches the framing arithmetic.

`corpus/`, `artifacts/`, `target/` and `coverage/` are generated and ignored. When a run does find a crash, minimise it
with `cargo fuzz tmin` and translate the result into a unit test in `framing.rs` rather than committing the artifact.

## seeds/

Inputs for `scan_invariants`. SOH is shown as `|`. The outcome column is what the scanner actually does with the bytes,
driving `scan` in a loop until Incomplete.

### embedded_begin_string (40 bytes)

```
8=FIX.4.4|9=18|35=A|58=8=FIX.4.4|10=179|
```

`8=FIX.4.4` appearing inside a Text (58) field. Framing must not treat it as a boundary.

Outcome: `Frame(35=A)`

### fixt_logon (47 bytes)

```
8=FIXT.1.1|9=24|35=A|34=1|1128=9|1137=9|10=143|
```

FIXT.1.1 with ApplVerID, so `BeginString::Fixt11` is recognised.

Outcome: `Frame(35=A)`

### garbage_then_frame (31 bytes)

```
YY8=Z8=FIX.4.4|9=5|35=0|10=163|
```

Leading junk, then `8=Z` which is not a FIX BeginString, then a real frame. Drives the resync search; this is the shape
of the frame boundary bug fixed in an earlier commit.

Outcome: `Garbled(MissingBeginString, skip 5) -> Frame(35=0)`

### heartbeat (32 bytes)

```
8=FIX.4.4|9=10|35=0|34=2|10=166|
```

Smallest realistic frame.

Outcome: `Frame(35=0)`

### logon (57 bytes)

```
8=FIX.4.4|9=35|35=A|34=1|49=ME|56=YOU|98=0|108=30|10=185|
```

Standard Logon. The baseline happy path.

Outcome: `Frame(35=A)`

### new_order (86 bytes)

```
8=FIX.4.2|9=64|35=D|34=3|49=ME|56=YOU|11=ord1|55=AAPL|54=1|38=100|40=2|44=1.23|10=209|
```

FIX.4.2 application message with a longer body and many fields.

Outcome: `Frame(35=D)`

### partial_tail (36 bytes)

```
8=FIX.4.4|9=10|35=0|34=9|10=173|8=FI
```

A complete frame followed by a bare `8=FI`. Drives preservation of a partial BeginString that a later read could
complete.

Outcome: `Frame(35=0) -> Incomplete(+4 buffered)`

### three_frames (96 bytes)

```
8=FIX.4.4|9=10|35=0|34=4|10=168|8=FIX.4.4|9=10|35=0|34=4|10=168|8=FIX.4.4|9=10|35=0|34=4|10=168|
```

Three frames back to back.

Outcome: `Frame(35=0) -> Frame(35=0) -> Frame(35=0)`

### truncated (32 bytes)

```
8=FIX.4.4|9=16|35=A|34=1|49=ME|1
```

A frame cut short. Drives the Incomplete path.

Outcome: `Incomplete(+32 buffered)`

### two_frames (64 bytes)

```
8=FIX.4.4|9=10|35=0|34=3|10=167|8=FIX.4.4|9=10|35=0|34=3|10=167|
```

Two frames back to back. Drives the caller's advance loop.

Outcome: `Frame(35=0) -> Frame(35=0)`

### unknown_version (32 bytes)

```
8=FIX.9.9|9=10|35=0|34=1|10=175|
```

`8=FIX.9.9` frames normally as `BeginString::Other`.

Outcome: `Frame(35=0)`

## seeds_chunking/

Inputs for `chunking_equivalence`. Each is one of the streams above prefixed with a single chunk-size byte, so the file
name suffix `_cs<N>` is that chunk size. Size 1 is byte-by-byte delivery, the most aggressive interleaving and where the
frame boundary bug appeared; 255 delivers every stream above in a single piece.

The chunk-sensitive streams (resync, partial tails, multiple frames, truncation)
carry variants at several sizes; the rest are seeded at size 7 only.

| stream                  | chunk sizes     | frames recovered | invariant holds |
|-------------------------|-----------------|------------------|-----------------|
| `embedded_begin_string` | 1, 2, 3, 7, 255 | 1                | yes             |
| `fixt_logon`            | 7               | 1                | yes             |
| `garbage_then_frame`    | 1, 2, 3, 7, 255 | 1                | yes             |
| `heartbeat`             | 7               | 1                | yes             |
| `logon`                 | 7               | 1                | yes             |
| `new_order`             | 7               | 1                | yes             |
| `partial_tail`          | 1, 2, 3, 7, 255 | 1                | yes             |
| `three_frames`          | 1, 2, 3, 7, 255 | 3                | yes             |
| `truncated`             | 1, 2, 3, 7, 255 | 0                | yes             |
| `two_frames`            | 1, 2, 3, 7, 255 | 2                | yes             |
| `unknown_version`       | 7               | 1                | yes             |
