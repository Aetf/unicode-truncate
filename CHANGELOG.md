# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.0](https://github.com/Aetf/unicode-truncate/compare/v0.2.0...v1.0.0) - 2024-04-26

It's about time for a 1.0 release. This crate has been stable over the years.

### Breaking changes
Formally define MSRV (minimum supported rust version) to `1.63`.

### Added
- new method `unicode_truncate_centered`. Thanks @EdJoPaTo (#2, #3)

### Fixed
- prevent arithmetic side effects ([#7](https://github.com/Aetf/unicode-truncate/pull/7))
- do not include zero-width characters at boundaries when truncate_start

### Other
- use release-plz
- use renovate (#4)
- move from Travis to Github Action
- update Rust crate criterion to 0.5 ([#5](https://github.com/Aetf/unicode-truncate/pull/5))
- fix broken links
- Check in Cargo.lock per the latest guide
