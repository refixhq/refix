# refix-cli

The `refix` command-line tool for [ReFIX](https://github.com/refixhq/refix). It generates typed Rust and Python messages
from a QuickFIX-format dictionary.

> ReFIX is in early development, and APIs change between 0.x releases.

```sh
cargo install refix-cli
refix codegen FIX44.xml --rust fix44.rs --python fix44.py
```

Without a Rust toolchain, `uv tool install refix-cli` installs a prebuilt binary. The generated Rust module depends on
`refix-message`, and the Python module on `refix-engine`. The stock FIX 4.4 messages are published ready-made as
`refix-fix44`.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
