# Task: add-cli-unit-tests

## Goal

Write comprehensive unit tests for all CLI components: enum parsing, argument parsing, validation logic, JWT key resolution, and exempt path parsing.

## Files Involved

- `src/cli.rs` — `#[cfg(test)] mod tests` block

## Steps (TDD-first)

1. **Enum parsing tests:**
   - All `Transport` variants parse from their string representations.
   - All `ApprovalMode` variants parse from their string representations.
   - All `LogLevel` variants parse from their string representations.
   - Invalid enum values produce errors.
   - `LogLevel::to_level_filter()` returns correct tracing levels.

2. **CliArgs parsing tests (using `CliArgs::try_parse_from`):**
   - Minimal args: `["apcore-mcp", "--extensions-dir", "/tmp"]` — verify all defaults.
   - Full args: every field explicitly set — verify all values.
   - Missing required `--extensions-dir` — verify error.
   - `--port 0` — verify clap rejects it (below range).
   - `--port 65535` — verify accepted.
   - `--transport streamable-http` — verify `Transport::StreamableHttp`.
   - `--no-jwt-require-auth` — verify `jwt_require_auth == false`.
   - `--jwt-require-auth` alone — verify `jwt_require_auth == true`.

3. **Validation tests:**
   - `validate_args` with non-existent extensions dir returns error.
   - `validate_args` with a file (not dir) as extensions dir returns error.
   - `validate_args` with name > 255 chars returns error.
   - `validate_args` with valid args returns Ok.

4. **JWT key resolution tests:**
   - With `jwt_key_file` pointing to a valid file — returns file contents.
   - With `jwt_key_file` pointing to non-existent file — returns error.
   - With `jwt_secret` set (no file) — returns secret string.
   - With `APCORE_JWT_SECRET` env var set (no file, no secret) — returns env value.
   - With nothing set — returns `None`.
   - Priority: file overrides secret, secret overrides env.

5. **Exempt paths tests:**
   - `parse_exempt_paths("/health,/metrics")` returns set of 2.
   - `parse_exempt_paths(" /a , /b ")` trims whitespace.
   - `parse_exempt_paths("")` returns empty set or set with one empty string (decide behavior).

6. **Run `cargo test`.**

## Acceptance Criteria

- [ ] Tests cover all enum variants and invalid values
- [ ] Tests cover all CliArgs defaults
- [ ] Tests cover required arg validation
- [ ] Tests cover port range validation
- [ ] Tests cover `--no-jwt-require-auth` negation
- [ ] Tests cover all validation error cases
- [ ] Tests cover JWT key resolution priority chain
- [ ] Tests cover exempt paths parsing
- [ ] All tests pass with `cargo test`
- [ ] Tests use `tempfile` crate for temporary directories/files

## Dependencies

- implement-run-function

## Estimated Time

1.5 hours
