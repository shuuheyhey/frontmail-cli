# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial Rust implementation of the read-only Front API CLI.
- JSON envelopes with actionable `next_actions` metadata.
- Typed inbox and conversation workflows plus safe generic GET commands.
- Contract tests, contributor documentation, and read-only CI checks.

### Changed

- Aligned command, configuration, pagination, architecture, and security
  documentation with the Rust implementation.
- Refreshed the API support review against the latest official Front Core API
  specification revision.

### Fixed

- Follow up to three validated HTTP 301 redirects while preserving the
  configured Front API origin.
- Use a message's `from` recipient as the sender when its author is absent.
- Report the requested command for authentication and configuration errors.

[Unreleased]: https://github.com/shuuheyhey/frontmail-cli/commits/develop
