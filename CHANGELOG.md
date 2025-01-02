# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
 - New RDB version and Datatypes
 - Rust based integration tests
 - Fixtures for protocol and plain output
 - Redis integration test covering 6.2 - 7.4
 - Option to output to file
 - Error handling with thiserror
 - Support for new encoding types:
    - listpack
    - quicklist
    - sorted set v2
 - Python bindings with Maturin

### Changed
 - Ported CLI to clap
 - Encoding of non-ascii characters - previously escaped, resulting in possible duplicate json keys, now as hex string
 - Separated decoding and formatting logic

### Removed
 - Previous docs and build pipeline


---
# Previous:

### 0.2.1 - 2016-08-03

* Bug fix: Correctly handle skipping blobs
* Fix: Pin dependency versions

### 0.2.0 - 2015-04-04

* Make it work on Rust 1.0 beta

### 0.1.0 - 2015-03-23

Initial release with basic functionality
