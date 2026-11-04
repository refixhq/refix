# refix

FIX message processing for Python, powered by Rust. These are the Python bindings
of [ReFIX](https://github.com/refixhq/refix). Requires Python 3.12 or later.

> ReFIX is in early development. It reads FIX messages and types them through
> generated code. Building messages and the session layer are not implemented
> yet, and APIs change between 0.x releases.

```python
import refix

for record in refix.read_log("fix.log"):
    print(record.prefix, record.message.get(35), record.diagnostics)
```

`read_log` reads logs as they are: any separator, a logger's prefix before each message, and
messages with a wrong BodyLength or CheckSum or cut short, reported in `record.diagnostics`.
`refix.MessageStream` reads a session's bytes strictly, and `refix.Tokenizer` reads a single frame.

Typed messages come from generated code: `refix-fix44` for FIX 4.4, or
`refix-cli` for your own dictionary. `refix.decode_log` reads a log as typed
messages through the generated module's `CODEC`.

Up to 0.2, this package was published as `refix-engine`. Uninstall that before installing `refix`, as both
provide the same module.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
