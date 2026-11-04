# refix-message

Framing, log reading, tokenising and raw FIX messages for [ReFIX](https://github.com/refixhq/refix). A `RawMessage`
indexes a frame's fields and reads them on demand, without copying.

> ReFIX is in early development, and APIs change between 0.x releases.

```rust
use refix_message::{LogReader, Tag};

fn main() {
    let log = b"2026-10-10 09:30:00 IN 8=FIX.4.4|9=5|35=0|10=163|\n";
    let mut reader = LogReader::default();
    reader.feed(log);
    for record in reader.finish() {
        let prefix = String::from_utf8_lossy(&record.prefix);
        println!("{prefix:?} {:?} {:?}", record.message.get_str(Tag(35)), record.diagnostics);
    }
}
```

`MessageStream` frames a session's bytes strictly, and `Tokenizer::tokenize` reads a single frame. Typed messages are
generated on top of the tokenizer: `refix-fix44` for FIX 4.4, or `refix-cli` for your own dictionary.

## Features

- `bigdecimal-04` converts `Decimal` to and from bigdecimal 0.4.
- `chrono-04` converts `Date`, `Time` and `UtcTimestamp` to and from chrono 0.4.
- `jiff-02` converts `Date`, `Time` and `UtcTimestamp` to and from jiff 0.2.
- `rust_decimal-1` converts `Decimal` to and from rust_decimal 1.
- `time-03` converts `Date`, `Time` and `UtcTimestamp` to and from time 0.3.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
