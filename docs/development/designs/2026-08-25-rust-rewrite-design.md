# Front CLI Rust Rewrite Design

## Status

Approved in chat on 2026-08-25 when the user asked to execute the recommended Rust rewrite.

## Goal

Replace the Go implementation of `front` with a Rust implementation while preserving the existing command-line interface, JSON envelope contract, configuration trust model, and read-only Front API behavior.

## Success criteria

- The produced executable is still named `front`.
- `front`, `front config`, `front inboxes`, `front inbox [inbox-id]`, and `front read <conversation-id>` preserve their current user-visible behavior.
- Existing flags, defaults, positional arguments, JSON keys, omission rules, error codes, fixes, and `next_actions` remain compatible.
- The Rust implementation sends only the same five read-only Front API requests used by the Go implementation.
- `FRONT_API_TOKEN` and `FRONT_USER` keep precedence over the config file.
- `token_command` remains argv-based, is executed without a shell, and is never printed by `front config`.
- Unit, CLI, and HTTP contract tests cover success, malformed input, transport errors, API errors, pagination, authentication, and response mapping.
- `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all-targets --all-features`, and `cargo build --release` all succeed.
- The final repository contains no Go implementation or Go module metadata.

## Non-goals

- Adding Front API write operations.
- Changing the command names, flags, default search query, output schema, or configuration file location.
- Replacing the deprecated teammate inbox endpoint during the language migration.
- Adding retries beyond the HTTP behavior already provided by the Go client.
- Storing API tokens in the repository, command history, logs, tests, or fixtures.
- Publishing a release, pushing a branch, or changing live Front data.

## Approaches considered

### 1. Handwritten typed client for the five operations — selected

Define only the request parameters and response fields consumed by the CLI. Use `reqwest` for HTTP and Serde types for JSON. This keeps the compiled surface and maintenance burden aligned with the actual product.

### 2. Generate a Rust client from the complete Front OpenAPI document

This preserves schema generation but introduces hundreds of unused operations and additional generator/tooling maintenance. Selective generation is possible, but it adds another compatibility layer without improving the four-command CLI.

### 3. Keep Go and add a separate Rust binary

This lowers cutover risk but creates two implementations and an unclear source of truth. The migration will temporarily build both implementations for parity testing, then remove Go before completion.

## Compatibility contract

### Commands

| Invocation | Behavior |
|---|---|
| `front` | Emit a successful JSON envelope with version and actions for all leaf commands. |
| `front --version` | Print the build version followed by a newline. |
| `front config` | Report config path, whether `token_command` is configured, and the configured user without resolving or exposing the token command. |
| `front inboxes` | List company inboxes, or teammate inboxes when `FRONT_USER`/configured user is set. |
| `front inbox [inbox-id]` | Search conversations with the existing query composition, filters, pagination, and limit behavior. |
| `front read <conversation-id>` | Fetch the conversation and first message page concurrently and emit mapped, truncated plain text. |

### Flags and defaults

- `front inbox --query`: default `is:open is:unassigned`.
- `front inbox --from`: append `from:<handle>`.
- `front inbox --assignee`: append `assignee:alt:email:<email>` and change the untouched default query to `is:open is:assigned`.
- `front inbox --before` and `--after`: accept only `YYYY-MM-DD` and convert midnight UTC to Unix seconds.
- `front inbox --limit`: default `25`.
- `front inbox --page-token`: accepted but hidden from generated `next_actions` parameter documentation unless used for pagination.
- `front read` accepts exactly one conversation ID and has no command-specific flags.

### Environment and config

- Token resolution: non-empty `FRONT_API_TOKEN`, then configured `token_command`, then `UNAUTHORIZED`.
- User resolution: non-empty `FRONT_USER`, then configured `user`, then no teammate scope.
- Config file: `<OS config directory>/front/config.yaml`, matching Go's `os.UserConfigDir` behavior.
- YAML schema:

  ```yaml
  token_command:
    - executable
    - argument
  user: user@example.com
  ```

- `token_command` is passed directly to `std::process::Command`; shell parsing is forbidden.
- The token is wrapped in `secrecy::SecretString` and exposed only while setting the Authorization header.

### HTTP

- Base URL: `https://api2.frontapp.com`.
- Total request timeout: 30 seconds.
- Headers: `Authorization: Bearer <token>` and `User-Agent: front-cli/<version>`.
- Operations:
  - `GET /conversations/search/{query}`
  - `GET /conversations/{conversation_id}`
  - `GET /conversations/{conversation_id}/messages`
  - `GET /inboxes`
  - `GET /teammates/{teammate_id}/inboxes`
- Path segments and query parameters are URL encoded by the URL/request library.
- The deprecated teammate inbox operation stays unchanged for parity. Replacing it requires a separate ADR and live result comparison.

### JSON envelopes

- Success fields stay ordered as `ok`, `command`, `result`, `next_actions`.
- Error fields stay ordered as `ok`, `command`, `error`, `fix`, `next_actions`.
- Output is two-space pretty-printed JSON followed by a newline.
- Optional values use the same omission rules as Go's `omitempty` tags.
- HTTP status mapping remains:
  - `401` → `UNAUTHORIZED`
  - `403` → `FORBIDDEN`
  - `404` → `NOT_FOUND`
  - `429` → `RATE_LIMITED`
  - other non-success status → `API_ERROR`
- Transport failures use `TRANSPORT_ERROR`, invalid dates use `INVALID_INPUT`, config parsing uses `CONFIG_ERROR`, and CLI usage errors use `CLI_ERROR`.

## Architecture

```text
Cargo.toml
src/
├── main.rs              process entry, signal/runtime setup, exit status
├── lib.rs               testable crate surface and command dispatcher
├── cli.rs               clap command tree and parsed invocation types
├── client.rs            authenticated reqwest client and five endpoints
├── config.rs            config path/loading and argv token resolution
├── envelope.rs          success/error envelopes and next actions
├── error.rs             typed failures and public error classification
├── models.rs            Front response subsets and public output models
└── commands/
    ├── mod.rs
    ├── config.rs
    ├── inboxes.rs
    ├── inbox.rs
    └── read.rs
tests/
├── cli_contract.rs      black-box command and JSON compatibility tests
├── http_contract.rs     wiremock request/response/error tests
└── fixtures/            complete, synthetic Front response documents
```

### CLI layer

Use the clap builder API as the single source of command metadata. The same metadata drives parsing and `next_actions`, avoiding duplicated descriptions/defaults. Parsed `ArgMatches` are converted into typed invocation structs before command execution.

### Client layer

`FrontClient` owns a reusable `reqwest::Client`, base URL, and `SecretString`. Production construction uses the fixed Front base URL; a `#[doc(hidden)]` public constructor lets integration tests inject a wiremock URL. The client returns typed response bodies on success and a typed status/body error otherwise.

### Command layer

Each command receives typed arguments plus the small interfaces it needs. Search query construction and Front-to-output mapping remain pure functions. `read` uses `tokio::try_join!` so the conversation and message requests retain the current concurrency.

### Error layer

Internal errors use `thiserror`. A single boundary converts errors to the public envelope taxonomy and determines exit code `0` or `1`. No logic depends on clap's human-readable error strings.

## Dependencies

### Runtime

- Rust edition 2024.
- `clap` with builder/string features for CLI parsing and metadata.
- `tokio` for async runtime, signals, and concurrent requests.
- `reqwest` with JSON and rustls TLS; default native TLS is disabled.
- `serde` and `serde_json` for API and envelope JSON.
- `serde_yaml_ng` for the existing YAML config format.
- `directories` for the OS config directory.
- `chrono` for UTC date parsing and timestamp formatting.
- `url` for pagination URL parsing.
- `thiserror` for typed internal errors.
- `secrecy` for API token handling.

### Development

- `assert_cmd` for black-box binary tests.
- `wiremock` for Front HTTP contract tests.
- `tempfile` for isolated config fixtures.
- `predicates` only where process-output assertions need it.

Dependencies are pinned through `Cargo.lock`, which is committed because this repository ships an application.

## Testing strategy

### Golden compatibility

Before deleting Go, capture literal expected JSON for the root, config, invalid arguments, search query construction, mapped conversations/messages, pagination actions, and API error envelopes. Rust tests assert semantic JSON equality plus exact omission behavior. Exact text assertions are limited to the public `--version` and JSON formatting contracts.

### Unit tests

- Search query composition for every flag combination and invalid dates.
- Conversation/message mapping, UTC formatting, HTML fallback, and 500-byte truncation behavior.
- Error classification and Front `_error.message` parsing.
- Pagination token extraction.
- Config path/loading, missing file, invalid YAML, token command success/failure/empty output, and arguments containing spaces.
- Command metadata and generated next actions.

### HTTP contract tests

Wiremock receives real requests from `FrontClient`. Tests verify observable behavior: method, encoded path/query, authentication, user agent, response mapping, API status mapping, malformed JSON, and the two requests used by `read`. Synthetic fixtures contain all documented fields used by the corresponding response type.

### Migration verification

- Run the original Go tests before deleting Go.
- Run Rust tests after every TDD slice.
- Compare Go and Rust outputs for synthetic equivalent inputs.
- Run `cargo fmt --check`, strict Clippy, all tests, debug build, and release build.
- Run `front --help`, `front`, and `front config` smoke checks without credentials.
- An optional live read-only Front smoke test requires explicit use of a freshly rotated token and must never persist the token.

## Migration sequence

1. Add Cargo metadata and the first failing CLI contract test.
2. Implement CLI parsing and root envelope through red/green/refactor cycles.
3. Implement envelope, config, and token resolution through unit tests.
4. Implement the five-operation client and complete HTTP fixtures through contract tests.
5. Implement `inboxes`, `inbox`, and `read`, preserving next actions and mappings.
6. Update Makefile, README, AGENTS.md, `.gitignore`, and add an accepted Rust rewrite ADR.
7. Remove Go source, generated client, OpenAPI snapshot, `go.mod`, and `go.sum` only after Rust parity tests pass.
8. Run the complete verification matrix and review the final diff for secrets and unintended changes.

## Documentation changes

- README installation changes from `go install` to Cargo-based source installation and release binary build instructions.
- AGENTS.md describes Cargo commands and Rust module boundaries.
- Makefile maps `build`, `test`, and `lint` to Cargo and removes Go generation.
- Add `docs/decisions/0003-rewrite-cli-in-rust.md` recording the accepted language/client strategy.
- Keep `skills/front/SKILL.md` command usage unchanged because the external CLI contract is unchanged.

## Risks and mitigations

| Risk | Mitigation |
|---|---|
| JSON fields or omissions drift | Literal golden fixtures and semantic JSON contract tests. |
| Clap usage errors differ from Cobra | Convert typed parse errors into the existing JSON taxonomy at the process boundary. |
| Partial Front response models reject valid data | Use Serde's default unknown-field tolerance and complete synthetic fixtures. |
| Token leaks through diagnostics | `SecretString`, no token serialization, no debug formatting, synthetic test credentials only. |
| Deprecated teammate endpoint changes behavior | Preserve it for parity and isolate replacement as a later decision. |
| Big-bang deletion loses a reference | Keep Go buildable until Rust parity tests pass; delete Go only in the final migration task. |

## Rollback

The Rust rewrite remains on its feature branch until verification and review. Before merge, rollback is deleting the feature branch. After merge, rollback is reverting the rewrite commit, restoring the Go implementation and its module metadata without data migration or external state changes.
