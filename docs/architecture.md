# Architecture

Front CLI is a small Rust 2024 application with a process boundary, pure input
validation, a read-only HTTP client, command-specific mapping, and one shared
JSON envelope format.

## Request flow

```text
arguments
  -> clap parser (`src/cli.rs`)
  -> pure resource/path validation (`src/resources.rs`)
  -> config and secret resolution (`src/config.rs`)
  -> fixed-origin GET client (`src/client.rs`)
  -> compact or generic command mapping (`src/commands/`)
  -> JSON envelope (`src/envelope.rs`)
  -> stdout
```

Invalid generic input is rejected before the token is resolved. Shell
completion is generated locally and also bypasses configuration and HTTP.

## Source layout

| Path | Responsibility |
|---|---|
| `src/main.rs` | Process lifecycle, routing, exit codes, and error classification |
| `src/cli.rs` | Clap command tree and conversion to validated read requests |
| `src/resources.rs` | Resource aliases, collection paths, relation allowlists, path and query validation |
| `src/client.rs` | Authenticated typed GETs and the generic JSON GET boundary |
| `src/config.rs` | Config loading, environment precedence, and token-command execution |
| `src/commands/mod.rs` | Compact inbox and conversation workflows |
| `src/commands/read_api.rs` | Generic JSON envelopes and pagination actions |
| `src/models.rs` | Front response models used by compact workflows |
| `src/envelope.rs` | Stable success, failure, action, and parameter shapes |
| `tests/` | Black-box CLI, mock-HTTP, pure registry, and repository contract tests |

## HTTP boundary

Production requests always use `https://api2.frontapp.com`. The public client
constructor used by the binary does not accept an alternative origin. Tests
inject a local mock-server URL through a library constructor.

`FrontClient::get_value` accepts already validated path segments and structured
query pairs. URL construction percent-encodes each segment and query value. The
client exposes GET only and has no generic method or request-body parameter.
Reqwest automatic redirects remain disabled. The client manually replays only
HTTP 301 responses, for at most three hops, after validating the `Location`
origin and path. Every approved redirect is rebuilt as a path and query on the
configured base origin before it is requested, so a redirect target is never
accessed directly.

A redirect target may use the configured origin. When the configured base is
the production API, `https://api2.frontapp.com` and HTTPS subdomains of
`api.frontapp.com` with effective port 443 are also accepted as aliases, but
are still rebuilt on the configured production origin. HTTP 302 and other
statuses, missing or malformed locations, and targets with unsafe paths or
unapproved origins are returned as redirect responses rather than followed.

The validator rejects absolute URLs, embedded queries, fragments, empty or
traversal segments, controls, and `download` segments. Resource and relation
shortcuts use closed registries pinned in [API support](api-support.md).

## Configuration security

Tokens are resolved into `secrecy::SecretString`. Precedence is
`FRONT_API_TOKEN`, then the config file's `token_command`. Token commands are
argv arrays executed directly without an implicit shell. The value is exposed
only when building the bearer-authenticated request.

Malformed YAML is reported with the config path but without the parser's raw
message or source chain. This keeps token-command contents and other config
values out of display and debug output.

The CLI never writes configuration. See [Configuration](configuration.md) and
the accepted decision to
[make CLI config read-only](decisions/0002-read-only-cli-config.md).

## Output contracts

All machine-readable success and failure values go through
`envelope::success` or `envelope::failure`. Generic reads preserve the decoded
API JSON; compact workflow commands deliberately map into context-efficient
models. See [Output format](output-format.md).

## Testing strategy

- CLI contract tests spawn the release target interface and assert literal JSON
  behavior and exit codes.
- HTTP contract tests use local mock servers to verify paths, query pairs,
  headers, error bodies, and pagination without live Front credentials.
- Pure registry tests cover resource mappings and invalid inputs.
- Repository contract tests validate public files, YAML, documentation links,
  CI safety, and package metadata.
- CI runs the same format, Clippy, test, and release-build gate on Linux,
  macOS, and Windows with Rust 1.88.0.

The full local quality gate is documented in [CONTRIBUTING.md](../CONTRIBUTING.md).

## Architecture decisions

Long-lived decisions are recorded in [Architecture Decision Records](decisions/README.md):

- read-only config and token-command security;
- the Rust rewrite and handwritten client;
- declarative read shortcuts plus a safe generic GET gateway.

Historical design specifications and implementation plans are retained under
`docs/development/` for maintainers; they are not user documentation.
