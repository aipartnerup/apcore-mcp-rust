# Task: resolve-utils

## Objective
Update `resolve_registry` and `resolve_executor` in `src/utils.rs` to accept `BackendSource` and return typed `Arc<Registry>` / `Arc<Executor>`.

## Estimate
~45 min

## TDD Tests (write first)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn resolve_registry_from_registry_returns_same() {
        // Given an Arc<Registry>, resolving should return it directly
        let reg = Arc::new(Registry::new(/* test config */));
        let source = BackendSource::Registry(reg.clone());
        let resolved = resolve_registry(&source).unwrap();
        assert!(Arc::ptr_eq(&reg, &resolved));
    }

    #[test]
    fn resolve_executor_from_executor_returns_same() {
        let exec = Arc::new(Executor::new(/* test config */));
        let source = BackendSource::Executor(exec.clone());
        let resolved = resolve_executor(&source, None).unwrap();
        assert!(Arc::ptr_eq(&exec, &resolved));
    }

    #[test]
    fn resolve_from_extensions_dir_discovers_registry() {
        // Requires a temp directory with valid extension structure
        // or a mock — test that ExtensionsDir variant triggers discover()
    }
}
```

## Implementation Steps
1. Change `resolve_registry` signature: `fn resolve_registry(source: &BackendSource) -> Result<Arc<Registry>, APCoreMCPError>`
2. Change `resolve_executor` signature: `fn resolve_executor(source: &BackendSource, approval_handler: Option<Arc<dyn ApprovalHandler>>) -> Result<Arc<Executor>, APCoreMCPError>`
3. For `ExtensionsDir`: create `Registry` from path, call `discover()`, wrap in `Arc`; create `Executor` from the registry
4. For `Registry`: return the `Arc` directly; for `Executor`, build executor from registry
5. For `Executor`: extract registry from executor (if possible) or error; return the `Arc` directly

## Acceptance Criteria
- [ ] Pass-through cases (Registry/Executor) work without I/O
- [ ] ExtensionsDir case creates and discovers
- [ ] Error cases produce clear `APCoreMCPError::BackendResolution` messages

## Dependencies
- `backend-source-enum`
- `config-and-error-types`

## Files Modified
- `src/utils.rs`
