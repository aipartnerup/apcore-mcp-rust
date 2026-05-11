# Task: backend-source-enum

## Objective
Define the `BackendSource` enum that replaces Python's polymorphic `str | Path | Registry | Executor` constructor parameter.

## Estimate
~30 min

## TDD Tests (write first)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn from_string_creates_extensions_dir() {
        let source: BackendSource = "./extensions".into();
        assert!(matches!(source, BackendSource::ExtensionsDir(_)));
    }

    #[test]
    fn from_pathbuf_creates_extensions_dir() {
        let source: BackendSource = PathBuf::from("./extensions").into();
        assert!(matches!(source, BackendSource::ExtensionsDir(_)));
    }

    #[test]
    fn from_str_ref_creates_extensions_dir() {
        let source = BackendSource::from("./my-ext");
        if let BackendSource::ExtensionsDir(p) = source {
            assert_eq!(p, PathBuf::from("./my-ext"));
        } else {
            panic!("expected ExtensionsDir");
        }
    }

    // Registry/Executor From impls tested once those types are available
}
```

## Implementation Steps
1. Define `BackendSource` enum with three variants: `ExtensionsDir(PathBuf)`, `Registry(Arc<Registry>)`, `Executor(Arc<Executor>)`
2. Implement `From<&str>`, `From<String>`, `From<PathBuf>` for `BackendSource` (all map to `ExtensionsDir`)
3. Implement `From<Arc<Registry>>` and `From<Arc<Executor>>` for `BackendSource`
4. Add `Debug` derive (manual impl if needed for trait objects)
5. Place in `src/apcore_mcp.rs` near the top

## Acceptance Criteria
- [ ] All `From` impls compile and pass tests
- [ ] Enum is public and documented
- [ ] Can be used with `impl Into<BackendSource>` in builder methods

## Dependencies
None (first task)

## Files Modified
- `src/apcore_mcp.rs`
