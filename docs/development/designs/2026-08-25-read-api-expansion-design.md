# Front CLI Read API Expansion Design

Date: 2026-08-25
Status: Approved (the user pre-approved all decisions and requested uninterrupted execution)

## Objective

Bring the Rust CLI documentation up to date and close the useful read-feature
gap identified from two primary sources:

- Front's official Core API OpenAPI repository at commit
  `dc76701bcf90f5b98dd56e76cae438243f63b94d` (published 2026-08-24), which
  defines 147 paths and 123 GET operations.
- `dedene/frontmail-cli` at commit
  `6333c681808569c0d91b63e85f5b62c9cd397b45`, which demonstrates a broad,
  script-friendly resource-oriented CLI.

The existing `front`, `front config`, `front inboxes`, `front inbox`, and
`front read` contracts remain compatible.

## Scope

### Included

1. `front whoami` for `/me`.
2. `front api get <path>` as a safe, same-origin, GET-only escape hatch for
   JSON endpoints in the current and future Front Core API.
3. Declarative resource shortcuts:
   - `front list <resource>` for top-level collection endpoints;
   - `front get <resource> <id>` for item endpoints;
   - `front related <resource> <id> <relation>` for nested collections.
4. Pagination inputs (`--limit`, `--page-token`) and repeatable arbitrary query
   parameters (`--param key=value`) without shell parsing.
5. `front completion <bash|zsh|fish|elvish|powershell>`.
6. Updated root `next_actions`, README, agent skill, architecture guidance,
   and an API support matrix pinned to the official spec revision.
7. Local mock-HTTP contract coverage for path encoding, query construction,
   raw JSON preservation, pagination, error classification, and secrets.

### Excluded

- POST, PATCH, PUT, DELETE, message send/reply, assignment, archive/trash,
  comment creation, draft mutations, contact mutations, tag mutations, and
  attachment writes.
- OAuth setup, keyring storage, multiple-account token refresh, retries, and
  circuit breakers.
- Binary download endpoints, because the CLI's stdout contract is JSON and an
  accidental download could write large or sensitive data.

Mutation support requires a separate design with explicit confirmation
semantics. This project remains read-only.

## Alternatives

### A. Generate the full Rust client

This maximizes compile-time typing but restores the large generated surface the
Rust rewrite intentionally removed. It also couples releases to generator
output for endpoints the CLI does not use directly.

### B. Curated typed command per official GET operation

This gives the richest help but means maintaining more than one hundred command
handlers and response models. Most handlers would only forward path and query
parameters.

### C. Declarative shortcuts plus a validated GET gateway (selected)

Typed legacy workflows stay compact, common resource reads get discoverable
shortcuts, and every JSON GET remains accessible through one carefully
validated boundary. This provides the broadest useful coverage with the
smallest auditable implementation.

## CLI Contract

### Universal GET

```text
front api get /company/statuses
front api get /conversations/cnv_123/events --limit 10
front api get /teammates/alt:email:user@example.com/inboxes
front api get /contacts --param q=alice --param limit=25
```

Rules:

- `path` must start with exactly one `/`.
- Absolute URLs, schemes, hosts, fragments, `.` segments, and `..` segments are
  rejected.
- The client reconstructs the URL from the fixed Front base URL and individually
  percent-encodes path segments.
- User query inputs are parsed only as `key=value` pairs.
- The method is permanently GET; callers cannot supply a method or request body.
- Download operation paths containing a `download` segment are rejected.

### Resource shortcuts

```text
front list tags --limit 50
front get tag tag_123
front related tag tag_123 children --limit 50
front related conversation cnv_123 comments --limit 25
```

Singular resource names map to official path segments through a closed
registry. The registry includes accounts, channels, comments, contacts,
conversations, events, inboxes, knowledge bases/articles/categories, links,
message templates/folders, messages, rules, shifts, signatures, tags,
teammate groups, teammates, teams, time offs, and views.

Relations use an allowlist keyed by parent resource. An unsupported
resource/relation returns `INVALID_INPUT` before authentication or HTTP.

### Output

All new commands use the existing envelope:

```json
{
  "ok": true,
  "command": "front list tags",
  "result": {
    "data": {"_results": []},
    "count": 0
  },
  "next_actions": []
}
```

`data` preserves the API JSON without lossy models. Collection responses also
include `count` and, when present, `next_page_token`. Pagination adds a
`next_actions` entry carrying the token as a structured parameter.

## Architecture

### `src/client.rs`

Add a public `get_value` operation that accepts validated path segments and
query pairs, reuses bearer authentication, timeout, error-body parsing, and the
fixed base URL, and decodes into `serde_json::Value`.

### `src/resources.rs`

Own the resource registry, relation allowlists, aliases, path validation, and
query-pair parsing. This module contains no I/O.

### `src/commands/read_api.rs`

Own `whoami`, universal GET, list/get/related execution, generic result
envelopes, count extraction, page-token extraction, and pagination actions.

### `src/cli.rs`

Move clap declarations out of `main.rs`. Export parser structures and
conversion helpers so command routing can be contract-tested without spawning
live HTTP calls.

### `src/main.rs`

Remain the process boundary: parse arguments, resolve config/token, build the
production client, route commands, print JSON, and select exit codes.

## Error Handling

- Invalid paths, resources, relations, or query pairs: `INVALID_INPUT`, exit 1.
- Missing token: `UNAUTHORIZED`, exit 1.
- HTTP status errors: existing `UNAUTHORIZED`, `FORBIDDEN`, `NOT_FOUND`,
  `RATE_LIMITED`, or `API_ERROR` classifications.
- JSON decode errors: `API_ERROR`.
- Transport errors: `TRANSPORT_ERROR`.
- Secrets never appear in paths, query parameters, debug output, fixtures,
  error strings, docs, or diffs.

## Testing

Every behavior is developed red-green-refactor:

1. Pure registry tests for all resource and relation mappings.
2. CLI parser contract tests for new commands and rejected combinations.
3. Mock-HTTP tests for encoded IDs, aliases, repeated parameters, pagination,
   auth headers, error bodies, and download rejection.
4. Envelope tests for object and list responses.
5. Existing compatibility tests remain unchanged and green.
6. Final `cargo fmt`, strict clippy, full tests, release build, smoke, secret
   scan, stale-doc scan, and `git diff --check` run on the feature branch and
   again after local integration into `develop`.

Live Front checks are read-only and may be run only with a token supplied via
non-echoing standard input; no token is persisted.

## Documentation

- README becomes the canonical user guide for legacy and expanded commands.
- `docs/api-support.md` records the official spec commit/date, supported
  command shapes, resource registry, relation registry, and deliberate gaps.
- `skills/front/SKILL.md` teaches agents to prefer curated shortcuts, fall back
  to `api get`, preserve pagination parameters, and never attempt mutations.
- ADR 0004 records the GET-only declarative coverage decision.
