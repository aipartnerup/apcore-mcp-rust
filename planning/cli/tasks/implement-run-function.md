# Task: implement-run-function

## Goal

Implement the `run()` function that ties together argument parsing, validation, tracing init, JWT key resolution, approval handler selection, and server startup. This is the main orchestration function.

## Files Involved

- `src/cli.rs` — implement `run()` and private helper functions

## Steps (TDD-first)

1. **Write tests first (see add-cli-unit-tests for full suite, but basic smoke tests here):**
   - `resolve_jwt_key()` with key file returns file contents.
   - `resolve_jwt_key()` with secret returns the secret.
   - `resolve_jwt_key()` with env var returns env value.
   - `resolve_jwt_key()` with nothing returns `None`.
   - `parse_exempt_paths("a,b,c")` returns `{"a", "b", "c"}`.
   - `parse_exempt_paths` trims whitespace.
2. **Implement `resolve_jwt_key(args: &CliArgs) -> Result<Option<String>, CliError>`:**
   - If `args.jwt_key_file` is `Some(path)`:
     - Check path exists, else return error (exit code 1).
     - Read file contents, trim, return.
   - Else if `args.jwt_secret` is `Some(secret)`:
     - Return secret.
   - Else:
     - Return `std::env::var("JWT_SECRET").ok()`.
3. **Implement `parse_exempt_paths(s: &str) -> HashSet<String>`:**
   - Split by `,`, trim each, collect into HashSet.
4. **Implement `validate_args(args: &CliArgs) -> Result<(), CliError>`:**
   - Check `extensions_dir` exists and is a directory.
   - Check `name.len() <= 255`.
   - Return `CliError` with exit code 1 on failure.
5. **Define `CliError` enum:**
   ```rust
   #[derive(Debug, thiserror::Error)]
   pub enum CliError {
       #[error("{0}")]
       InvalidArgs(String),    // exit code 1
       #[error("{0}")]
       StartupFailure(String), // exit code 2
   }
   ```
   With a method `exit_code() -> i32`.
6. **Implement `pub async fn run() -> Result<(), CliError>`:**
   - Parse args via `CliArgs::parse()`.
   - Call `init_tracing(&args.log_level)`.
   - Call `validate_args(&args)?`.
   - Resolve JWT key.
   - Build authenticator if key found (using `JWTAuthenticator`).
   - Parse exempt paths.
   - Build approval handler based on `args.approval`.
   - Resolve server version: `args.version.unwrap_or_else(|| crate::VERSION.to_string())`.
   - Call `serve()` or build `APCoreMCP` and serve.
   - Map serve errors to `CliError::StartupFailure`.
7. **Update the public `run()` signature** from `Result<(), Box<dyn Error>>` to use `CliError` or keep `Box<dyn Error>` and handle exit codes in a wrapper.
8. **Remove all `todo!()` macros and `#![allow(unused)]`.**
9. **Run `cargo check`.**

## Acceptance Criteria

- [ ] `resolve_jwt_key()` implements the priority chain: file > arg > env
- [ ] JWT key file not found produces exit code 1
- [ ] Extensions dir validation: must exist and be a directory (exit code 1)
- [ ] Name length validation: <= 255 characters (exit code 1)
- [ ] `parse_exempt_paths()` splits and trims correctly
- [ ] Approval handler construction matches Python: elicit/auto-approve/always-deny/off
- [ ] Server version defaults to `crate::VERSION` if not provided
- [ ] Server startup errors map to exit code 2
- [ ] Tracing initialized before any logging occurs
- [ ] All `todo!()` removed
- [ ] All `#![allow(unused)]` removed

## Dependencies

- implement-cli-args
- implement-tracing-init

## Estimated Time

1.5 hours
