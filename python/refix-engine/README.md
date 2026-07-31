# refix-engine

FIX message processing for Python, powered by Rust.

This package provides the Python bindings for
[ReFIX](https://github.com/refixhq/refix), a FIX engine written in pure Rust
with first-party Python support. The import name is `refix`:

```python
import refix

message = refix.Tokenizer().tokenize(frame)
print(message.get(35))
```

> Development on ReFIX has just started. In its current state, the project
> isn't useful, but make sure to check back later to monitor progress.

See the [project repository](https://github.com/refixhq/refix) for
documentation, roadmap and licensing (MIT OR Apache-2.0).
