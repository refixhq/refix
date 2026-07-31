# ReFIX

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE-MIT)
[![License: Apache 2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE-APACHE)

ReFIX is a FIX engine written in pure Rust with first-party bindings for Python.

> [!WARNING]
> Development on ReFIX has just started. In its current state, the project
> isn't useful, but make sure to check back later to monitor progress.

## Why ReFIX?

I previously built [HotFIX](https://github.com/Validus-Risk-Management/hotfix)
(also in pure Rust) with a very specific objective:
a robust FIX engine for buy-side use cases supporting FIX 4.4.
HotFIX is feature complete, and if you need an engine now,
I recommend you check it out to see if it fits your use case.

ReFIX has different goals - to build high-quality building blocks with
first-class support for both Python and Rust. The aim is still a functional engine
working end-to-end, but the design philosophy is different.

I've written a
[longer blog post](https://davidsteiner.dev/writing/refix-a-new-fix-engine)
on my motivations for ReFIX.

## Near-term goals

The first milestone is a message layer which natively supports Python,
with fully typed messages and minimal compromises on performance in either language.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  https://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or
  https://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
