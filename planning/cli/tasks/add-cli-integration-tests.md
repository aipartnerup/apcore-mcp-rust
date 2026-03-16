# Task: add-cli-integration-tests

## Goal

Write integration tests that exercise the CLI binary with various argument combinations, verifying exit codes and error messages match the specification.

## Files Involved

- `tests/cli_integration.rs` — new integration test file

## Steps (TDD-first)

1. **Write tests using `std::process::Command` to invoke the binary:**
   - Missing `--extensions-dir` — exit code 2 (clap error).
   - `--extensions-dir /nonexistent` — exit code 1 (validation error).
   - `--extensions-dir <tmpdir>` with valid dir — verify startup begins (may need to send signal to stop).
   - `--port 0` — exit code 2 (clap validation error).
   - Name > 255 chars — exit code 1.
   - `--help` — exit code 0 and contains expected help text.

2. **Test exit code contract:**
   - Exit code 0: normal (hard to test without a running server, may skip).
   - Exit code 1: invalid args after parse (extensions dir, name, jwt key file).
   - Exit code 2: startup failure or clap parse error.

3. **Test `--jwt-key-file` with non-existent file:**
   - Should produce exit code 1 with error message.

4. **Run `cargo test --test cli_integration`.**

## Acceptance Criteria

- [ ] Integration tests invoke the actual binary via `Command`
- [ ] Exit code 1 verified for validation errors
- [ ] Exit code 2 verified for clap parse errors
- [ ] `--help` produces useful output
- [ ] Tests use `tempfile` for temporary directories
- [ ] Tests are `#[ignore]`-gated if they require the binary to be built first (or use `cargo build` in test setup)

## Dependencies

- add-cli-unit-tests

## Estimated Time

1 hour
