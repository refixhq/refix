# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.1](https://github.com/refixhq/refix/compare/refix-fix42-v0.4.0...refix-fix42-v0.4.1) - 2026-11-28

### Other

- updated the following local packages: refix-message

## [0.4.0](https://github.com/refixhq/refix/compare/refix-fix42-v0.3.0...refix-fix42-v0.4.0) - 2026-11-25

### Added

- *(codegen)* [**breaking**] write a Rust module tree with a file per message and component ([#204](https://github.com/refixhq/refix/pull/204))
- *(codegen)* [**breaking**] write a Python package with a module per message and component ([#203](https://github.com/refixhq/refix/pull/203))
- *(codegen)* generate a warm-up function in Rust ([#202](https://github.com/refixhq/refix/pull/202))
- *(codegen)* [**breaking**] keep known tags private in Rust ([#199](https://github.com/refixhq/refix/pull/199))
- *(codegen)* print the own fields of a header, trailer or group instance in Rust ([#195](https://github.com/refixhq/refix/pull/195))
- *(codegen)* [**breaking**] wrap received messages with from_raw in Python ([#193](https://github.com/refixhq/refix/pull/193))
- *(codegen)* [**breaking**] make a group's class its component in Python ([#192](https://github.com/refixhq/refix/pull/192))
- *(codegen)* [**breaking**] make a group's type its component in Rust ([#189](https://github.com/refixhq/refix/pull/189))

### Fixed

- declare the refix-message version the stock crates need ([#207](https://github.com/refixhq/refix/pull/207))

### Other

- remove the generated files of the stock crates from git ([#205](https://github.com/refixhq/refix/pull/205))
- *(codegen)* build known tags from each component in Rust ([#201](https://github.com/refixhq/refix/pull/201))

## [0.3.0](https://github.com/refixhq/refix/compare/refix-fix42-v0.2.0...refix-fix42-v0.3.0) - 2026-11-21

### Added

- *(codegen)* [**breaking**] generate Python accessors as declarations ([#183](https://github.com/refixhq/refix/pull/183))

## [0.2.0](https://github.com/refixhq/refix/compare/refix-fix42-v0.1.0...refix-fix42-v0.2.0) - 2026-11-20

### Added

- *(codegen)* check the runtime version in generated Python ([#181](https://github.com/refixhq/refix/pull/181))

## [0.1.0](https://github.com/refixhq/refix/releases/tag/refix-fix42-v0.1.0) - 2026-11-17

### Added

- add stock FIX 4.2 packages ([#163](https://github.com/refixhq/refix/pull/163))
