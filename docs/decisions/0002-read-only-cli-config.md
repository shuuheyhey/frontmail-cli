---
status: accepted
date: 2026-03-14
decision-makers: Josh
---

# Make CLI config read-only to prevent agent-planted command injection

## Context and Problem Statement

The Front CLI uses `token_command` in its config file to resolve API tokens by executing an external command. This enables composability with secret managers like [agent-secrets](https://github.com/joelhooks/agent-secrets), 1Password CLI, and similar tools.

A security audit identified that any agent invoking the CLI can run `front config set token_command "malicious command"` to plant a persistent executable hook in the config file. This command then executes on every subsequent CLI invocation — by the user, other tools, or other agents.

How should we secure `token_command` without losing composability with external secret managers?

## Decision

Remove all config-writing subcommands (`config set`, `config path`). Keep `token_command` in the config file, but it can only be set by humans editing `~/.config/front/config.yaml` directly. The CLI treats local config as trusted — same model as Git's `.gitconfig`.

Two additional changes:

- `token_command` is now an argv-style `[]string` instead of a whitespace-split string, so arguments with spaces work correctly:
  ```yaml
  token_command:
    - op
    - read
    - "My Vault/API Token"
  ```
- `front config` is retained as a read-only command — it shows the config file path, `token_command: (configured)` or omits it when unset, and the `user` value. It never exposes the actual `token_command` value.

## Consequences

- Good, because agents can no longer plant persistent code-execution hooks
- Good, because composability with agent-secrets and similar tools is preserved
- Good, because argv-style format handles paths with spaces correctly
- Good, because argv-style format eliminates shell injection as a concern — no string parsing step, each element goes directly to `exec.Command`
- Bad, because initial setup requires manually editing YAML

## Alternatives Considered

- **OS keyring storage** (`byteness/keyring`): Eliminates command execution entirely, but also eliminates composability with agent-focused secret managers that provide time-bounded leases, audit trails, and killswitch. A long-lived token in a keyring can be exfiltrated once and used forever.
- **OS keyring with biometrics (Touch ID)**: Explored via `byteness/keyring`. Requires a dedicated named keychain (not the login keychain), meaning Touch ID prompts on every CLI invocation with no "Always Allow" equivalent. Wrong UX for a CLI tool.
- **Env-var-only `FRONT_TOKEN_COMMAND`**: Unnecessary — `FRONT_API_TOKEN` already covers the env var path. Orchestrators should resolve the secret before calling the CLI.

## More Information

**Trust model**: Local config is trusted, human-managed. Agents invoke the CLI but cannot modify config.
