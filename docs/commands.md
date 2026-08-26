# Command reference

Front CLI uses plain text only for `--help`, `--version`, and generated shell
completion. All other commands return JSON envelopes documented in
[Output format](output-format.md).

## Command catalog

Run `front` with no arguments to return the current command catalog and
structured parameter descriptions.

## Global profile option

`--profile <name>` selects a named config profile for `front config` and every
authenticated command. Clap accepts the global option before or after a
subcommand, including nested `api get` commands:

```bash
front --profile work list tags
front list tags --profile work
front api get /me --profile work
```

Explicit selection ignores ambient `FRONT_API_TOKEN`, `FRONT_USER`, and legacy
top-level credentials. Without the option, legacy sources take priority,
followed by `default_profile`, then automatic selection when exactly one
profile exists. See [Configuration](configuration.md) for the complete truth
table and redaction behavior.

Profile names must contain a non-whitespace character. Names are exact and are
not trimmed, so avoid leading or trailing spaces. An explicit empty or
whitespace-only argument returns a canonical JSON `CONFIG_ERROR` after clap has
accepted the argument value.

| Command | Description |
|---|---|
| `front` | Return available commands and parameters |
| `front config` | Show selected config metadata and redacted token-command state |
| `front doctor` | Run redacted authentication and read-scope diagnostics |
| `front inboxes` | List all accessible inboxes or those for `FRONT_USER` |
| `front inbox [inbox-id]` | Search conversations, optionally in one inbox |
| `front read <conversation-id>` | Read a compact conversation and its messages |
| `front whoami` | GET `/me` for the authenticated token |
| `front list <resource>` | List a supported top-level collection |
| `front get <resource> <id>` | Get a resource by Front or alternate ID |
| `front related <resource> <id> <relation>` | List an allowlisted nested resource |
| `front api get <path>` | GET a validated relative Core API path |
| `front completion <shell>` | Generate a completion script |

## Conversation triage

```bash
front inboxes
front inbox
front inbox inb_123 --limit 10
front read cnv_123
```

`front inbox` defaults to `is:open is:unassigned` and accepts:

| Flag | Meaning |
|---|---|
| `--query <query>` | Front conversation search syntax |
| `--from <handle>` | Append a sender filter |
| `--assignee <email>` | Append an assignee filter using `alt:email:` |
| `--before <YYYY-MM-DD>` | Append a UTC upper time bound |
| `--after <YYYY-MM-DD>` | Append a UTC lower time bound |
| `--limit <number>` | Maximum results (1 through 100); default 25 |

When `--assignee` is used without an explicit query, the default changes to
`is:open is:assigned`. An inbox ID adds `inbox:<id>` to the query.

`front read` requests the conversation and up to 25 messages. Each message body
is truncated at a valid UTF-8 boundary at or below 500 bytes.

## Redacted diagnostics

`front doctor` resolves configuration and the token once, then performs only
GET requests. It first calls `/me`; an authentication failure uses the normal
top-level failure envelope with a fixed, status-only CLI message. Front
response text is discarded. After authentication succeeds, it checks:

- `/tags?limit=1`;
- `/inboxes`;
- `/teammates`.

Each optional read check reports `ok`, `forbidden` for HTTP 403, or `error` for
any other failure. One failed optional check does not prevent the remaining
checks from running.

When an effective user is configured, the command also GETs
`/teammates/alt:email:<URL-encoded-user>` and compares its ID with `/me` only in
memory. `configured_user_matches_token` is the boolean `true` or `false` when
both IDs are available, `unavailable` when they cannot be compared, and
`not_configured` when there is no effective user.

Doctor output contains only fixed diagnostic strings, booleans, source names,
and HTTP status where needed for failure classification. It never serializes
the token, token-command arguments, effective user, resource IDs, response
bodies, Front-provided error messages, or customer data.

## Resource-oriented reads

```bash
front whoami
front list tags --limit 50
front get tag tag_123
front related tag tag_123 children --limit 50
front related conversation cnv_123 messages --limit 25
```

Resource names accept documented singular, plural, and short aliases. Not every
resource has an official top-level collection. Relations use a closed allowlist
per parent resource. See [API support](api-support.md) for the exact mappings.

`front list` accepts `--limit` and `--page-token` only when the resource's
official top-level endpoint documents those parameters. See the resource table
in [API support](api-support.md). Supplying either structured flag for an
unsupported collection exits with status 1 and an `INVALID_INPUT` envelope
whose command uses the canonical singular resource name; validation happens
before configuration or token resolution.

Supported collection commands and all relation commands accept:

- `--limit <number>` (1 through 100);
- `--page-token <token>`;

Every collection and relation command accepts repeatable `--param <key=value>`
values. This generic gateway remains available even for collection parameters
that do not have a structured flag.

`front get` accepts repeatable `--param <key=value>` values. Parameters split
on the first `=` and are URL encoded as structured query pairs.

### Generic output controls

`front list`, `front get`, `front related`, and `front api get` accept local
output controls. These flags do not apply to `front inbox`, `front inboxes`,
`front read`, or `front whoami`.

| Flag | Local behavior |
|---|---|
| `--count-only` | Omit `result.data` and report a collection's original `count` with `returned: 0` |
| `--keys-only` | Replace each object with its sorted top-level key names and omit object values |
| `--fields <a,b>` | Keep only the named literal top-level keys on each `_results` item or a single object |
| `--max-items <number>` | Keep at most this many decoded collection items; the value must be greater than zero |

`--count-only` is a standalone mode. `--keys-only` and `--fields` are mutually
exclusive. `--max-items` can be used alone or with `--keys-only` or `--fields`.
Incompatible combinations are rejected during CLI parsing, before token or
configuration resolution.

`--limit` and `--max-items` have different boundaries. `--limit` is sent to
Front as an upstream query parameter. `--max-items` never changes the request;
it truncates an already decoded JSON collection locally. When local truncation
occurs, `count` remains the number Front returned, `returned` is the number left
in `data`, and `truncated` is `true`.

Field names are literal. For example, `--fields id,metadata.name` selects keys
named `id` and `metadata.name`; it does not traverse a nested `metadata` object.
Missing fields are omitted.

## Universal JSON GET

Use the generic command when an official JSON GET endpoint has no shortcut:

```bash
front api get /company/statuses
front api get /conversations/cnv_123/events --limit 10
front api get /contacts --param q=alice --param limit=25
front api get /teammates/alt:email:user@example.com/inboxes
```

Its `--limit <number>` option also accepts integers from 1 through 100.

The path must start with exactly one `/`. Front CLI rejects:

- absolute URLs, schemes, and hosts;
- query strings or fragments embedded in the path;
- empty, `.` or `..` segments;
- control characters;
- any segment equal to `download`, ignoring case.

The production origin is fixed to `https://api2.frontapp.com`. The method is
always GET and callers cannot provide a request body.

## Pagination

When Front returns `_pagination.next` with a `page_token`, generic collection
output exposes it as `result.next_page_token` and in a structured
`next_actions` entry. The entry retains the command and every flag needed to
request the next page:

```bash
front list tag --limit 25 --param q=alice --param sort_by=created_at \
  --page-token "next-token"
```

In an action parameter, pass `value` once and repeat the flag once for each item
in `values`. A parameter with neither is an active boolean switch and should be
passed once without a following value. This preserves an explicitly supplied
`--profile`, the original limit, filters, arbitrary parameters, local output
controls, and replacement page token without reconstructing them from a URL.
Default or automatically selected profiles are not converted into an explicit
flag. The compact `inboxes`, `inbox`, and `read` navigation and refresh actions
use the same rule.

Structured `--limit` values stay structured, while a `limit=...` supplied with
`--param` remains a repeated passthrough value. For `api get` and resources
without structured page-token support, continuation replaces stale
passthrough page tokens with one `page_token=<new-token>` in `--param.values`.
Resources with structured pagination use `--page-token`. Generated
continuations remain valid input to the normal CLI parser. See
[Output format](output-format.md) for the complete action schema.

## Shell completion

Completion generation is local and does not require a token:

```bash
front completion bash > front.bash
front completion zsh > _front
front completion fish > front.fish
```

Supported values are `bash`, `elvish`, `fish`, `powershell`, and `zsh`.

## Read-only boundary

Front CLI cannot send or reply to messages, update conversations, create
comments or drafts, modify contacts or tags, import messages, validate channels,
or download attachments. Use `front api get` only for reads; it cannot bypass
this boundary.
