# Open-Source Readiness Design

Date: 2026-08-25
Status: Approved

## Objective

Make `shuuheyhey/frontmail-cli` understandable, trustworthy, and easy to
contribute to as a public open-source Rust project. All public repository copy
will be English. The CLI behavior and its read-only Front API safety boundary
will remain unchanged.

## Current State

The repository already has a working Rust CLI, an MIT license, tests, an API
support matrix, architecture decision records, and an agent skill. Its public
GitHub community profile is incomplete: contribution guidelines, a code of
conduct, issue templates, and a pull-request template are missing. The README
also points its clone command at the former upstream owner instead of
`shuuheyhey/frontmail-cli`.

Detailed installation, configuration, command, and output documentation is
currently concentrated in the root README. Internal implementation plans live
under `docs/superpowers`, which is not a useful public information category.
The project has no GitHub Actions workflow, Dependabot configuration, or
repository metadata in `Cargo.toml`.

## Selected Approach

Adopt a community-ready structure without adding automated publishing or
release credentials.

```text
frontmail-cli/
├── .github/
│   ├── ISSUE_TEMPLATE/
│   │   ├── bug_report.yml
│   │   ├── feature_request.yml
│   │   └── config.yml
│   ├── workflows/ci.yml
│   ├── dependabot.yml
│   └── pull_request_template.md
├── docs/
│   ├── README.md
│   ├── getting-started.md
│   ├── commands.md
│   ├── configuration.md
│   ├── output-format.md
│   ├── api-support.md
│   ├── architecture.md
│   ├── decisions/
│   └── development/
│       ├── designs/
│       └── plans/
├── skills/front/
├── src/
├── tests/
├── .editorconfig
├── CHANGELOG.md
├── CODE_OF_CONDUCT.md
├── CONTRIBUTING.md
├── SECURITY.md
├── SUPPORT.md
├── LICENSE
└── README.md
```

`AGENTS.md` and its `CLAUDE.md` symlink remain at the repository root because
they are active contributor tooling, not user documentation.

## README Information Architecture

The root README is a concise landing page optimized for the first five minutes:

1. Product statement, badges, read-only positioning, and non-affiliation note.
2. A short example that demonstrates structured JSON and scriptability.
3. Installation and minimum configuration.
4. A compact command overview and links to task-specific documentation.
5. Contribution, security, support, license, and acknowledgements links.

Long configuration examples, the complete command guide, detailed envelope
examples, and support matrices move to focused documents. The README must not
claim crates.io packages, GitHub releases, Discussions, or other distribution
channels that do not exist.

## Documentation Responsibilities

- `docs/README.md`: documentation index and reading routes for users,
  contributors, and maintainers.
- `docs/getting-started.md`: prerequisites, source installation, first command,
  and a safe quick start.
- `docs/configuration.md`: environment precedence, config-file locations,
  `token_command`, and secret-handling guidance.
- `docs/commands.md`: legacy and resource-oriented command reference,
  pagination, shell completion, and read-only restrictions.
- `docs/output-format.md`: success and failure envelopes, error codes,
  `next_actions`, raw JSON preservation, and truncation behavior.
- `docs/api-support.md`: official specification pin, resource registry,
  relation registry, and deliberate API gaps.
- `docs/architecture.md`: source layout, request flow, security boundaries, and
  testing strategy.
- `docs/decisions/`: accepted architecture decision records.
- `docs/development/designs/` and `docs/development/plans/`: historical design
  and implementation records moved from `docs/superpowers` without deleting
  their Git history.

Each fact has one canonical home. Other documents link to it instead of
copying substantial sections.

## Community Health Files

- `CONTRIBUTING.md` explains issue selection, development setup, conventional
  commits, tests, pull-request expectations, documentation changes, and the
  explicit approval requirement for API mutations.
- `CODE_OF_CONDUCT.md` defines concise participation and enforcement
  expectations without inventing a maintainer email address.
- `SECURITY.md` directs private vulnerability reports to GitHub Security
  Advisories and prohibits public disclosure of tokens or customer data.
- `SUPPORT.md` routes reproducible bugs and usage questions through GitHub
  Issues while routing Front product/API questions to Front documentation and
  support.
- `CHANGELOG.md` follows Keep a Changelog structure, starts with an
  `Unreleased` section, and does not invent release tags or dates.
- Issue forms collect version, platform, reproduction, expected behavior, and
  redacted output. Pull requests get a concise safety and verification
  checklist. Blank issues are disabled.

## Continuous Integration

Add `.github/workflows/ci.yml` for pushes and pull requests targeting
`develop`. It will use read-only repository permissions, install Rust 1.88.0,
and run the same locked checks documented for contributors:

```text
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets
cargo build --locked --release
```

No workflow receives repository write permissions, secrets, Front tokens,
release permissions, or package publishing credentials. Dependabot checks
Cargo and GitHub Actions dependencies weekly with a small open-PR limit.

## Package Metadata

Update `Cargo.toml` with:

- `repository = "https://github.com/shuuheyhey/frontmail-cli"`;
- `readme = "README.md"`;
- accurate CLI keywords;
- the `command-line-utilities` category.

Do not add a homepage when the repository is the project homepage. Do not
change the package version or publish to crates.io as part of this work.

## Validation

The completed structure must pass:

1. YAML parsing for every GitHub workflow, issue form, and Dependabot file.
2. Markdown relative-link validation for root and `docs/` documents.
3. `cargo metadata --locked --offline` to validate package metadata.
4. Rust formatting, strict Clippy, the complete test suite, and release build.
5. `git diff --check`, stale-owner/stale-path scans, secret-pattern scans, and
   a final clean status review.

CI configuration is also inspected with GitHub's workflow schema conventions;
the local environment cannot claim a hosted Actions run until the changes are
pushed and GitHub reports a result.

## Deliberate Exclusions

- No crates.io publication.
- No GitHub Release creation or cross-platform release workflow.
- No branch-protection, repository-topic, homepage, Discussions, or other
  GitHub settings changes.
- No generated website, logo, screenshots, or social preview image.
- No CLI feature, API request, configuration, or output-contract changes.
- No replacement of the existing MIT license or copyright attribution.

These can be handled independently after the community-ready repository is in
place and a release/versioning policy has been selected.
