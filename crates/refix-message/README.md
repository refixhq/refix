# refix-message

Framing, tokenising and raw FIX messages for [ReFIX](https://github.com/refixhq/refix). A `RawMessage` indexes a frame's
fields and reads them on demand, without copying.

> ReFIX is in early development, and APIs change between 0.x releases.

```rust
use refix_message::stream::Outcome;
use refix_message::{MessageStream, Tag};

fn main() {
    let log = b"2026-10-10 09:30:00 IN 8=FIX.4.4\x019=5\x0135=0\x0110=163\x01\n";
    let mut stream = MessageStream::default();
    stream.feed(log);
    loop {
        match stream.next_message() {
            Outcome::Message(message) => println!("{:?}", message.get_str(Tag(35))),
            Outcome::Garbled { bytes, .. } => println!("skipped {} bytes", bytes.len()),
            Outcome::Incomplete => break,
        }
    }
}
```

`Tokenizer::tokenize` reads a single frame. Typed messages are generated on top of it: `refix-fix44` for FIX 4.4, or
`refix-cli` for your own dictionary.

## Features

- `bigdecimal-04` converts `Decimal` to and from bigdecimal 0.4.
- `chrono-04` converts `Date`, `Time` and `UtcTimestamp` to and from chrono 0.4.
- `jiff-02` converts `Date`, `Time` and `UtcTimestamp` to and from jiff 0.2.
- `rust_decimal-1` converts `Decimal` to and from rust_decimal 1.
- `time-03` converts `Date`, `Time` and `UtcTimestamp` to and from time 0.3.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
