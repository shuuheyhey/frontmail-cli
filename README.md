# Front CLI

[![CI](https://github.com/shuuheyhey/frontmail-cli/actions/workflows/ci.yml/badge.svg?branch=develop)](https://github.com/shuuheyhey/frontmail-cli/actions/workflows/ci.yml)
[![Rust 1.88+](https://img.shields.io/badge/Rust-1.88%2B-000000?logo=rust)](Cargo.toml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

Fast, script-friendly, agent-friendly command-line access to the
[Front Core API](https://dev.frontapp.com/reference/introduction). Front CLI is
written in Rust, emits structured JSON with actionable next steps, and is
intentionally read-only.

This community project is not affiliated with, endorsed by, or supported by
Front.

## Why Front CLI?

- Use stable JSON envelopes instead of parsing human-oriented terminal output.
- Triage inboxes and conversations with compact, context-efficient commands.
- Read common Front resources through `list`, `get`, and `related` shortcuts.
- Reach other JSON GET endpoints through a validated, same-origin API gateway.
- Keep automation safe: the client has no POST, PUT, PATCH, DELETE, or download
  operation.

## Quick start

Install from the public repository with Rust 1.88 or newer:

```bash
cargo install --locked --git https://github.com/shuuheyhey/frontmail-cli --branch develop
```

Configure a Front API token without placing it in a command argument:

```bash
export FRONT_API_TOKEN="your-api-token"
export FRONT_USER="user@example.com" # optional

front config
front whoami
front inboxes
```

Run `front` with no arguments to get the machine-readable command catalog and
its `next_actions`.

## Commands

| Command | Purpose |
|---|---|
| `front` | Return the command catalog |
| `front config` | Show configuration state without exposing the token |
| `front doctor` | Run redacted authentication and read-scope diagnostics |
| `front inboxes` | List available inboxes |
| `front inbox [inbox-id]` | Search conversations |
| `front read <conversation-id>` | Read a compact conversation thread |
| `front whoami` | Show the authenticated Front user |
| `front list <resource>` | List a supported Front resource |
| `front get <resource> <id>` | Get one supported resource |
| `front related <resource> <id> <relation>` | List an allowlisted relation |
| `front api get <path>` | GET a validated relative Core API path |
| `front completion <shell>` | Generate shell completion code |

See the [command reference](docs/commands.md) for flags, pagination, resource
names, examples, and safety rules.

## Documentation

- [Getting started](docs/getting-started.md)
- [Configuration and secret handling](docs/configuration.md)
- [Command reference](docs/commands.md)
- [JSON output format](docs/output-format.md)
- [Front API support matrix](docs/api-support.md)
- [Architecture](docs/architecture.md)
- [Documentation index](docs/README.md)

## Contributing

Contributions are welcome. Read [CONTRIBUTING.md](CONTRIBUTING.md) for setup,
quality checks, commit conventions, and the read-only API boundary. Community
participation follows the [Code of Conduct](CODE_OF_CONDUCT.md).

## Security

Never post Front tokens or customer data. Report vulnerabilities privately by
following [SECURITY.md](SECURITY.md).

## Support

See [SUPPORT.md](SUPPORT.md) for CLI support and guidance on Front product or
API questions.

## License

Front CLI is available under the [MIT License](LICENSE).
