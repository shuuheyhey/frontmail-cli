# Output format

Front CLI is designed for scripts and agents. API commands return one JSON
document to standard output and use a stable top-level envelope.

`--help`, `--version`, and `front completion` intentionally return plain text
for compatibility with normal CLI tooling.

## Success envelope

```json
{
  "ok": true,
  "command": "front list tag",
  "result": {
    "data": {
      "_results": [
        { "id": "tag_123", "name": "Urgent" }
      ]
    },
    "count": 1,
    "next_page_token": "next-token"
  },
  "next_actions": [
    {
      "command": "front list tag",
      "description": "Next page of results",
      "params": {
        "--page-token": {
          "description": "Next page token",
          "value": "next-token"
        },
        "--param": {
          "description": "Additional query parameters; repeat this flag for each value",
          "values": ["q=alice", "sort_by=created_at"]
        }
      }
    }
  ]
}
```

- `ok` is always `true` for success.
- `command` identifies the command shape that produced the response.
- `result` contains command-specific data.
- `next_actions` is omitted when no continuation is available.

## Configuration result

`front config` returns a success envelope whose `result` includes the config
`path`, the effective `user`, and non-secret `token_source` and `user_source`
fields. `token_source` is one of `environment`, `token_command`, or `none`.
`user_source` is one of `environment`, `config`, or `none`. When a
`token_command` is configured, the optional `result.token_command` field is the
literal `(configured)`; the executable, its arguments, and token values are
never included.

## Doctor result

`front doctor` returns the normal success envelope with a redacted diagnostic
result:

```json
{
  "ok": true,
  "command": "front doctor",
  "result": {
    "token_source": "environment",
    "authentication": "ok",
    "configured_user_source": "config",
    "configured_user_matches_token": true,
    "checks": {
      "tags_read": "ok",
      "inboxes_read": "forbidden",
      "teammates_read": "error"
    }
  }
}
```

`authentication` is `ok` in a success result because `/me` failures remain
top-level failures. Those failures contain only a fixed CLI message and the
HTTP status used for normal error-code classification; Front response text is
discarded. Each optional read check is `ok`, `forbidden`, or `error`.
`configured_user_matches_token` is `true`, `false`, `unavailable`, or
`not_configured`. The two boolean states are JSON booleans; the unavailable
states are strings.

Doctor success output never includes API response bodies, Front-provided error
messages, token values, token-command arguments, effective user values,
resource IDs, or customer data.

## Generic read results

By default, `front whoami`, `front list`, `front get`, `front related`, and
`front api get` preserve the complete decoded API response under `result.data`.
The CLI does not rename or discard API fields. With no output control, the
generic result shape is unchanged and does not include `returned`, `projection`,
or `truncated`.

When `data._results` is an array, `result.count` contains the number returned in
the current response. Item and other non-collection JSON responses omit
`count`. When `_pagination.next` contains a page token, the result includes
`next_page_token` and a corresponding next action.

With a local output control active, the transformer recognizes two collection
shapes. A response object whose `_results` value is an array is a wrapped
collection; projection keeps the `_results` wrapper. A top-level JSON array is
an unwrapped collection; projection keeps it as a top-level array and does not
introduce an `_results` object. In both cases, `count` is the size before local
truncation and `returned` is the size afterward. Default output retains the
legacy behavior above and only derives `count` from an `_results` array.

A parameter with `value` is passed once. A parameter with `values` is repeated
once for each array item. A parameter with neither represents a bare boolean
switch. Pagination actions preserve filters, arbitrary query parameters, and
active generic output controls. When the original command explicitly supplied
`--profile`, the action also retains it as `--profile.value`. Default and
single-profile automatic selections remain implicit and are not added as a
new flag.

### Projected and bounded results

The generic resource and API GET commands support the following result
metadata only when a local output option is active:

| Field | Meaning |
|---|---|
| `count` | Original decoded `_results` array or top-level array size before local truncation; omitted for non-collections |
| `returned` | Number of collection items or the single projected item left in `data`; always `0` for count-only output |
| `projection` | Projection mode and, for field projection, the requested literal field names |
| `truncated` | `true` only when `--max-items` removed collection items |

Count-only output omits `data`:

```json
{
  "count": 42,
  "returned": 0,
  "projection": { "mode": "count-only" }
}
```

Keys-only output replaces objects with sorted key arrays. For a wrapped
collection, the `_results` wrapper remains but response values and the original
pagination object are not copied into `data`:

```json
{
  "data": {
    "_results": [
      ["id", "name"]
    ]
  },
  "count": 1,
  "returned": 1,
  "projection": { "mode": "keys-only" },
  "next_page_token": "next-token"
}
```

For a top-level array, keys-only output remains a top-level array:

```json
{
  "data": [
    ["id", "name"]
  ],
  "count": 1,
  "returned": 1,
  "projection": { "mode": "keys-only" }
}
```

Field projection uses a stable tagged form:

```json
{
  "data": { "id": "tag_123", "name": "Urgent" },
  "returned": 1,
  "projection": {
    "mode": "fields",
    "fields": ["id", "name"]
  }
}
```

For a wrapped collection, field projection applies to every object in
`data._results` and retains that wrapper. For a top-level array, it applies to
every array item and keeps `data` as an array. `--max-items` truncates the same
recognized collection shape without converting one shape into the other.

`--count-only` and `--keys-only` keep customer response values out of
`result.data`; they are not complete redaction modes. Counts, key names,
commands, resource identifiers, pagination tokens, errors, and action metadata
can still be sensitive. `--fields` deliberately returns the selected values,
and the default output returns the full decoded response. Review output before
sharing it outside its intended trust boundary.

## Compact workflow results

`front inboxes`, `front inbox`, and `front read` return compact models optimized
for triage. They do not preserve every field from the Front API. `front read`
truncates each message body at a valid UTF-8 boundary at or below 500 bytes and
reports when the message page is truncated.

Use an official read endpoint through `front api get` when a workflow requires
fields omitted by a compact result.

## Failure envelope

```json
{
  "ok": false,
  "command": "front list",
  "error": {
    "message": "unknown resource \"planets\"",
    "code": "INVALID_INPUT"
  },
  "fix": "Run 'front' to see supported resources and path rules"
}
```

Failures use exit status 1, write the JSON envelope to standard output, and do
not write a second machine-readable document to standard error.

## Error codes

| Code | Meaning |
|---|---|
| `CLI_ERROR` | Command or argument parsing failed |
| `INVALID_INPUT` | A date, resource, relation, path, ID, or query pair is invalid |
| `CONFIG_ERROR` | The config file or API client configuration is invalid |
| `UNAUTHORIZED` | No usable token exists or Front returned HTTP 401 |
| `FORBIDDEN` | The token lacks required access |
| `NOT_FOUND` | Front returned HTTP 404 |
| `RATE_LIMITED` | Front returned HTTP 429 |
| `TRANSPORT_ERROR` | The HTTP request could not complete |
| `API_ERROR` | Front returned another error or invalid JSON |
| `INTERNAL_ERROR` | The CLI could not serialize its result |

Invalid resources, relations, IDs, generic paths, and query pairs are rejected
before configuration resolution and before any HTTP request.

## Secret handling

Resolved tokens are never part of success output, error output, URLs, query
parameters, debug representations, or `front config`. `front config` does not
execute `token_command`. Doctor output never includes Front-provided response
text: required authentication failures use a fixed status-only message, and
optional API errors use fixed status strings.
