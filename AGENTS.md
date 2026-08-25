# Front CLI

Agent-first CLI for the Front API. HATEOAS JSON envelope output with `next_actions`.

## Build & Test

```
cargo build --locked --release
cargo test --locked --all-targets
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
```

## Commits

Use conventional commits: `feat:`, `fix:`, `refactor:`, `test:`, `docs:`, `chore:`.

## Architecture

- `src/main.rs` — process-level routing, configuration, and error handling.
- `src/cli.rs` — clap command tree and validation-to-request conversion.
- `src/commands/` — typed legacy workflows and generic read envelopes.
- `src/client.rs` — typed legacy GETs plus the safe generic JSON GET boundary.
- `src/config.rs` — config file loading and secret token resolution.
- `src/envelope.rs` — JSON envelope output format.
- `src/models.rs` — Front response models and compact CLI output models.
- `src/resources.rs` — pure resource registry, relation allowlists, and input validation.
- `tests/` — black-box CLI and local mock-HTTP contract tests.
- `skills/front/` — Agent skill definition.
- `docs/` — public user, contributor, architecture, and API documentation.
- `docs/development/` — approved designs and historical implementation plans.
- `.github/` — issue forms, pull-request guidance, CI, and dependency updates.

## Key Patterns

- All machine-readable output goes through `envelope::success` / `envelope::failure`.
- Token resolution: `FRONT_API_TOKEN` env > `token_command` from config > error.
- User resolution: `FRONT_USER` env > `user` from config > empty.
- Teammate references use `alt:email:<email>` format.
- API access is read-only. Do not add a write endpoint without an explicit design and approval.
- Generic API paths remain same-origin GETs and reject absolute, traversal, and download paths.
- Keep `docs/api-support.md` pinned to the official specification revision used for registry changes.
- Keep the root README concise and put detailed guidance in the matching `docs/` file.
- Public documentation, issue templates, and contribution copy are English.
- Secrets use `secrecy::SecretString` and must never appear in logs, errors, fixtures, or diffs.
