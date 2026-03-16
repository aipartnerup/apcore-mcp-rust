# Task: config-and-error-types

## Objective
Expand `APCoreMCPConfig` to cover all Python constructor parameters and define a proper `APCoreMCPError` error type using `thiserror`.

## Estimate
~30 min

## TDD Tests (write first)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_expected_values() {
        let cfg = APCoreMCPConfig::default();
        assert_eq!(cfg.name, "apcore-mcp");
        assert_eq!(cfg.host, "127.0.0.1");
        assert_eq!(cfg.port, 8000);
        assert_eq!(cfg.transport, "stdio");
        assert!(!cfg.validate_inputs);
        assert!(cfg.version.is_none());
        assert!(cfg.tags.is_none());
        assert!(cfg.prefix.is_none());
        assert!(cfg.require_auth);
    }

    #[test]
    fn error_display_empty_name() {
        let err = APCoreMCPError::EmptyName;
        assert!(err.to_string().contains("name"));
    }

    #[test]
    fn error_display_name_too_long() {
        let err = APCoreMCPError::NameTooLong(300);
        assert!(err.to_string().contains("255"));
    }

    #[test]
    fn error_display_empty_tag() {
        let err = APCoreMCPError::EmptyTag;
        assert!(err.to_string().contains("tag"));
    }

    #[test]
    fn error_display_invalid_log_level() {
        let err = APCoreMCPError::InvalidLogLevel("VERBOSE".into());
        assert!(err.to_string().contains("VERBOSE"));
    }
}
```

## Implementation Steps
1. Expand `APCoreMCPConfig` with fields matching Python constructor:
   - `name: String`
   - `version: Option<String>`
   - `transport: String`
   - `host: String`
   - `port: u16`
   - `tags: Option<Vec<String>>`
   - `prefix: Option<String>`
   - `log_level: Option<String>`
   - `validate_inputs: bool`
   - `require_auth: bool`
   - `exempt_paths: Option<HashSet<String>>`
   - `explorer: bool`
   - `explorer_prefix: String`
2. Set defaults matching Python: port=8000, validate_inputs=false, require_auth=true
3. Define `APCoreMCPError` enum with `thiserror::Error`:
   - `EmptyName`
   - `NameTooLong(usize)`
   - `EmptyTag`
   - `EmptyPrefix`
   - `InvalidLogLevel(String)`
   - `InvalidExplorerPrefix`
   - `BackendResolution(String)`
   - `ServerError(String)`

## Acceptance Criteria
- [ ] `APCoreMCPConfig::default()` matches Python defaults
- [ ] All error variants display meaningful messages
- [ ] Types are `Debug`, `Clone` where appropriate

## Dependencies
- `backend-source-enum`

## Files Modified
- `src/apcore_mcp.rs`
