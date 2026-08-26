# Configuration

Front CLI reads an API token and an optional teammate identity. Configuration
is read-only: the CLI never writes a config file and never prints a resolved
token.

## Named-profile selection

Front CLI supports the legacy top-level credentials and optional named
profiles. Use the global option before or after a subcommand:

```bash
front --profile work whoami
front inboxes --profile work
```

An explicit `--profile` uses only that profile's `token_command` and `user`.
It completely ignores `FRONT_API_TOKEN`, `FRONT_USER`, and the legacy
top-level `token_command` and `user`. This isolation prevents an ambient shell
credential from silently changing the selected account.

Without `--profile`, selection follows this order:

1. If either legacy token or user selection has a source, use the complete
   legacy selection and preserve environment precedence for both values.
2. Otherwise, use `default_profile` when configured.
3. Otherwise, automatically select the profile when exactly one exists.
4. With multiple profiles, require `--profile`; with no profiles, retain the
   legacy no-token behavior.

An unknown explicit profile, an unknown `default_profile`, or multiple
profiles without a default returns `CONFIG_ERROR`. The error may list profile
names, but never profile users or token-command values. A configured
`default_profile` is validated before the one-profile automatic fallback.

Every profile key, `default_profile`, and explicit `--profile` value must
contain at least one non-whitespace character. Blank values return
`CONFIG_ERROR` at the shared selection boundary before any profile is chosen,
and blank keys are never rendered as available names. Non-blank names are
matched exactly: Front CLI does not trim leading or trailing spaces or treat a
trimmed spelling as an alias.

If Front returns HTTP 401, legacy selection retains the existing guidance to
check `FRONT_API_TOKEN` or top-level `token_command`. Named-profile selection
instead points to that profile's `token_command`; it does not suggest ambient
or top-level credentials that the selected profile cannot use. Only the
profile name and config path may appear in this guidance.

## Legacy credential resolution

| Value | First choice | Fallback | Missing behavior |
|---|---|---|---|
| API token | non-empty `FRONT_API_TOKEN` | `token_command` | `UNAUTHORIZED` |
| Teammate email | non-empty `FRONT_USER` | config `user` | empty; list all accessible inboxes |

Environment variables take precedence only in legacy selection. They never
override an explicitly selected, default, or automatically selected profile.

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
arguments, or resolved token, and it does not execute `token_command`.

`front config` uses the same profile selection as authenticated commands. For
legacy selection it reports the effective `result.user` for compatibility.
For a selected profile it reports `result.profile` and `result.profile_source`
but omits `result.user`. The profile source is `explicit`, `default`, or
`single`. Profile user values, token values, and token-command argv are never
printed.

The command also reports non-secret source fields with these exact values:

| Field | Values | Selection |
|---|---|---|
| `result.token_source` | `environment`, `token_command`, `none` | Non-empty `FRONT_API_TOKEN`, then a non-empty `token_command`, then no token source. |
| `result.user_source` | `environment`, `config`, `none` | Non-empty `FRONT_USER`, then a non-empty config `user`, then no user source. |

For legacy selection, an empty environment variable falls back to the next
source. For profile selection, `token_command` and `config` refer to fields on
the selected profile. The source report only identifies the selected source;
it never resolves a token command.

`front doctor` uses the same effective token and user selection. Its redacted
success result includes `token_source` and `configured_user_source`, but never
the selected token, token-command arguments, or effective user value. Unlike
`front config`, diagnostics resolve the token and perform the read-only network
checks documented in [Command reference](commands.md#redacted-diagnostics).

## Config file format

```yaml
user: user@example.com
token_command:
  - op
  - read
  - op://Vault/front_api_token/password

default_profile: work
profiles:
  work:
    user: work-user@example.com
    token_command:
      - op
      - read
      - op://Vault/work_front_api_token/password
  sandbox:
    token_command:
      - op
      - read
      - op://Vault/sandbox_front_api_token/password
```

`token_command` is a YAML list. The first item is the executable and every
remaining item is one argument. Front CLI executes it directly without a
shell, trims its standard output, and uses the result as the token.

The command fails safely when the executable cannot run, returns a non-zero
status, emits invalid UTF-8, or returns empty output.

Profile `token_command` uses the same argv-array execution. Arguments that
contain spaces remain one argument, and no shell parses or expands them.

## Migrating legacy configuration

Existing top-level `user` and `token_command` files continue to work without
changes. To migrate, move those fields under a profile name, then either set
`default_profile` or pass `--profile` explicitly. Remove the top-level fields
only after checking `front config --profile <name>`; otherwise their legacy
source takes priority when no profile is specified.

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
