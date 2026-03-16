# Task: update-binary-entrypoint

## Goal

Update `src/bin/apcore-mcp.rs` to use `#[tokio::main]` for async support and properly handle exit codes from `cli::run()`.

## Files Involved

- `src/bin/apcore-mcp.rs` — update main function
- `src/cli.rs` — final cleanup pass

## Steps (TDD-first)

1. **Update `src/bin/apcore-mcp.rs`:**
   ```rust
   #[tokio::main]
   async fn main() {
       match apcore_mcp::cli::run().await {
           Ok(()) => std::process::exit(0),
           Err(e) => {
               eprintln!("Error: {e}");
               std::process::exit(e.exit_code());
           }
       }
   }
   ```
2. **Final cleanup in `src/cli.rs`:**
   - Remove all `#![allow(unused)]` directives.
   - Remove all `todo!()` macros.
   - Ensure all public items have doc comments.
   - Run `cargo clippy` and fix any warnings.
3. **Verify `src/lib.rs`** re-exports are correct for `cli` module.
4. **Run `cargo build` and `cargo test`.**

## Acceptance Criteria

- [ ] Binary entry point uses `#[tokio::main]`
- [ ] Exit codes propagated correctly: 0, 1, 2
- [ ] Error messages printed to stderr
- [ ] No `todo!()` macros remain in `src/cli.rs`
- [ ] No `#![allow(unused)]` in `src/cli.rs`
- [ ] All public items documented
- [ ] `cargo build` succeeds
- [ ] `cargo clippy` passes without warnings
- [ ] `cargo test` passes

## Dependencies

- add-cli-integration-tests

## Estimated Time

20 minutes
