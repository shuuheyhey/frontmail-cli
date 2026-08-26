# Configuration

Front CLI reads an API token and an optional teammate identity. Configuration
is read-only: the CLI never writes a config file and never prints a resolved
token.

## Resolution order

| Value | First choice | Fallback | Missing behavior |
|---|---|---|---|
| API token | non-empty `FRONT_API_TOKEN` | `token_command` | `UNAUTHORIZED` |
| Teammate email | non-empty `FRONT_USER` | config `user` | empty; list all accessible inboxes |

Environment variables always take precedence over the config file.

## Config file location

The file is named `front/config.yaml` inside the operating system's standard
configuration directory:

- Linux: `${XDG_CONFIG_HOME:-~/.config}/front/config.yaml`
- macOS: `~/Library/Application Support/front/config.yaml`
- Windows: Roaming AppData, typically `%APPDATA%\front\config.yaml`

Run `front config` to see the exact path selected on the current machine.
A missing file is valid and behaves like an empty configuration.

The command reports `result.token_command` as `(configured)` when the list is
non-empty and omits the field when it is unset. It never prints the executable,
arguments, or resolved token.

## Config file format

```yaml
user: user@example.com
token_command:
  - op
  - read
  - op://Vault/front_api_token/password
```

`token_command` is a YAML list. The first item is the executable and every
remaining item is one argument. Front CLI executes it directly without a
shell, trims its standard output, and uses the result as the token.

The command fails safely when the executable cannot run, returns a non-zero
status, emits invalid UTF-8, or returns empty output.

## Environment-only configuration

```bash
export FRONT_API_TOKEN="your-api-token"
export FRONT_USER="user@example.com"
front config
```

Environment variables are convenient for short-lived sessions and CI jobs, but
process environments may be visible to other software on the same machine. A
secret manager plus `token_command` is preferred for persistent use.

## Secret hygiene

- Never pass a token as a CLI argument.
- Never store a token in this repository, a dotfile committed to source
  control, shell history, an issue, or a test fixture.
- Use synthetic values such as `test-token`, `cnv_123`, and
  `user@example.com` in examples.
- Redact customer messages, attachments, personal data, and real Front resource
  IDs before sharing output.
- Rotate a token immediately if it is exposed.

See the [Security policy](../SECURITY.md) for private vulnerability reporting.
