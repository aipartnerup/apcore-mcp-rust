# Task: setup

## Goal

Prepare the project for the constants rewrite: add the `strum` dependency, strip the existing stub down to a clean starting point, and establish the test scaffolding.

## Files Involved

- `Cargo.toml` -- add `strum` and `strum_macros` dependencies
- `src/constants.rs` -- remove current placeholder content, add module-level doc comment and imports
- `src/lib.rs` -- verify `pub mod constants;` is present

## Steps

1. **Add strum dependency**
   ```bash
   # In Cargo.toml [dependencies] section, add:
   # strum = "0.26"
   # strum_macros = "0.26"
   cargo check  # verify it compiles
   ```

2. **Clean up `src/constants.rs`**
   Replace the current stub with:
   ```rust
   //! Constants used throughout the apcore-mcp bridge.
   //!
   //! Provides [`ErrorCode`], [`RegistryEvent`], and module ID validation.

   use std::sync::LazyLock;
   use regex::Regex;
   use serde::{Deserialize, Serialize};
   use strum_macros::{Display, EnumString, EnumIter, IntoStaticStr};

   // Implementations will be added in subsequent tasks.
   ```

3. **Write a skeleton test module**
   At the bottom of `src/constants.rs`:
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;
       // Tests will be added per-task.
   }
   ```

4. **Verify**
   ```bash
   cargo check
   cargo test -- constants  # should pass (no tests yet, but no compile errors)
   ```

## Acceptance Criteria

- [ ] `strum = "0.26"` and `strum_macros = "0.26"` are in `[dependencies]`
- [ ] `src/constants.rs` compiles with the new imports
- [ ] `cargo check` succeeds with no errors
- [ ] Existing modules that depend on old `constants` exports are updated or still compile

## Dependencies

- **Depends on:** nothing
- **Required by:** error-codes, registry-events, patterns

## Estimated Time

10 minutes
