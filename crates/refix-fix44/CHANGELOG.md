# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.9.0](https://github.com/refixhq/refix/compare/refix-fix44-v0.8.0...refix-fix44-v0.9.0) - 2026-11-17

### Other

- tidy the Rust example in the refix-fix44 README ([#169](https://github.com/refixhq/refix/pull/169))

## [0.8.0](https://github.com/refixhq/refix/compare/refix-fix44-v0.7.0...refix-fix44-v0.8.0) - 2026-11-16

### Other

- say FIX protocol in the package descriptions ([#158](https://github.com/refixhq/refix/pull/158))

## [0.7.0](https://github.com/refixhq/refix/compare/refix-fix44-v0.6.0...refix-fix44-v0.7.0) - 2026-11-16

### Added

- document the generated Rust code ([#152](https://github.com/refixhq/refix/pull/152))
- make TokenizeError a standard error and print generated messages ([#151](https://github.com/refixhq/refix/pull/151))

## [0.6.0](https://github.com/refixhq/refix/compare/refix-fix44-v0.5.0...refix-fix44-v0.6.0) - 2026-11-16

### Added

- add repr to the Python messages ([#140](https://github.com/refixhq/refix/pull/140))
- [**breaking**] name group accessors after the count field ([#138](https://github.com/refixhq/refix/pull/138))

### Other

- point the packages at refix.dev ([#148](https://github.com/refixhq/refix/pull/148))
- declare the minimum Rust version ([#145](https://github.com/refixhq/refix/pull/145))
- document the refix crate ([#142](https://github.com/refixhq/refix/pull/142))
- add docstrings to the Python classes ([#141](https://github.com/refixhq/refix/pull/141))

## [0.5.0](https://github.com/refixhq/refix/compare/refix-fix44-v0.4.0...refix-fix44-v0.5.0) - 2026-11-02

### Added

- decode logs through a generated codec ([#122](https://github.com/refixhq/refix/pull/122))
- generate a dialect constant ([#121](https://github.com/refixhq/refix/pull/121))
- read each message as its type in generated code ([#118](https://github.com/refixhq/refix/pull/118))

## [0.4.0](https://github.com/refixhq/refix/compare/refix-fix44-v0.3.0...refix-fix44-v0.4.0) - 2026-11-01

### Added

- read DATA as bytes and warn on uninterpreted types in generated code ([#108](https://github.com/refixhq/refix/pull/108))
- read times and timestamps in generated code ([#107](https://github.com/refixhq/refix/pull/107))
- read BOOLEAN fields as bool in generated code ([#106](https://github.com/refixhq/refix/pull/106))
- read string subtypes, dates and MonthYear in generated code ([#105](https://github.com/refixhq/refix/pull/105))
- read float fields as decimals in generated code ([#104](https://github.com/refixhq/refix/pull/104))

### Changed

- fields read as their types, not raw bytes, so 278 accessors lose their `_raw` suffix in Rust and Python: `price_raw` is now `price`, returning a `Decimal`, and `yield_raw` is `r#yield` in Rust and `yield_` in Python
- the 24 BOOLEAN fields that list `Y` and `N`, such as `poss_dup_flag`, read as `bool`

### Removed

- the enums of those 24 fields: `AggregatedBook`, `DueToRelated`, `ExchangeForPhysical`, `ForexReq`, `GapFillFlag`, `IOINaturalFlag`, `InViewOfCommon`, `LastFragment`, `LegalConfirm`, `LocateReqd`, `MDImplicitDelete`, `NotifyBrokerOfCredit`, `OddLot`, `PossDupFlag`, `PossResend`, `PreviouslyReported`, `PublishTrdIndicator`, `ReportToExch`, `ResetSeqNumFlag`, `SolicitedFlag`, `TestMessageIndicator`, `TradedFlatSwitch`, `UnsolicitedIndicator`, `WorkingIndicator`

## [0.3.0](https://github.com/refixhq/refix/compare/refix-fix44-v0.2.0...refix-fix44-v0.3.0) - 2026-10-11

### Added

- publish the Python package as refix ([#89](https://github.com/refixhq/refix/pull/89))

## [0.2.0](https://github.com/refixhq/refix/compare/refix-fix44-v0.1.0...refix-fix44-v0.2.0) - 2026-10-10

### Other

- re-release dependents only on breaking runtime releases ([#83](https://github.com/refixhq/refix/pull/83))

## [0.1.0](https://github.com/refixhq/refix/releases/tag/refix-fix44-v0.1.0) - 2026-10-08

### Added

- add the stock FIX44 packages ([#75](https://github.com/refixhq/refix/pull/75))

### Other

- automate releases with release-plz ([#76](https://github.com/refixhq/refix/pull/76))
