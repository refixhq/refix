# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
