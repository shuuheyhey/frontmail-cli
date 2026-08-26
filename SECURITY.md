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
and download paths at the generic API boundary. Automatic HTTP redirects remain
disabled. The client manually follows only HTTP 301 responses, for at most three
hops, after validating the `Location` target's origin and API path. Each
accepted target is rebuilt onto the configured base origin before the next
authenticated GET; it is never requested directly.

The configured origin is accepted for redirect targets. When the configured
origin is the production API, Front production aliases are also accepted only
over HTTPS with effective port 443: `api2.frontapp.com` and subdomains of
`api.frontapp.com`. HTTP 302 and other redirect statuses, missing or malformed
locations, and targets with unsafe paths or unapproved origins are not followed.
Adding any mutation requires a separate approved design and explicit safety
review.

Resolved tokens and token-command arguments are not printed. Configuration
parse failures are sanitized to avoid including raw YAML values or parser source
chains in display or debug output.

Rotate a Front API token immediately if it is exposed in a terminal capture,
log, issue, commit, or other shared artifact.
