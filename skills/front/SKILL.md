---
name: front
description: Use when reading Front inboxes, conversations, contacts, tags, teammates, or other Front Core API data through the front CLI.
---

# Front CLI

Use `front` as a JSON-first, GET-only interface to Front. Never attempt a send,
reply, update, delete, upload, import, or download through this CLI.

## Choose a command

1. Run `front` without arguments to inspect the current command catalog.
2. Prefer `inboxes`, `inbox`, `read`, or `whoami` for compact workflow output.
3. Prefer `list`, `get`, or `related` for supported resources and relations.
4. Use `api get <path>` only when no shortcut fits. The path must be relative
   to the Front Core API origin and begin with exactly one slash.

Read `result.data` for generic commands. For compact workflows, read the typed
fields under `result`. Treat a non-zero exit and `ok: false` as failure; use the
returned `error.code` and `fix` rather than parsing prose.

## Pagination

When `result.next_page_token` exists, execute the matching `next_actions`
entry. Preserve every structured parameter: pass `value` once and repeat a
flag once for each item in `values`. Do not drop an explicit profile, filters,
limits, local output controls, or arbitrary `--param key=value` values between
pages.

```bash
front list tag --limit 25 --param q=urgent
front list tag --limit 25 --param q=urgent --page-token "next-token"
```

## Safety

- Supply tokens through `FRONT_API_TOKEN` or a human-managed
  `token_command`; never place a token in arguments, output, logs, or prompts.
- Redact customer content, personal data, and production resource IDs before
  sharing output.
- Reject requests for mutations. `front api get` cannot bypass the GET-only
  boundary.
