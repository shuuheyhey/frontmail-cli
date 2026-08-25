---
status: accepted
date: 2026-08-25
decision-makers: maintainers
---

# Rewrite the Front CLI in Rust

## Context and Problem Statement

The original CLI was implemented in Go with Cobra and a generated client for
the complete Front OpenAPI document. The generated client and checked-in API
specification were much larger than the command code even though the CLI uses
only five read-only endpoints. The project also needs a single, portable binary
with explicit secret handling and stable agent-facing JSON contracts.

## Decision

Replace the Go implementation with Rust while preserving the `front` binary,
commands, flags, configuration precedence, JSON envelopes, and `next_actions`.

Use:

- `clap` for argument parsing;
- `tokio` and `reqwest` with rustls for HTTP;
- `serde` for Front responses and CLI output;
- `secrecy::SecretString` for resolved API tokens;
- a handwritten typed client, initially limited to the five GET endpoints the
  legacy CLI calls.

The production API base URL remains fixed at `https://api2.frontapp.com`.
Tests inject a local mock-server URL through the library constructor, not a
user-facing flag or environment variable. Token commands remain argv arrays and
are executed directly without an implicit shell.

## Consequences

- Good, because the maintained API surface is small and auditable.
- Good, because release builds remain self-contained binaries without OpenSSL.
- Good, because token values are held in a redacting secret type.
- Good, because mock-HTTP tests verify paths, query parameters, authentication,
  error bodies, pagination, and output mapping without using live Front data.
- Good, because JSON compatibility is guarded by black-box contract tests.
- Bad, because API schema changes must be reflected manually in the small set
  of response models.
- Bad, because contributors need the Rust toolchain instead of Go.

One intentional safety correction is included: message truncation stops at a
valid UTF-8 boundary at or below 500 bytes. The old byte slice could split a
multi-byte character and emit invalid text.

## Alternatives Considered

- Keep Go and the generated client: lowest migration cost, but retains the
  oversized generated surface and does not meet the Rust rewrite objective.
- Generate a full Rust OpenAPI client: type-safe, but recreates the maintenance
  and compile-time cost for endpoints the CLI never uses.
- Use dynamic JSON values throughout: smaller upfront model work, but weaker
  guarantees for the stable output contract.

This decision's initial read-coverage limit was expanded by
[ADR 0004](0004-expand-read-api-coverage.md), while keeping the client
handwritten, same-origin, and read-only.
