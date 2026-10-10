# refix-engine

FIX message processing for Python, powered by Rust. These are the Python bindings
of [ReFIX](https://github.com/refixhq/refix). The import name is `refix`. Requires Python 3.12 or later.

> ReFIX is in early development. It reads FIX messages and types them through
> generated code. Building messages and the session layer are not implemented
> yet, and APIs change between 0.x releases.

```python
import refix

for message in refix.read_log("fix.log"):
    if isinstance(message, refix.RawMessage):
        print(message.get(35), message.get(49))
```

Anything between frames that isn't FIX comes back as `refix.Garble`.
`refix.Tokenizer` reads a single frame, and `refix.MessageStream` reads frames as bytes arrive.

Typed messages come from generated code: `refix-fix44` for FIX 4.4, or
`refix-cli` for your own dictionary.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
