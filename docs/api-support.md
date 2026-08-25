# Front API support

Last reviewed: 2026-08-25

[Documentation index](README.md) · [Command reference](commands.md) ·
[Architecture](architecture.md)

## Source revisions

This matrix is pinned to the official
[Front Core API specification](https://github.com/frontapp/front-api-specs/tree/dc76701bcf90f5b98dd56e76cae438243f63b94d)
at commit `dc76701bcf90f5b98dd56e76cae438243f63b94d`, published
2026-08-24. That document contains 147 paths and 123 GET operations.

The command-shape survey also reviewed
[dedene/frontmail-cli](https://github.com/dedene/frontmail-cli/tree/6333c681808569c0d91b63e85f5b62c9cd397b45)
at commit `6333c681808569c0d91b63e85f5b62c9cd397b45`. Its broader
resource-oriented design informed the shortcuts below; its write operations,
OAuth flow, keyring integration, and multiple-account support were not copied.

## Coverage model

The CLI has three read layers:

1. `front inboxes`, `front inbox`, and `front read` provide compact,
   conversation-focused output compatible with the original CLI.
2. `front list`, `front get`, and `front related` provide discoverable
   shortcuts backed by a closed resource and relation registry.
3. `front api get` exposes other JSON GET operations through a validated,
   fixed-origin gateway.

The third layer means new or uncommon official JSON GET paths do not require a
new release. It is intentionally not an arbitrary HTTP client: the host and
method cannot be changed, request bodies do not exist, and binary download
paths are rejected.

## Resource shortcuts

All names accept singular and plural forms. Underscores are normalized to
hyphens. `conv`, `msg`, `template`, and `template-folder` are also accepted
aliases.

| Resource | Official segment | `list` | `get` |
|---|---|---:|---:|
| `account` | `accounts` | yes | yes |
| `channel` | `channels` | yes | yes |
| `comment` | `comments` | no | yes |
| `contact` | `contacts` | yes | yes |
| `conversation` | `conversations` | yes | yes |
| `event` | `events` | yes | yes |
| `inbox` | `inboxes` | yes | yes |
| `knowledge-base` | `knowledge_bases` | yes | yes |
| `knowledge-base-article` | `knowledge_base_articles` | no | yes |
| `knowledge-base-category` | `knowledge_base_categories` | no | yes |
| `link` | `links` | yes | yes |
| `message` | `messages` | no | yes |
| `message-template` | `message_templates` | yes | yes |
| `message-template-folder` | `message_template_folders` | yes | yes |
| `rule` | `rules` | yes | yes |
| `shift` | `shifts` | yes | yes |
| `signature` | `signatures` | no | yes |
| `tag` | `tags` | yes | yes |
| `teammate` | `teammates` | yes | yes |
| `teammate-group` | `teammate_groups` | yes | yes |
| `team` | `teams` | yes | yes |
| `time-off` | `time_offs` | no | yes |
| `view` | `views` | yes | yes |

`list` is unavailable when the official specification has no top-level GET
collection. Those resources remain available through `get` and parent
relations.

## Relation shortcuts

| Parent resource | Allowed relations |
|---|---|
| `account` | `contacts` |
| `comment` | `mentions` |
| `contact` | `conversations`, `notes` |
| `conversation` | `comments`, `drafts`, `events`, `followers`, `inboxes`, `messages` |
| `inbox` | `channels`, `conversations`, `teammates` |
| `knowledge-base` | `articles`, `categories`, `content` |
| `knowledge-base-article` | `content` |
| `knowledge-base-category` | `articles`, `content` |
| `link` | `conversations` |
| `message` | `seen` |
| `message-template-folder` | `message_template_folders`, `message_templates` |
| `shift` | `teammates` |
| `tag` | `children`, `conversations` |
| `teammate` | `channels`, `contact_groups`, `contact_lists`, `contacts`, `conversations`, `inboxes`, `message_template_folders`, `message_templates`, `private_inboxes`, `rules`, `shifts`, `signatures`, `tags`, `time_offs` |
| `teammate-group` | `inboxes`, `teammates`, `teams` |
| `team` | `channels`, `contact_groups`, `contact_lists`, `contacts`, `inboxes`, `message_template_folders`, `message_templates`, `rules`, `shifts`, `signatures`, `tags`, `time_offs`, `views` |

The aliases `convos`, `folders`, and `templates` normalize to the corresponding
official relation segment. Resources not shown in this table have no
allowlisted GET relation in the pinned specification.

Locale-specific knowledge-base paths, custom-field schemas, company endpoints,
conversation search, and other GET shapes remain accessible with
`front api get`.

## Query and pagination contract

```text
front list contacts --limit 25 --param q=alice
front related inbox inb_123 conversations --page-token TOKEN
front api get /accounts/custom_fields --param limit=50
```

- `--param` is repeatable and splits on the first `=` only.
- `--limit` and `--page-token` are appended as structured query pairs.
- The entire API response is retained under `result.data`.
- When `_results` is an array, `result.count` reports the number returned.
- When `_pagination.next` contains a page token, the token is exposed in the
  result and in a structured `next_actions` parameter.

## Deliberate gaps

The client exposes no POST, PUT, PATCH, or DELETE operation. Unsupported
capabilities include message send/reply, conversation mutation, comment and
draft creation, contact/tag changes, imports, channel validation, and every
other write.

Paths containing a segment equal to `download` are rejected even when the
official method is GET. This keeps stdout JSON-only and avoids accidentally
streaming large or sensitive attachments. OAuth setup, keyring storage,
multiple accounts, retries, and circuit breaking are also outside the current
scope.
