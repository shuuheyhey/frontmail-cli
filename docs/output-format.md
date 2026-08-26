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

## Generic read results

`front whoami`, `front list`, `front get`, `front related`, and
`front api get` preserve the complete decoded API response under `result.data`.
The CLI does not rename or discard API fields.

When `data._results` is an array, `result.count` contains the number returned in
the current response. Item and other non-collection JSON responses omit
`count`. When `_pagination.next` contains a page token, the result includes
`next_page_token` and a corresponding next action.

A parameter with `value` is passed once. A parameter with `values` is repeated
once for each array item, preserving filters and arbitrary query parameters
when an agent follows a pagination action.

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
execute `token_command`. API error messages may still contain Front-provided
text, so redact output before sharing it.
