# Architecture Decision Records (ADR)

An Architecture Decision Record (ADR) captures an important architecture decision along with its context and consequences.

Return to the [documentation index](../README.md) or read the current
[architecture overview](../architecture.md).

## Conventions

- Directory: `docs/decisions`
- Naming:
  - Prefer numbered files when starting fresh: `0001-choose-database.md`
  - If the repo already uses slug-only names, keep that: `choose-database.md`
- Status values: `proposed`, `accepted`, `rejected`, `deprecated`, `superseded`

## Workflow

- Create a new ADR as `proposed`.
- Discuss and iterate.
- When the team commits: mark it `accepted` (or `rejected`).
- If replaced later: create a new ADR and mark the old one `superseded` with a link.

## ADRs

- [Adopt architecture decision records](0001-adopt-architecture-decision-records.md) (accepted, 2026-03-14)
- [Make CLI config read-only to prevent agent-planted command injection](0002-read-only-cli-config.md) (accepted, 2026-03-14)
- [Rewrite the Front CLI in Rust](0003-rewrite-cli-in-rust.md) (accepted, 2026-08-25)
- [Expand read coverage with declarative shortcuts and a safe GET gateway](0004-expand-read-api-coverage.md) (accepted, 2026-08-25)
- [Isolate named configuration profiles from ambient credentials](0005-isolate-named-configuration-profiles.md) (accepted, 2026-08-26)
