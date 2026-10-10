# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/refixhq/refix/releases/tag/refix-codegen-v0.1.0) - 2026-10-10

### Added

- publish the refix CLI ([#84](https://github.com/refixhq/refix/pull/84))
- read CHAR fields as text ([#74](https://github.com/refixhq/refix/pull/74))
- generate header and trailer views ([#73](https://github.com/refixhq/refix/pull/73))
- model the header and trailer in the dictionary ([#72](https://github.com/refixhq/refix/pull/72))
- settle name clashes so every dictionary generates ([#71](https://github.com/refixhq/refix/pull/71))
- report name clashes in generated code ([#70](https://github.com/refixhq/refix/pull/70))
- prefix enum names that start with a digit ([#69](https://github.com/refixhq/refix/pull/69))
- generate Python groups ([#68](https://github.com/refixhq/refix/pull/68))
- generate typed Python messages ([#65](https://github.com/refixhq/refix/pull/65))
- add the refix CLI and split out the Rust backend ([#64](https://github.com/refixhq/refix/pull/64))
- type generated enums by their field's FIX type ([#62](https://github.com/refixhq/refix/pull/62))
- generate typed access to repeating groups ([#60](https://github.com/refixhq/refix/pull/60))
- model repeating groups in the dictionary ([#56](https://github.com/refixhq/refix/pull/56))
- resolve specs into validated dictionaries ([#51](https://github.com/refixhq/refix/pull/51))
- model components in dictionaries and implement quickfix parsing ([#49](https://github.com/refixhq/refix/pull/49))
- complete code generation for enum values ([#48](https://github.com/refixhq/refix/pull/48))
- add converter function to convert enums to PascalCase ([#46](https://github.com/refixhq/refix/pull/46))
- support enum values in data dictionaries ([#45](https://github.com/refixhq/refix/pull/45))
- end-to-end codegen with integration tests to cover it ([#44](https://github.com/refixhq/refix/pull/44))
- first cut of code emitter ([#43](https://github.com/refixhq/refix/pull/43))
- add initial structure for refix-codegen crate ([#42](https://github.com/refixhq/refix/pull/42))
- first pass the the README
- initial commit with placeholder README.md

### Fixed

- rename the generated enum catch-all to Unrecognized ([#61](https://github.com/refixhq/refix/pull/61))

### Other

- use the Tag newtype in the dictionary ([#53](https://github.com/refixhq/refix/pull/53))
- introduce a Tag newtype for FIX tag numbers ([#52](https://github.com/refixhq/refix/pull/52))
- rename Dictionary to Spec ([#50](https://github.com/refixhq/refix/pull/50))
- split emitter up into submodules ([#47](https://github.com/refixhq/refix/pull/47))
- add codecov badge ([#24](https://github.com/refixhq/refix/pull/24))
- add badges to readme ([#23](https://github.com/refixhq/refix/pull/23))
- add license files ([#22](https://github.com/refixhq/refix/pull/22))
