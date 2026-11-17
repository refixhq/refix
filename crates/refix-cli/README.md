# refix-cli

The `refix` command-line tool for [ReFIX](https://refix.dev). It generates typed Rust and Python messages
from a QuickFIX-format dictionary.

> ReFIX is in early development, and APIs change between 0.x releases.

```sh
cargo install refix-cli
refix codegen FIX44.xml --rust fix44.rs --python fix44.py
```

Without a Rust toolchain, `uv tool install refix-cli` installs a prebuilt binary. The generated Rust module depends on
`refix-message`, and the Python module on the `refix` package from PyPI. The stock FIX 4.2 and 4.4 messages are
published ready-made as `refix-fix42` and `refix-fix44`.

From FIX 5.0 the header, trailer and session messages are in a transport dictionary. Give it with `--transport`:

```sh
refix codegen FIX50SP2.xml --transport FIXT11.xml --rust fix50sp2.rs --python fix50sp2.py
```

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
