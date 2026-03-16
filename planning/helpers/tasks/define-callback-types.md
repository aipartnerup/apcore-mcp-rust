# Task: Define Callback Types

## Goal

Define the async callback type aliases (`ProgressCallback`, `ElicitCallback`) and finalize the `ElicitAction` enum and `ElicitResult` struct with proper serde attributes and documentation.

## Files Involved

- `src/helpers.rs` — primary implementation file

## Steps (TDD-first)

1. **Write tests first**: Add tests that verify `ElicitAction` serializes to/from `"accept"`, `"decline"`, `"cancel"`. Add tests that verify `ElicitResult` round-trips through serde_json.
2. **Confirm existing types**: The stub already has `ElicitAction` and `ElicitResult`. Verify they have the correct serde attributes (`rename_all = "snake_case"` on the enum, derive `Serialize`/`Deserialize` on both).
3. **Define `ProgressCallback` type alias**:
   ```rust
   use std::future::Future;
   use std::pin::Pin;

   pub type ProgressCallback = Box<
       dyn Fn(f64, Option<f64>, Option<String>) -> Pin<Box<dyn Future<Output = ()> + Send>>
           + Send
           + Sync,
   >;
   ```
4. **Define `ElicitCallback` type alias**:
   ```rust
   pub type ElicitCallback = Box<
       dyn Fn(String, Option<Value>) -> Pin<Box<dyn Future<Output = Option<ElicitResult>> + Send>>
           + Send
           + Sync,
   >;
   ```
5. **Add `JsonSchema` derive** to `ElicitAction` and `ElicitResult` (using `schemars::JsonSchema`) for schema generation support.
6. **Run tests** — confirm serde round-trip tests pass.

## Acceptance Criteria

- [ ] `ElicitAction` has `Accept`, `Decline`, `Cancel` variants with `snake_case` serde rename
- [ ] `ElicitResult` has `action: ElicitAction` and `content: Option<Value>`
- [ ] Both types derive `Debug`, `Clone`, `Serialize`, `Deserialize`, `JsonSchema`
- [ ] `ProgressCallback` type alias is defined and publicly exported
- [ ] `ElicitCallback` type alias is defined and publicly exported
- [ ] Serde round-trip tests pass for all `ElicitAction` variants and `ElicitResult`

## Dependencies

None

## Estimated Time

30 minutes
