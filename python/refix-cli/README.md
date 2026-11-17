# refix-cli

The `refix` command-line tool for [ReFIX](https://refix.dev). It generates typed Rust and Python messages
from a QuickFIX-format dictionary.

> ReFIX is in early development, and APIs change between 0.x releases.

```sh
uv tool install refix-cli
refix codegen FIX44.xml --python fix44.py --rust fix44.rs
```

The generated Python module needs `refix` at runtime. The stock FIX 4.2 and 4.4 messages are published ready-made as
`refix-fix42` and `refix-fix44`.

From FIX 5.0 the header, trailer and session messages are in a transport dictionary. Give it with `--transport`:

```sh
refix codegen FIX50SP2.xml --transport FIXT11.xml --python fix50sp2.py --rust fix50sp2.rs
```

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
