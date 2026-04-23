//! AnnotationMapper — converts apcore module annotations to MCP tool annotations.

use apcore::module::ModuleAnnotations;
use serde::{Deserialize, Serialize};

/// MCP tool annotations with camelCase field names for JSON serialization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct McpAnnotations {
    pub read_only_hint: bool,
    pub destructive_hint: bool,
    pub idempotent_hint: bool,
    pub open_world_hint: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// Maps apcore module annotations to MCP-compatible tool annotations.
pub struct AnnotationMapper;

impl AnnotationMapper {
    /// Convert apcore annotations to MCP tool annotations.
    ///
    /// When `annotations` is `None`, returns sensible defaults:
    /// `readOnlyHint: false`, `destructiveHint: false`, `idempotentHint: false`,
    /// `openWorldHint: true`, `title: null`.
    pub fn to_mcp_annotations(annotations: Option<&ModuleAnnotations>) -> McpAnnotations {
        match annotations {
            None => McpAnnotations {
                read_only_hint: false,
                destructive_hint: false,
                idempotent_hint: false,
                open_world_hint: true,
                title: None,
            },
            Some(a) => McpAnnotations {
                read_only_hint: a.readonly,
                destructive_hint: a.destructive,
                idempotent_hint: a.idempotent,
                open_world_hint: a.open_world,
                title: None,
            },
        }
    }

    /// Generate annotation text to append to tool descriptions.
    ///
    /// Produces two sections:
    /// 1. Safety warnings for destructive/approval operations.
    /// 2. Machine-readable annotation block for non-default values.
    ///
    /// Returns an empty string if annotations is `None` or all values are defaults.
    pub fn to_description_suffix(annotations: Option<&ModuleAnnotations>) -> String {
        let annotations = match annotations {
            None => return String::new(),
            Some(a) => a,
        };

        let defaults = ModuleAnnotations::default();

        let mut warnings: Vec<String> = Vec::new();
        if annotations.destructive {
            warnings.push(
                "WARNING: DESTRUCTIVE - This operation may irreversibly modify or \
                 delete data. Confirm with user before calling."
                    .to_string(),
            );
        }
        if annotations.requires_approval {
            warnings.push(
                "REQUIRES APPROVAL: Human confirmation is required before execution.".to_string(),
            );
        }

        let mut parts: Vec<String> = Vec::new();
        if annotations.readonly != defaults.readonly {
            parts.push(format!("readonly={}", annotations.readonly));
        }
        if annotations.destructive != defaults.destructive {
            parts.push(format!("destructive={}", annotations.destructive));
        }
        if annotations.idempotent != defaults.idempotent {
            parts.push(format!("idempotent={}", annotations.idempotent));
        }
        if annotations.requires_approval != defaults.requires_approval {
            parts.push(format!(
                "requires_approval={}",
                annotations.requires_approval
            ));
        }
        if annotations.open_world != defaults.open_world {
            parts.push(format!("open_world={}", annotations.open_world));
        }
        if annotations.streaming != defaults.streaming {
            parts.push(format!("streaming={}", annotations.streaming));
        }
        if annotations.cacheable != defaults.cacheable {
            parts.push(format!("cacheable={}", annotations.cacheable));
        }
        if annotations.cache_ttl != defaults.cache_ttl {
            parts.push(format!("cache_ttl={}", annotations.cache_ttl));
        }
        if annotations.cache_key_fields != defaults.cache_key_fields {
            if let Some(fields) = &annotations.cache_key_fields {
                parts.push(format!("cache_key_fields=[{}]", fields.join(",")));
            }
        }
        if annotations.paginated != defaults.paginated {
            parts.push(format!("paginated={}", annotations.paginated));
        }
        if annotations.pagination_style != defaults.pagination_style {
            parts.push(format!("pagination_style={}", annotations.pagination_style));
        }

        // F-041 annotation metadata passthrough: surface any `mcp_` prefixed
        // extension keys from `annotations.extra` verbatim. Keys are sorted so
        // the rendered block is stable across runs. Values are formatted with
        // `serde_json::Value::to_string` which handles scalars and JSON fragments
        // identically.
        let mut mcp_extras: Vec<(&String, &serde_json::Value)> = annotations
            .extra
            .iter()
            .filter(|(k, _)| k.starts_with("mcp_"))
            .collect();
        mcp_extras.sort_by(|a, b| a.0.cmp(b.0));
        for (k, v) in mcp_extras {
            parts.push(format!("{}={}", k, v));
        }

        if warnings.is_empty() && parts.is_empty() {
            return String::new();
        }

        let mut sections: Vec<String> = Vec::new();
        if !warnings.is_empty() {
            sections.push(warnings.join("\n"));
        }
        if !parts.is_empty() {
            sections.push(format!("[Annotations: {}]", parts.join(", ")));
        }

        format!("\n\n{}", sections.join("\n\n"))
    }

    /// Check if module requires human approval before execution.
    ///
    /// Returns `false` if annotations is `None`.
    pub fn has_requires_approval(annotations: Option<&ModuleAnnotations>) -> bool {
        match annotations {
            None => false,
            Some(a) => a.requires_approval,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- to_mcp_annotations tests ----

    #[test]
    fn test_to_mcp_annotations_none() {
        let result = AnnotationMapper::to_mcp_annotations(None);
        assert_eq!(
            result,
            McpAnnotations {
                read_only_hint: false,
                destructive_hint: false,
                idempotent_hint: false,
                open_world_hint: true,
                title: None,
            }
        );
    }

    #[test]
    fn test_to_mcp_annotations_readonly() {
        let ann = ModuleAnnotations {
            readonly: true,
            ..Default::default()
        };
        let result = AnnotationMapper::to_mcp_annotations(Some(&ann));
        assert!(result.read_only_hint);
        assert!(!result.destructive_hint);
    }

    #[test]
    fn test_to_mcp_annotations_destructive() {
        let ann = ModuleAnnotations {
            destructive: true,
            ..Default::default()
        };
        let result = AnnotationMapper::to_mcp_annotations(Some(&ann));
        assert!(result.destructive_hint);
        assert!(!result.read_only_hint);
    }

    #[test]
    fn test_to_mcp_annotations_all_set() {
        let ann = ModuleAnnotations {
            readonly: true,
            destructive: true,
            idempotent: true,
            open_world: false,
            ..Default::default()
        };
        let result = AnnotationMapper::to_mcp_annotations(Some(&ann));
        assert_eq!(
            result,
            McpAnnotations {
                read_only_hint: true,
                destructive_hint: true,
                idempotent_hint: true,
                open_world_hint: false,
                title: None,
            }
        );
    }

    #[test]
    fn test_to_mcp_annotations_serializes_camelcase() {
        let result = AnnotationMapper::to_mcp_annotations(None);
        let json = serde_json::to_value(&result).unwrap();
        assert!(json.get("readOnlyHint").is_some());
        assert!(json.get("destructiveHint").is_some());
        assert!(json.get("idempotentHint").is_some());
        assert!(json.get("openWorldHint").is_some());
        // title should be absent when None due to skip_serializing_if
        assert!(json.get("title").is_none());
    }

    // ---- to_description_suffix tests ----

    #[test]
    fn test_to_description_suffix_none() {
        let result = AnnotationMapper::to_description_suffix(None);
        assert_eq!(result, "");
    }

    #[test]
    fn test_to_description_suffix_destructive() {
        let ann = ModuleAnnotations {
            destructive: true,
            ..Default::default()
        };
        let result = AnnotationMapper::to_description_suffix(Some(&ann));
        assert!(result.contains("DESTRUCTIVE"));
        assert!(result.contains("WARNING"));
    }

    #[test]
    fn test_to_description_suffix_requires_approval() {
        let ann = ModuleAnnotations {
            requires_approval: true,
            ..Default::default()
        };
        let result = AnnotationMapper::to_description_suffix(Some(&ann));
        assert!(result.contains("REQUIRES APPROVAL"));
    }

    #[test]
    fn test_to_description_suffix_non_default_values() {
        let ann = ModuleAnnotations {
            readonly: true,
            streaming: true,
            ..Default::default()
        };
        let result = AnnotationMapper::to_description_suffix(Some(&ann));
        assert!(result.contains("[Annotations:"));
        assert!(result.contains("readonly=true"));
        assert!(result.contains("streaming=true"));
    }

    #[test]
    fn test_to_description_suffix_no_changes() {
        let ann = ModuleAnnotations::default();
        let result = AnnotationMapper::to_description_suffix(Some(&ann));
        assert_eq!(result, "");
    }

    #[test]
    fn test_to_description_suffix_destructive_and_approval() {
        let ann = ModuleAnnotations {
            destructive: true,
            requires_approval: true,
            ..Default::default()
        };
        let result = AnnotationMapper::to_description_suffix(Some(&ann));
        assert!(result.contains("DESTRUCTIVE"));
        assert!(result.contains("REQUIRES APPROVAL"));
        assert!(result.contains("[Annotations:"));
        assert!(result.contains("destructive=true"));
        assert!(result.contains("requires_approval=true"));
        // Verify it starts with \n\n
        assert!(result.starts_with("\n\n"));
    }

    // ---- has_requires_approval tests ----

    #[test]
    fn test_has_requires_approval_none() {
        assert!(!AnnotationMapper::has_requires_approval(None));
    }

    #[test]
    fn test_has_requires_approval_true() {
        let ann = ModuleAnnotations {
            requires_approval: true,
            ..Default::default()
        };
        assert!(AnnotationMapper::has_requires_approval(Some(&ann)));
    }

    #[test]
    fn test_has_requires_approval_false() {
        let ann = ModuleAnnotations::default();
        assert!(!AnnotationMapper::has_requires_approval(Some(&ann)));
    }
}
