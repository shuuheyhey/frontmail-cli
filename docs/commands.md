# Command reference

Front CLI uses plain text only for `--help`, `--version`, and generated shell
completion. All other commands return JSON envelopes documented in
[Output format](output-format.md).

## Command catalog

Run `front` with no arguments to return the current command catalog and
structured parameter descriptions.

| Command | Description |
|---|---|
| `front` | Return available commands and parameters |
| `front config` | Show config path, user, and redacted token-command state |
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
in `values`. This preserves the original limit, filters, arbitrary parameters,
and replacement page token without reconstructing them from a URL. See
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
