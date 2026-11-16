# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.4](https://github.com/refixhq/refix/compare/refix-message-v0.4.3...refix-message-v0.4.4) - 2026-11-16

### Added

- make TokenizeError a standard error and print generated messages ([#151](https://github.com/refixhq/refix/pull/151))

### Other

- add the guides, benchmarks, reference and roadmap ([#149](https://github.com/refixhq/refix/pull/149))

## [0.4.3](https://github.com/refixhq/refix/compare/refix-message-v0.4.2...refix-message-v0.4.3) - 2026-11-16

### Added

- print messages in wire form ([#139](https://github.com/refixhq/refix/pull/139))

### Other

- point the packages at refix.dev ([#148](https://github.com/refixhq/refix/pull/148))
- declare the minimum Rust version ([#145](https://github.com/refixhq/refix/pull/145))

## [0.4.2](https://github.com/refixhq/refix/compare/refix-message-v0.4.1...refix-message-v0.4.2) - 2026-11-14

### Other

- look up length tags in a bitmap ([#135](https://github.com/refixhq/refix/pull/135))
- check for an empty tag with a flag ([#134](https://github.com/refixhq/refix/pull/134))
- parse the tag in the same pass that finds its end ([#132](https://github.com/refixhq/refix/pull/132))
- add Criterion benchmarks for the tokenizer ([#131](https://github.com/refixhq/refix/pull/131))

## [0.4.1](https://github.com/refixhq/refix/compare/refix-message-v0.4.0...refix-message-v0.4.1) - 2026-11-13

### Added

- read messages logged with ^A for SOH ([#127](https://github.com/refixhq/refix/pull/127))
- add a lenient log reader ([#125](https://github.com/refixhq/refix/pull/125))

## [0.4.0](https://github.com/refixhq/refix/compare/refix-message-v0.3.4...refix-message-v0.4.0) - 2026-11-02

### Added

- [**breaking**] take a dialect in place of extra length tags ([#120](https://github.com/refixhq/refix/pull/120))

## [0.3.4](https://github.com/refixhq/refix/compare/refix-message-v0.3.3...refix-message-v0.3.4) - 2026-11-01

### Added

- convert decimals to and from bigdecimal ([#114](https://github.com/refixhq/refix/pull/114))
- convert decimals to and from rust_decimal ([#113](https://github.com/refixhq/refix/pull/113))
- convert dates, times and timestamps to and from time ([#112](https://github.com/refixhq/refix/pull/112))
- convert dates, times and timestamps to and from chrono ([#111](https://github.com/refixhq/refix/pull/111))
- convert dates, times and timestamps to and from jiff ([#110](https://github.com/refixhq/refix/pull/110))
- split timestamps into dates and times ([#102](https://github.com/refixhq/refix/pull/102))
- read FIX MonthYear fields ([#100](https://github.com/refixhq/refix/pull/100))
- read FIX booleans ([#99](https://github.com/refixhq/refix/pull/99))
- read FIX times ([#98](https://github.com/refixhq/refix/pull/98))
- read FIX dates ([#97](https://github.com/refixhq/refix/pull/97))
- read FIX timestamps ([#96](https://github.com/refixhq/refix/pull/96))

### Fixed

- declare the minimum versions of the conversion crates ([#115](https://github.com/refixhq/refix/pull/115))

## [0.3.3](https://github.com/refixhq/refix/compare/refix-message-v0.3.2...refix-message-v0.3.3) - 2026-10-17

### Added

- read FIX floats as decimals ([#91](https://github.com/refixhq/refix/pull/91))

## [0.3.2](https://github.com/refixhq/refix/compare/refix-message-v0.3.1...refix-message-v0.3.2) - 2026-10-10

### Other

- give each package its own README ([#87](https://github.com/refixhq/refix/pull/87))

## [0.3.1](https://github.com/refixhq/refix/compare/refix-message-v0.3.0...refix-message-v0.3.1) - 2026-10-10

### Fixed

- allow publishing the generated stock crate ([#79](https://github.com/refixhq/refix/pull/79))

### Other

- move the fuzz crate to the repo root ([#82](https://github.com/refixhq/refix/pull/82))

## [0.3.0](https://github.com/refixhq/refix/compare/refix-message-v0.2.0...refix-message-v0.3.0) - 2026-10-08

### Added

- walk groups with tables built at runtime ([#66](https://github.com/refixhq/refix/pull/66))
- type generated enums by their field's FIX type ([#62](https://github.com/refixhq/refix/pull/62))
- read typed group instances ([#59](https://github.com/refixhq/refix/pull/59))
- walk repeating groups over raw messages ([#57](https://github.com/refixhq/refix/pull/57))
- first cut of code emitter ([#43](https://github.com/refixhq/refix/pull/43))

### Other

- introduce a Tag newtype for FIX tag numbers ([#52](https://github.com/refixhq/refix/pull/52))
