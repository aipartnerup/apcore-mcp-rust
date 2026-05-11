# Task: implement-tracing-init

## Goal

Implement a function to initialize the tracing subscriber based on the `LogLevel` enum, equivalent to Python's `logging.basicConfig()`.

## Files Involved

- `src/cli.rs` — add `init_tracing()` function

## Steps (TDD-first)

1. **Write tests first:**
   - `init_tracing(LogLevel::Debug)` does not panic.
   - `init_tracing(LogLevel::Info)` does not panic.
   - After init, tracing macros execute without panic.
2. **Implement `init_tracing(level: &LogLevel)`:**
   ```rust
   fn init_tracing(level: &LogLevel) {
       use tracing_subscriber::{fmt, EnvFilter};

       let filter = EnvFilter::new(level.to_filter_str());

       fmt()
           .with_env_filter(filter)
           .with_target(true)
           .with_thread_ids(false)
           .with_file(false)
           .init();
   }
   ```
3. **Implement `LogLevel::to_filter_str()`** returning the tracing-compatible string:
   - Debug -> "debug"
   - Info -> "info"
   - Warning -> "warn"
   - Error -> "error"
4. **Run `cargo check`.**

## Acceptance Criteria

- [ ] `init_tracing()` creates a `tracing_subscriber` with the correct level filter
- [ ] Uses `EnvFilter` for flexibility (allows RUST_LOG override)
- [ ] Output format includes timestamp and target (similar to Python's `%(asctime)s [%(levelname)s] %(name)s: %(message)s`)
- [ ] Does not panic on any valid LogLevel
- [ ] Function is idempotent-safe (called once at startup)

## Dependencies

- none (uses LogLevel enum, but can be developed with a simple string-based version first)

## Estimated Time

30 minutes
