# Front CLI Rust Rewrite Implementation Plan

Date: 2026-08-25
Status: Completed implementation record

## Goal

Replace the former Go implementation with a Rust binary named `front` while
preserving the command-line interface, JSON envelopes, configuration trust
model, and GET-only Front API behavior defined in the
[approved design](../designs/2026-08-25-rust-rewrite-design.md).

## Implementation sequence

1. Establish Cargo metadata, a locked dependency graph, and black-box CLI
   contracts for root output, version output, configuration, and errors.
2. Implement the clap command model and common success and failure envelopes.
3. Implement OS-standard configuration loading, environment precedence, and
   argv-based `token_command` resolution using `SecretString`.
4. Implement the authenticated `reqwest` client and typed GET workflows for
   inboxes, conversation search, and compact conversation reading.
5. Add local mock-HTTP contracts for URL encoding, authorization, pagination,
   response mapping, and error classification.
6. Remove the former Go source only after the Rust compatibility contracts are
   green, then update contributor and architecture documentation.

## Verification gate

The migration is complete only when all of these commands succeed:

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets
cargo build --locked --release
```

The final review must also confirm that no Go module metadata remains, no
credential or customer data appears in tracked files, and the generic client
cannot issue a Front mutation.
