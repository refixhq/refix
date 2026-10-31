# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.0](https://github.com/refixhq/refix/compare/python-v0.4.0...python-v0.5.0) - 2026-10-31

### Added

- read float fields as decimals in generated code ([#104](https://github.com/refixhq/refix/pull/104))
- read times and timestamps from Python ([#103](https://github.com/refixhq/refix/pull/103))
- read booleans, dates and MonthYear from Python ([#101](https://github.com/refixhq/refix/pull/101))

## [0.4.0](https://github.com/refixhq/refix/compare/python-v0.3.0...python-v0.4.0) - 2026-10-17

### Added

- read decimals from Python ([#93](https://github.com/refixhq/refix/pull/93))

## [0.3.0](https://github.com/refixhq/refix/compare/python-v0.2.2...python-v0.3.0) - 2026-10-11

### Added

- publish the Python package as refix ([#89](https://github.com/refixhq/refix/pull/89))

## [0.2.2](https://github.com/refixhq/refix/compare/python-v0.2.1...python-v0.2.2) - 2026-10-10

### Other

- give each package its own README ([#87](https://github.com/refixhq/refix/pull/87))

## [0.2.1](https://github.com/refixhq/refix/compare/python-v0.2.0...python-v0.2.1) - 2026-10-10

### Other

- re-release dependents only on breaking runtime releases ([#83](https://github.com/refixhq/refix/pull/83))

## [0.2.0](https://github.com/refixhq/refix/compare/python-v0.1.0...python-v0.2.0) - 2026-10-08

### Added

- generate header and trailer views ([#73](https://github.com/refixhq/refix/pull/73))
- generate Python groups ([#68](https://github.com/refixhq/refix/pull/68))
- read groups from Python ([#67](https://github.com/refixhq/refix/pull/67))
- generate typed Python messages ([#65](https://github.com/refixhq/refix/pull/65))
- read typed field values from Python ([#63](https://github.com/refixhq/refix/pull/63))

### Other

- introduce a Tag newtype for FIX tag numbers ([#52](https://github.com/refixhq/refix/pull/52))
- add ruff for automated linting and formatting ([#35](https://github.com/refixhq/refix/pull/35))
