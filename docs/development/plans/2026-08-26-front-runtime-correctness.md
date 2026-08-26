# Front Runtime Correctness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Safely follow Front's merged-conversation 301 responses, retain sender identity when a message has no author, and report the invoked command on pre-request authentication failures.

**Architecture:** Keep redirects disabled in reqwest and handle only HTTP 301 explicitly inside `FrontClient`, revalidating and rewriting approved Front API targets onto the configured base origin before repeating the authenticated GET. Extend the compact message model with documented recipients and use the `from` recipient only when `author` is absent. Derive the command label before configuration and token resolution so every failure envelope uses the same label as the requested operation.

**Tech Stack:** Rust 1.88.0, Rust 2024, reqwest, url, serde, wiremock, assert_cmd.

**Spec:** Approved findings and fix order from the 2026-08-26 Front CLI audit in this conversation.

## Global Constraints

- Preserve the read-only boundary: authenticated GET requests only, with no request bodies or mutation methods.
- Keep reqwest automatic redirects disabled.
- Follow only HTTP 301 responses, with at most three redirect hops.
- Accept a redirect only when it remains on the configured origin, or when the production client receives an HTTPS `api2.frontapp.com` / `*.api.frontapp.com` target; always rebuild the request on the configured base origin.
- Re-run existing path validation on every redirect and continue rejecting `download`, traversal, empty, fragment, and control paths.
- Never include tokens in URLs, errors, debug output, fixtures, or reports.
- Preserve the current JSON envelope and exit-code contracts except for the corrected `command` value.
- Do not commit or push; leave reviewed changes in the working tree.

---

### Task 1: Safe HTTP 301 following

**Files:**
- Modify: `src/client.rs`
- Test: `tests/http_contract.rs`

**Interfaces:**
- Consumes: `resources::validate_api_path`, `FrontClient.base_url`, and the existing authenticated GET construction.
- Produces: `FrontClient::get_json` behavior that follows up to three validated 301 locations while preserving the fixed-origin boundary.

- [ ] **Step 1: Write failing redirect tests**

Add tests that exercise real `FrontClient` requests against wiremock:

```rust
#[tokio::test]
async fn get_value_follows_a_safe_301_on_the_configured_origin() {
    // `/old` returns 301 Location: `/new`; `/new` returns JSON.
    // Assert the decoded JSON comes from `/new`.
}

#[tokio::test]
async fn get_value_does_not_follow_a_301_to_a_download_path() {
    // `/safe` returns 301 Location: `/download/file`.
    // Assert ClientError::Http { status: 301, .. } and zero `/download/file` calls.
}
```

- [ ] **Step 2: Run the targeted tests and verify RED**

Run: `cargo test --locked --test http_contract get_value_follows_a_safe_301_on_the_configured_origin -- --exact`

Expected: FAIL because the current client returns `ClientError::Http { status: 301, .. }` instead of requesting `/new`.

- [ ] **Step 3: Implement minimal safe 301 handling**

Keep `redirect(Policy::none())`. In `get_json`, repeat at most three times only when `status == StatusCode::MOVED_PERMANENTLY`. Resolve `Location`, validate its origin and path, call `validate_api_path`, rebuild the path and query on `self.base_url`, and resend with bearer auth. Unsafe, missing, or excessive redirect responses remain `ClientError::Http` with the original 301 status.

- [ ] **Step 4: Verify GREEN**

Run: `cargo test --locked --test http_contract`

Expected: all HTTP contract tests pass, including the existing no-follow 302 test.

### Task 2: Message sender fallback

**Files:**
- Modify: `src/models.rs`
- Modify: `src/commands/mod.rs`
- Test: `tests/read_contract.rs`

**Interfaces:**
- Consumes: Front message `author` and `recipients` fields.
- Produces: `MessageSummary.from` populated from `author`, or from the first recipient whose role is exactly `from` when `author` is absent.

- [ ] **Step 1: Write a failing compact-read test**

Add a complete message fixture with `author: null` and documented recipients:

```json
{
  "id": "msg_chat",
  "author": null,
  "recipients": [
    {"name": "Visitor", "handle": "visitor-123", "role": "from"},
    {"name": "Support", "handle": "support", "role": "to"}
  ],
  "text": "Hello",
  "is_inbound": true
}
```

Assert `result.messages[*].from` equals `{"handle":"visitor-123","name":"Visitor"}`.

- [ ] **Step 2: Run the targeted test and verify RED**

Run: `cargo test --locked --test read_contract read_fetches_conversation_and_messages_and_truncates_utf8_safely -- --exact`

Expected: FAIL because `MessageResponse` currently discards recipients and `map_message` returns no sender when `author` is null.

- [ ] **Step 3: Implement the fallback**

Add `role: String` to `RecipientResponse`, add `recipients: Vec<RecipientResponse>` to `MessageResponse`, and update `map_message` so `author` remains preferred while an absent author falls back to the first `role == "from"` recipient using its `handle` and optional `name`.

- [ ] **Step 4: Verify GREEN**

Run: `cargo test --locked --test read_contract`

Expected: all compact-read contract tests pass.

### Task 3: Authentication failure command label

**Files:**
- Modify: `src/main.rs`
- Test: `tests/cli_contract.rs`

**Interfaces:**
- Consumes: the prepared `ReadRequest.command` when present and `command_name(&Commands)` otherwise.
- Produces: config, token-resolution, and client-construction failure envelopes labeled with the requested command.

- [ ] **Step 1: Write failing CLI tests**

Add isolated no-token assertions for both a fixed command and a normalized resource command:

```rust
assert_eq!(whoami_error["command"], "front whoami");
assert_eq!(list_error["command"], "front list tag");
assert_eq!(whoami_error["error"]["code"], "UNAUTHORIZED");
```

- [ ] **Step 2: Run the targeted tests and verify RED**

Run: `cargo test --locked --test cli_contract authentication_failures_report_the_requested_command -- --exact`

Expected: FAIL because `run_api_command` currently passes the literal `front` during token resolution.

- [ ] **Step 3: Implement command derivation before authentication**

At the start of `run_api_command`, derive an owned command string from `request.as_ref().map(|request| request.command.clone())`, falling back to `command_name(&command)`. Use that value for configuration, token-resolution, and client-construction failures without changing successful command labels.

- [ ] **Step 4: Verify GREEN**

Run: `cargo test --locked --test cli_contract`

Expected: all CLI contract tests pass.

### Task 4: Integrated verification and documentation

**Files:**
- Modify if required: `CHANGELOG.md`
- Modify if required: `SECURITY.md`
- Modify if required: `docs/architecture.md`

**Interfaces:**
- Consumes: completed Tasks 1-3.
- Produces: documentation that describes validated 301 handling without weakening the fixed-origin boundary.

- [ ] **Step 1: Update affected documentation**

Document that automatic redirects remain disabled and only validated HTTP 301 targets are replayed on the configured origin. Add the three fixes under `Unreleased` in the changelog.

- [ ] **Step 2: Run the full quality gate**

Run:

```text
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets
cargo build --locked --release
git diff --check
```

Expected: all commands exit 0; 301, sender fallback, and authentication label regression tests pass.
