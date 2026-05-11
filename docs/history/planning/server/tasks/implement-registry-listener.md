# Task: implement-registry-listener

## Goal

Implement `RegistryListener` that subscribes to apcore Registry register/unregister events and maintains a thread-safe tool map via `RwLock<HashMap<String, Value>>`.

## Files Involved

- `src/server/listener.rs` — full implementation of `RegistryListener`

## Steps (TDD-first)

1. **Write tests first** (see task `add-registry-listener-tests` for comprehensive tests, but write smoke tests here):
   - `tools()` returns empty map on fresh listener.
   - After simulating a register event, `tools()` contains the tool.
   - After simulating an unregister event, `tools()` no longer contains the tool.
   - `start()` is idempotent (calling twice does not panic or double-subscribe).
   - `stop()` causes subsequent events to be ignored.
2. **Define inner state:**
   ```rust
   use std::collections::HashMap;
   use std::sync::{Arc, RwLock, atomic::{AtomicBool, Ordering}};
   use serde_json::Value;

   pub struct RegistryListener {
       tools: Arc<RwLock<HashMap<String, Value>>>,
       active: Arc<AtomicBool>,
       // registry and factory references (trait objects or concrete types)
   }
   ```
3. **Implement `RegistryListener::new(registry, factory)`:**
   - Accept references to a registry and factory (use trait objects or concrete types based on `apcore` crate API).
   - Initialize `tools` as empty `RwLock<HashMap>`.
   - Initialize `active` as `AtomicBool::new(false)`.
4. **Implement `start()`:**
   - Check `active` with `compare_exchange` for idempotency.
   - Register callbacks on registry for "register" and "unregister" events.
   - Use `Arc::clone` of self's inner state for the callbacks.
5. **Implement `stop()`:**
   - Set `active` to `false` via `AtomicBool::store`.
   - Subsequent callback invocations check `active` and no-op if false.
6. **Implement `tools() -> HashMap<String, Value>`:**
   - Acquire read lock on `self.tools`.
   - Clone and return the HashMap.
7. **Implement `_on_register(module_id, module)` callback logic:**
   - Check `active` flag; return early if false.
   - Call `registry.get_definition(module_id)`.
   - If `None`, log warning and return.
   - Call `factory.build_tool(descriptor)`.
   - Acquire write lock, insert tool.
   - Log info: "Tool registered: {module_id}".
   - Catch errors from `build_tool`, log warning on failure.
8. **Implement `_on_unregister(module_id, module)` callback logic:**
   - Check `active` flag; return early if false.
   - Acquire write lock, remove `module_id`.
   - If removed, log info: "Tool unregistered: {module_id}".
9. **Remove `#![allow(unused)]`** and all `todo!()` macros.
10. **Run `cargo check` and `cargo test`.**

## Acceptance Criteria

- [ ] `tools()` returns a cloned snapshot (not a reference to internal state)
- [ ] `tools()` is thread-safe via `RwLock` read lock
- [ ] Register callback adds tool to map via write lock
- [ ] Unregister callback removes tool from map via write lock
- [ ] `start()` is idempotent (second call is a no-op)
- [ ] `stop()` causes callbacks to no-op
- [ ] Missing definition during register logs a warning and does not panic
- [ ] Failed `build_tool` logs a warning and does not panic
- [ ] All `todo!()` macros removed

## Dependencies

None

## Estimated Time

1.5 hours
