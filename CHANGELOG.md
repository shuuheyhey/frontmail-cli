# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial Rust implementation of the read-only Front API CLI.
- JSON envelopes with actionable `next_actions` metadata.
- Typed inbox and conversation workflows plus safe generic GET commands.
- Bounded and projected output controls for generic resource and API reads.
- Contract tests, contributor documentation, and read-only CI checks.
- Effective configuration source reporting in `front config` without resolving
  or exposing token commands.
- Redacted `front doctor` authentication, configured-user, and optional
  read-scope diagnostics using GET requests only.
- Named configuration profiles with explicit, default, and one-profile
  automatic selection plus ambient credential isolation.

### Changed

- Aligned command, configuration, pagination, architecture, and security
  documentation with the Rust implementation.
- Refreshed the API support review against the latest official Front Core API
  specification revision.

### Fixed

- Reject structured pagination flags before authentication when a top-level
  resource does not document the corresponding query parameter.
- Reject collection limits outside the inclusive range of 1 through 100.
- Keep HTTP 401 remediation scoped to the selected named profile, while
  preserving legacy token guidance, and reject empty or whitespace-only
  profile keys and selectors.
- Preserve an explicit named profile in every authenticated compact and generic
  continuation, navigation, and refresh action while keeping implicit profile
  selection hidden.
- Preserve structured versus passthrough pagination origins so continuation
  actions remain parseable, replace stale page tokens once, and retain repeated
  parameter order and values.
- Follow up to three validated HTTP 301 redirects while preserving the
  configured Front API origin.
- Use a message's `from` recipient as the sender when its author is absent.
- Report the requested command for authentication and configuration errors.
- Redact Front-provided response text from `front doctor` authentication
  failures and keep encoded teammate-alias checks free of query components.

[Unreleased]: https://github.com/shuuheyhey/frontmail-cli/commits/develop
