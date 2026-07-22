# Contributing to ReFIX

ReFIX is in early development and conventions are still settling. This document
describes how the repository is organised and the practices we follow.

## Repository layout

- `crates/` — pure Rust crates. Every crate in this directory is intended for
  publication to crates.io.
- `python/` — everything Python. `python/bindings` contains the PyO3 crate
  (`refix-bindings`) together with the pure-Python package; it builds the
  `refix-engine` distribution on PyPI (imported as `refix`) and is never
  published to crates.io. Other Python-related directories (benchmarks,
  examples) will live alongside it.

## Branches, commits and pull requests

Work happens on feature branches off `main`, merged via pull request.

- **Pull requests are squash-merged.** The PR title becomes the commit message
  on `main`, so it must follow
  [Conventional Commits](https://www.conventionalcommits.org/): a type prefix
  such as `feat:`, `fix:`, `docs:`, `chore:`, with no scope. Mark breaking
  changes with `!` (e.g. `feat!: ...`).
- PR titles drive releases: version bumps and changelogs are generated from
  the commit history on `main` (via release-plz), so choose the type
  deliberately.
- **Interim commits within a PR have no prescribed format.** Ordinary git
  style is appreciated — a capitalised, imperative subject line — but nothing
  is enforced. Since these messages disappear on squash, put anything worth
  keeping in the PR title or description.
- Keep PRs to a single concern, so the squashed commit forms a logical unit.

## Dependencies

- All dependency versions are declared once, in `[workspace.dependencies]` in
  the root `Cargo.toml`. Member crates reference them with
  `{ workspace = true }` and never carry their own version numbers.
- Declared versions should be the minimum actually required, not simply the
  latest at the time of writing.

## Building

Rust:

```bash
cargo build
cargo test
```

The Python bindings are excluded from default cargo commands and are built
with [maturin](https://www.maturin.rs/):

```bash
cd python/bindings
python -m venv .venv && source .venv/bin/activate
pip install maturin
maturin develop
```

## License

ReFIX is dual-licensed under MIT or Apache-2.0. Unless you explicitly state
otherwise, any contribution intentionally submitted for inclusion in the work
by you shall be dual-licensed as above, without any additional terms or
conditions.
