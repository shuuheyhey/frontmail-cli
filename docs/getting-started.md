# Getting started

This guide installs Front CLI from source, configures a token safely, and runs
the first read-only workflow.

## Requirements

- Rust 1.88 or newer
- A Front API token with access to the data you need to read
- Linux, macOS, or Windows

Front CLI is an independent community project. Front account setup, token
creation, scopes, and product access are documented by
[Front](https://dev.frontapp.com/reference/introduction).

## Install from the repository

Install the `front` binary directly from the public `develop` branch:

```bash
cargo install --locked --git https://github.com/shuuheyhey/frontmail-cli --branch develop
front --version
```

To build a checkout instead:

```bash
git clone https://github.com/shuuheyhey/frontmail-cli.git
cd frontmail-cli
cargo build --locked --release
./target/release/front --version
```

No crates.io package or prebuilt release is currently advertised.

## Configure the token

Export the token in your current shell:

```bash
export FRONT_API_TOKEN="your-api-token"
export FRONT_USER="user@example.com" # optional
```

`FRONT_USER` limits the legacy `front inboxes` workflow to the inboxes of the
matching teammate. It is not required by resource-oriented commands.

For persistent secret-manager integration, use `token_command` as described in
[Configuration](configuration.md). Do not put a token in a command argument,
issue, log, screenshot, or repository file.

To keep multiple accounts or environments separate, configure named profiles:

```yaml
default_profile: work
profiles:
  work:
    user: user@example.com
    token_command:
      - op
      - read
      - op://Vault/work_front_api_token/password
```

Then use the default or select it explicitly:

```bash
front config
front --profile work config
front whoami --profile work
```

An explicit profile ignores ambient `FRONT_API_TOKEN` and `FRONT_USER` values.
See [Configuration](configuration.md) before migrating legacy top-level fields.

## Verify access

```bash
front config
front whoami
```

For legacy selection, `front config` reports the config path and effective
user. For a named profile it reports the profile name and source while omitting
the profile user. When a token command exists, `result.token_command` is the
literal string `(configured)`; when it is unset, that field is omitted. The
command never resolves or prints the token or token-command arguments. `front
whoami` performs the first authenticated GET and returns the Front user
represented by the token.

## Run the first workflow

```bash
front inboxes
front inbox <inbox-id> --limit 10
front read <conversation-id>
```

Every API command returns a JSON envelope. Follow values in `next_actions`
instead of reconstructing IDs or pagination parameters by hand.

Next: read the [Command reference](commands.md) and
[Output format](output-format.md).
