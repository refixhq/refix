# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.12.2](https://github.com/refixhq/refix/compare/refix-fix44-v0.12.1...refix-fix44-v0.12.2) - 2026-11-28

### Other

- updated the following local packages: refix-message

## [0.12.1](https://github.com/refixhq/refix/compare/refix-fix44-v0.12.0...refix-fix44-v0.12.1) - 2026-11-28

### Other

- updated the following local packages: refix-message

## [0.12.0](https://github.com/refixhq/refix/compare/refix-fix44-v0.11.0...refix-fix44-v0.12.0) - 2026-11-25

### Added

- *(codegen)* [**breaking**] write a Rust module tree with a file per message and component ([#204](https://github.com/refixhq/refix/pull/204))
- *(codegen)* [**breaking**] write a Python package with a module per message and component ([#203](https://github.com/refixhq/refix/pull/203))
- *(codegen)* generate a warm-up function in Rust ([#202](https://github.com/refixhq/refix/pull/202))
- *(codegen)* [**breaking**] keep known tags private in Rust ([#199](https://github.com/refixhq/refix/pull/199))
- *(codegen)* print the own fields of a header, trailer or group instance in Rust ([#195](https://github.com/refixhq/refix/pull/195))
- *(codegen)* [**breaking**] wrap received messages with from_raw in Python ([#193](https://github.com/refixhq/refix/pull/193))
- *(codegen)* [**breaking**] make a group's class its component in Python ([#192](https://github.com/refixhq/refix/pull/192))
- *(codegen)* [**breaking**] read component fields through their view in Python ([#191](https://github.com/refixhq/refix/pull/191))
- *(codegen)* generate a view for each component with fields in Python ([#190](https://github.com/refixhq/refix/pull/190))
- *(codegen)* [**breaking**] make a group's type its component in Rust ([#189](https://github.com/refixhq/refix/pull/189))
- *(codegen)* [**breaking**] read component fields through their view in Rust ([#188](https://github.com/refixhq/refix/pull/188))
- *(codegen)* [**breaking**] generate a view for each component with fields in Rust ([#187](https://github.com/refixhq/refix/pull/187))

### Fixed

- declare the refix-message version the stock crates need ([#207](https://github.com/refixhq/refix/pull/207))

### Other

- remove the generated files of the stock crates from git ([#205](https://github.com/refixhq/refix/pull/205))
- *(codegen)* build known tags from each component in Rust ([#201](https://github.com/refixhq/refix/pull/201))

## [0.11.0](https://github.com/refixhq/refix/compare/refix-fix44-v0.10.0...refix-fix44-v0.11.0) - 2026-11-21

### Added

- *(codegen)* [**breaking**] generate Python accessors as declarations ([#183](https://github.com/refixhq/refix/pull/183))

## [0.10.0](https://github.com/refixhq/refix/compare/refix-fix44-v0.9.0...refix-fix44-v0.10.0) - 2026-11-20

### Added

- *(codegen)* check the runtime version in generated Python ([#181](https://github.com/refixhq/refix/pull/181))

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
