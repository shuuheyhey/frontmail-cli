# Security Policy

## Reporting a vulnerability

Report suspected vulnerabilities privately through a
[GitHub Security Advisory](https://github.com/shuuheyhey/frontmail-cli/security/advisories/new).
Do not open a public issue for a vulnerability before a fix or coordinated
disclosure is ready.

Include the affected command or module, impact, reproduction steps, and a
minimal proof of concept when safe. Remove Front API tokens, customer content,
personal data, and production resource identifiers from every report and
attachment.

Maintainers will acknowledge the report, investigate it, and coordinate next
steps through the advisory. No response-time or release-time guarantee is made
for this volunteer project.

## Security boundary

Front CLI is intentionally read-only. It sends authenticated GET requests only
to the configured Front API origin and rejects absolute, traversal, fragment,
and download paths at the generic API boundary. HTTP redirects are not followed,
so a validated request cannot cross that fixed-origin boundary through a
redirect. Adding any mutation requires a separate approved design and explicit
safety review.

Resolved tokens and token-command arguments are not printed. Configuration
parse failures are sanitized to avoid including raw YAML values or parser source
chains in display or debug output.

Rotate a Front API token immediately if it is exposed in a terminal capture,
log, issue, commit, or other shared artifact.
