# refix-cli

The `refix` command-line tool for [ReFIX](https://github.com/refixhq/refix). It generates typed Rust and Python messages
from a QuickFIX-format dictionary.

> ReFIX is in early development, and APIs change between 0.x releases.

```sh
uv tool install refix-cli
refix codegen FIX44.xml --python fix44.py --rust fix44.rs
```

The generated Python module needs `refix-engine` at runtime. The stock FIX 4.4 messages are published ready-made as
`refix-fix44`.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
