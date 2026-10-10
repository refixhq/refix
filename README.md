<div align="center">

# ReFIX

[![crates-badge]](https://crates.io/crates/refix)
[![pypi-badge]](https://pypi.org/project/refix-engine/)
[![docs-badge]](https://docs.rs/refix)
[![codecov](https://codecov.io/gh/refixhq/refix/graph/badge.svg?token=S738KU2U1K)](https://codecov.io/gh/refixhq/refix)
[![CI](https://github.com/refixhq/refix/actions/workflows/ci.yml/badge.svg)](https://github.com/refixhq/refix/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)][license-mit]
[![License: Apache 2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)][license-apache]

**ReFIX is a FIX engine written in pure Rust with first-party bindings for Python.**

</div>

> [!WARNING]
> ReFIX is in early development. Currently, it supports reading TagValue FIX messages and code generation for
> typed messages. Message serialisation, the network and session layers are not implemented yet,
> and APIs change between 0.x releases.

## Quick start

The Python bindings require Python 3.12 or later.

```sh
pip install refix-engine refix-fix44
```

```python
import refix
from refix_fix44 import NewOrderSingle

for message in refix.read_log("fix.log"):
    if isinstance(message, refix.RawMessage) and message.get(35) == b"D":
        order = NewOrderSingle(message)
        print(order.header.sender_comp_id, order.cl_ord_id, order.symbol)
```

In Rust, `cargo add refix-fix44`; see [refix-fix44].

## Why ReFIX?

ReFIX builds FIX building blocks with first-class support for both Rust and Python. See
the [longer blog post](https://davidsteiner.dev/writing/refix-a-new-fix-engine) for the motivations. If you need a
complete FIX 4.4 engine today, see [HotFIX](https://github.com/Validus-Risk-Management/hotfix), which I built before
ReFIX.

## Near-term goals

Reading and typing messages works in both languages. Planned next:

- better type coverage: prices, quantities and timestamps
- stock packages for more FIX versions
- reading each message as its type automatically
- building and serialising messages
- the session layer

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE][license-apache] or
  https://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT][license-mit] or
  https://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as
defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.


[crates-badge]: https://img.shields.io/crates/v/refix.svg

[pypi-badge]: https://img.shields.io/pypi/v/refix-engine.svg

[docs-badge]: https://docs.rs/refix/badge.svg

[license-mit]: https://github.com/refixhq/refix/blob/main/LICENSE-MIT

[license-apache]: https://github.com/refixhq/refix/blob/main/LICENSE-APACHE

[refix-fix44]: https://crates.io/crates/refix-fix44