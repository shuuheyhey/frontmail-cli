## Summary

Describe the problem, the selected approach, and any user-visible effect.

## Safety

- [ ] This change preserves the GET-only API boundary, or an approved design for
      a mutation is linked below.
- [ ] Tests, fixtures, logs, screenshots, and documentation contain no Front
      token, customer data, or personal information.
- [ ] Generic API paths remain same-origin and reject traversal and downloads.

## Verification

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --locked --all-targets --all-features -- -D warnings`
- [ ] `cargo test --locked --all-targets`
- [ ] `cargo build --locked --release`

## Related issue or design

Link the issue, design, or architecture decision when applicable.
