---
status: accepted
date: 2026-08-26
decision-makers: maintainers
---

# Isolate named configuration profiles from ambient credentials

## Context and Problem Statement

The legacy configuration model combines `FRONT_API_TOKEN` and `FRONT_USER`
with top-level `token_command` and `user` fields. That precedence is convenient
for one account, but it cannot select among multiple Front accounts or
environments. Reusing ambient or top-level credentials inside a named selection
would also risk silently authenticating to a different account than the user
requested.

Named profiles must preserve existing legacy behavior, keep token commands
human-managed and read-only, avoid exposing credential values, and remain
stable when commands are replayed from `next_actions`.

## Decision

Add a `profiles` map and optional `default_profile` to `front/config.yaml`, plus
a global `--profile <name>` option. Profile names are non-blank, matched
exactly, and are not trimmed or treated as aliases.

Selection follows these rules:

1. An explicit `--profile` selects only that profile's `token_command` and
   `user`. It ignores `FRONT_API_TOKEN`, `FRONT_USER`, and legacy top-level
   credentials.
2. Without `--profile`, use the complete legacy selection when either its token
   or user has a source.
3. Otherwise use `default_profile`, then automatically select the sole profile.
4. Multiple profiles without a default require explicit selection.

`front config` reports only non-secret source metadata. For named selection it
includes the profile name and whether selection was explicit, defaulted, or
automatic, but omits the profile user and token-command arguments. It does not
execute a token command. Authentication diagnostics resolve the selected token
but return only redacted status and source fields.

Only an explicitly supplied profile is preserved in authenticated navigation,
refresh, and pagination actions. Default and sole-profile selections remain
implicit so action output does not turn an ambient configuration decision into
a newly pinned command argument.

## Consequences

- Good, because one config file can safely select multiple accounts and
  environments.
- Good, because explicit selection cannot be silently overridden by ambient or
  legacy credentials.
- Good, because existing single-account environment and top-level config
  behavior remains compatible.
- Good, because replayable actions retain an explicit account boundary without
  exposing implicit profile selection.
- Bad, because a named profile cannot inherit `FRONT_API_TOKEN` or top-level
  fields; it must provide its own human-managed `token_command` and optional
  `user`.
- Bad, because any remaining legacy token or user source takes priority over a
  default or sole profile until the legacy field or environment variable is
  removed.
- Bad, because profile names may appear in config metadata, errors, and actions
  even though profile credential values remain redacted.

## Alternatives Considered

- Let named profiles inherit environment variables: convenient, but an ambient
  credential could override the explicitly selected account.
- Use only environment-variable prefixes per account: avoids config schema
  changes, but makes discovery, default selection, and portable secret-manager
  integration harder.
- Add config-writing commands or store tokens directly: easier setup, but
  conflicts with the accepted read-only config trust model in
  [ADR 0002](0002-read-only-cli-config.md).
