---
status: accepted
date: 2026-08-25
decision-makers: maintainers
---

# Expand read coverage with declarative shortcuts and a safe GET gateway

## Context and Problem Statement

The Rust rewrite initially retained only the five GET calls needed by the
legacy inbox workflow. Front's official Core API specification at commit
`dc76701bcf90f5b98dd56e76cae438243f63b94d` contains 147 paths and 123 GET
operations. The `dedene/frontmail-cli` reference also demonstrates that scripts
benefit from resource-oriented commands beyond conversations and inboxes.

Generating a complete client would restore a large maintenance surface. Adding
one handwritten command and response model per GET operation would create more
than one hundred thin forwarding handlers. Copying the reference CLI's write
surface would also conflict with this project's read-only safety boundary.

## Decision

Keep the existing typed conversation commands and add two complementary read
mechanisms:

- a closed registry powering `list`, `get`, and allowlisted `related`
  shortcuts; and
- `api get`, which accepts validated relative path segments and structured
  query pairs and decodes any JSON response without a lossy model.

The production origin remains fixed at `https://api2.frontapp.com`. The generic
client method is GET-only and has no request-body parameter. Absolute URLs,
queries embedded in paths, fragments, traversal, empty segments, and download
segments are rejected before authentication and HTTP. Shell completion is
generated locally and also runs before authentication.

## Consequences

- Good, because common resources are discoverable and future JSON GET paths
  remain usable without generated code.
- Good, because typed legacy output and command compatibility remain intact.
- Good, because URL construction stays segment-based and same-origin.
- Good, because invalid inputs fail before secrets are resolved or requests are
  sent.
- Good, because the API's original JSON is preserved for scripts.
- Bad, because generic results do not provide compile-time response schemas.
- Bad, because relation allowlists and documentation require review when Front
  changes the official specification.
- Bad, because binary downloads, OAuth, multiple accounts, and mutation
  workflows remain unavailable.

## Alternatives Considered

- Generate the full Rust OpenAPI client: strongest static typing, but high
  compile-time and maintenance cost for mostly unused operations.
- Handwrite every GET command: richest command help, but extensive repetitive
  code and continual synchronization work.
- Copy all reference-repository features: broadest feature parity, but adds
  mutation and credential-management behavior without a suitable safety
  design.
