# Contributing to Front CLI

Thank you for helping improve Front CLI. Contributions are welcome for bug
fixes, documentation, tests, platform support, and carefully designed read-only
features.

## Before you start

1. Search existing issues and pull requests.
2. Open an issue for behavior changes or work that affects multiple files.
3. Keep changes focused on one problem.
4. Never include Front API tokens, customer messages, email addresses, or other
   private data in an issue, test fixture, commit, or screenshot.

API mutations require an approved design before implementation. Do not add
message sending, conversation updates, contact changes, downloads, or another
write operation as an incidental change.

## Development setup

Install Rust 1.88 or newer, then clone your fork:

```bash
git clone https://github.com/YOUR-USER/frontmail-cli.git
cd frontmail-cli
cargo build --locked
```

Tests use local mock servers and do not require a Front account or API token.

## Make a change

Create a branch from `develop`:

```bash
git switch develop
git pull --ff-only
git switch -c fix/short-description
```

Follow the existing module boundaries and preserve the JSON envelope contract.
Add or update a contract test for observable behavior changes. Keep user-facing
copy and documentation in English.

Use conventional commit prefixes such as `feat:`, `fix:`, `docs:`, `test:`,
`refactor:`, `ci:`, and `chore:`.

## Verify locally

Run the complete quality gate before opening a pull request:

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets
cargo build --locked --release
```

You can also run `make check` for the fast formatting, lint, and test checks.

## Documentation changes

- Keep the root README concise.
- Put setup details in `docs/getting-started.md` or `docs/configuration.md`.
- Put command behavior in `docs/commands.md` or `docs/output-format.md`.
- Update `docs/api-support.md` when the supported Front API surface changes.
- Add an architecture decision record for a lasting security or architecture
  choice.

## Pull requests

A pull request should explain the problem, the chosen approach, user-visible
effects, safety implications, and verification performed. Link the relevant
issue and update the changelog when the change is notable to users.

By participating, you agree to follow the [Code of Conduct](CODE_OF_CONDUCT.md).
Report vulnerabilities according to the [Security Policy](SECURITY.md), not in
a public issue.
