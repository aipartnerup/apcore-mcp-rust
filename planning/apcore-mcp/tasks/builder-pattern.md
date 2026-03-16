# Task: builder-pattern

## Objective
Rewrite `APCoreMCPBuilder` with all configuration setters, input validation (matching Python), and a `build()` method that resolves the backend and returns `APCoreMCP`.

## Estimate
~1 hr

## TDD Tests (write first)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_requires_backend() {
        let result = APCoreMCP::builder().build();
        assert!(result.is_err()); // no backend set
    }

    #[test]
    fn builder_rejects_empty_name() {
        let result = APCoreMCP::builder()
            .backend("./ext")
            .name("")
            .build();
        assert!(matches!(result, Err(APCoreMCPError::EmptyName)));
    }

    #[test]
    fn builder_rejects_name_over_255() {
        let long = "a".repeat(256);
        let result = APCoreMCP::builder()
            .backend("./ext")
            .name(&long)
            .build();
        assert!(matches!(result, Err(APCoreMCPError::NameTooLong(256))));
    }

    #[test]
    fn builder_rejects_empty_tag() {
        let result = APCoreMCP::builder()
            .backend("./ext")
            .tags(vec!["".to_string()])
            .build();
        assert!(matches!(result, Err(APCoreMCPError::EmptyTag)));
    }

    #[test]
    fn builder_rejects_empty_prefix() {
        let result = APCoreMCP::builder()
            .backend("./ext")
            .prefix("")
            .build();
        assert!(matches!(result, Err(APCoreMCPError::EmptyPrefix)));
    }

    #[test]
    fn builder_rejects_invalid_log_level() {
        let result = APCoreMCP::builder()
            .backend("./ext")
            .log_level("VERBOSE")
            .build();
        assert!(matches!(result, Err(APCoreMCPError::InvalidLogLevel(_))));
    }

    #[test]
    fn builder_accepts_valid_log_levels() {
        for level in &["DEBUG", "INFO", "WARNING", "ERROR", "CRITICAL"] {
            // Should not fail validation (may fail on backend resolution)
            let result = APCoreMCP::builder()
                .backend("./ext")
                .log_level(level)
                .build();
            // We only check it doesn't fail with InvalidLogLevel
            if let Err(e) = &result {
                assert!(!matches!(e, APCoreMCPError::InvalidLogLevel(_)));
            }
        }
    }

    #[test]
    fn builder_sets_all_config_fields() {
        // Verify config flows through (use a mock backend)
        let builder = APCoreMCP::builder()
            .name("test-server")
            .version("1.0.0")
            .tags(vec!["public".into()])
            .prefix("my_")
            .transport("streamable-http")
            .host("0.0.0.0")
            .port(9000)
            .validate_inputs(true)
            .require_auth(false);
        // Check builder state before build
        assert_eq!(builder.config.name, "test-server");
        assert_eq!(builder.config.port, 9000);
    }
}
```

## Implementation Steps
1. Add `backend: Option<BackendSource>` field to `APCoreMCPBuilder`
2. Add `backend(impl Into<BackendSource>) -> Self` setter
3. Add setters for all config fields: `name`, `version`, `tags`, `prefix`, `log_level`, `transport`, `host`, `port`, `validate_inputs`, `metrics_collector`, `authenticator`, `require_auth`, `exempt_paths`, `approval_handler`, `output_formatter`
4. In `build()`: validate all inputs (matching Python validation order), then call `resolve_registry` / `resolve_executor`, construct `APCoreMCP`
5. Store function-like fields as `Option<Box<dyn Fn(...) + Send + Sync>>` in the builder
6. Return `Result<APCoreMCP, APCoreMCPError>`

## Acceptance Criteria
- [ ] All validation rules from Python are enforced
- [ ] Builder is ergonomic with method chaining
- [ ] `build()` fails clearly when backend is not set
- [ ] All optional fields default correctly

## Dependencies
- `config-and-error-types`
- `resolve-utils`

## Files Modified
- `src/apcore_mcp.rs`
