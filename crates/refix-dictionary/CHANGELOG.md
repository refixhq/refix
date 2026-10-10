# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/refixhq/refix/releases/tag/refix-dictionary-v0.1.0) - 2026-10-10

### Added

- publish the refix CLI ([#84](https://github.com/refixhq/refix/pull/84))
- add the stock FIX44 packages ([#75](https://github.com/refixhq/refix/pull/75))
- model the header and trailer in the dictionary ([#72](https://github.com/refixhq/refix/pull/72))
- settle name clashes so every dictionary generates ([#71](https://github.com/refixhq/refix/pull/71))
- report name clashes in generated code ([#70](https://github.com/refixhq/refix/pull/70))
- type generated enums by their field's FIX type ([#62](https://github.com/refixhq/refix/pull/62))
- record where repeating groups are declared ([#58](https://github.com/refixhq/refix/pull/58))
- walk repeating groups over raw messages ([#57](https://github.com/refixhq/refix/pull/57))
- model repeating groups in the dictionary ([#56](https://github.com/refixhq/refix/pull/56))
- resolve specs into validated dictionaries ([#51](https://github.com/refixhq/refix/pull/51))
- model components in dictionaries and implement quickfix parsing ([#49](https://github.com/refixhq/refix/pull/49))
- support enum values in data dictionaries ([#45](https://github.com/refixhq/refix/pull/45))
- add Display implementations for errors and warnings ([#40](https://github.com/refixhq/refix/pull/40))
- turn duplicate field definitions into a clear error ([#39](https://github.com/refixhq/refix/pull/39))
- implement parsing of message and field definitions ([#38](https://github.com/refixhq/refix/pull/38))
- initial code for parsing QuickFIX XMLs into dictionaries ([#37](https://github.com/refixhq/refix/pull/37))
- scaffold the refix-dictionary crate ([#36](https://github.com/refixhq/refix/pull/36))
- first pass the the README
- initial commit with placeholder README.md

### Other

- restructure the dictionary resolver ([#55](https://github.com/refixhq/refix/pull/55))
- add typed indices to the dictionary resolver ([#54](https://github.com/refixhq/refix/pull/54))
- use the Tag newtype in the dictionary ([#53](https://github.com/refixhq/refix/pull/53))
- rename Dictionary to Spec ([#50](https://github.com/refixhq/refix/pull/50))
- add doc comments for dictionary and quickfix types ([#41](https://github.com/refixhq/refix/pull/41))
- add codecov badge ([#24](https://github.com/refixhq/refix/pull/24))
- add badges to readme ([#23](https://github.com/refixhq/refix/pull/23))
- add license files ([#22](https://github.com/refixhq/refix/pull/22))
