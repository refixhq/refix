# ReFIX

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
