//! SchemaConverter — converts between apcore JSON schemas and MCP tool schemas.

use std::collections::HashSet;

use serde_json::{json, Value};

use super::AdapterError;

const MAX_REF_DEPTH: usize = 32;

/// Converts apcore module input/output schemas to MCP-compatible JSON Schema.
///
/// Key transformations:
/// - Empty/null schemas become `{"type": "object", "properties": {}}`
/// - `$ref` references are inlined recursively (up to depth 32)
/// - `$defs` is stripped from the final output
/// - Root schema is guaranteed to have `"type": "object"`
pub struct SchemaConverter;

impl SchemaConverter {
    /// Convert an apcore input schema to an MCP tool input schema.
    ///
    /// MCP requires `type: "object"` at the top level with explicit `properties`.
    pub fn convert_input_schema(schema: &Value) -> Result<Value, AdapterError> {
        Self::convert_schema(schema)
    }

    /// Convert an apcore output schema to an MCP-compatible output description.
    pub fn convert_output_schema(schema: &Value) -> Result<Value, AdapterError> {
        Self::convert_schema(schema)
    }

    fn convert_schema(schema: &Value) -> Result<Value, AdapterError> {
        // Clone to avoid mutating input
        let mut schema = schema.clone();

        // Handle null or empty object
        if schema.is_null() || schema.as_object().is_some_and(|m| m.is_empty()) {
            return Ok(json!({"type": "object", "properties": {}}));
        }

        // Inline $refs if $defs present
        if let Some(defs) = schema.get("$defs").cloned() {
            schema = Self::inline_refs(&schema, &defs, &HashSet::new(), 0)?;
            if let Some(obj) = schema.as_object_mut() {
                obj.remove("$defs");
            }
        }

        // Ensure root type: object
        Self::ensure_object_type(&mut schema);

        Ok(schema)
    }

    fn inline_refs(
        schema: &Value,
        defs: &Value,
        seen: &HashSet<String>,
        depth: usize,
    ) -> Result<Value, AdapterError> {
        if depth > MAX_REF_DEPTH {
            return Err(AdapterError::SchemaConversion(format!(
                "$ref resolution exceeded maximum depth of {MAX_REF_DEPTH}"
            )));
        }

        match schema {
            Value::Object(map) => {
                // If this is a $ref, resolve it
                if let Some(ref_val) = map.get("$ref") {
                    let ref_path = ref_val.as_str().ok_or_else(|| {
                        AdapterError::SchemaConversion("$ref value must be a string".into())
                    })?;

                    if seen.contains(ref_path) {
                        return Err(AdapterError::SchemaConversion(format!(
                            "Circular $ref detected: {ref_path}"
                        )));
                    }

                    let mut new_seen = seen.clone();
                    new_seen.insert(ref_path.to_string());

                    let resolved = Self::resolve_ref(ref_path, defs)?;
                    return Self::inline_refs(&resolved, defs, &new_seen, depth + 1);
                }

                // Otherwise, recursively process all values
                let mut result = serde_json::Map::new();
                for (key, value) in map {
                    if key == "$defs" {
                        continue;
                    }
                    result.insert(
                        key.clone(),
                        Self::inline_refs(value, defs, seen, depth + 1)?,
                    );
                }
                Ok(Value::Object(result))
            }
            Value::Array(arr) => {
                let items: Result<Vec<Value>, _> = arr
                    .iter()
                    .map(|item| Self::inline_refs(item, defs, seen, depth + 1))
                    .collect();
                Ok(Value::Array(items?))
            }
            // Primitive value, return as-is
            other => Ok(other.clone()),
        }
    }

    fn resolve_ref(ref_path: &str, defs: &Value) -> Result<Value, AdapterError> {
        if !ref_path.starts_with("#/$defs/") {
            return Err(AdapterError::SchemaConversion(format!(
                "Unsupported $ref format: {ref_path}"
            )));
        }

        let def_name = &ref_path[8..]; // Remove "#/$defs/"

        let defs_map = defs
            .as_object()
            .ok_or_else(|| AdapterError::SchemaConversion("$defs must be an object".into()))?;

        defs_map.get(def_name).cloned().ok_or_else(|| {
            AdapterError::SchemaConversion(format!("Definition not found: {def_name}"))
        })
    }

    fn ensure_object_type(schema: &mut Value) {
        if let Some(map) = schema.as_object_mut() {
            if !map.contains_key("type") {
                map.insert("type".to_string(), json!("object"));
            }
            if map.contains_key("properties") && map.get("type") != Some(&json!("object")) {
                map.insert("type".to_string(), json!("object"));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_empty_schema() {
        let schema = json!({});
        let result = SchemaConverter::convert_input_schema(&schema).unwrap();
        assert_eq!(result, json!({"type": "object", "properties": {}}));
    }

    #[test]
    fn test_null_schema() {
        let result = SchemaConverter::convert_input_schema(&Value::Null).unwrap();
        assert_eq!(result, json!({"type": "object", "properties": {}}));
    }

    #[test]
    fn test_simple_object_passthrough() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        });
        let result = SchemaConverter::convert_input_schema(&schema).unwrap();
        assert_eq!(result, schema);
    }

    #[test]
    fn test_missing_type_gets_object() {
        let schema = json!({
            "properties": {
                "name": {"type": "string"}
            }
        });
        let result = SchemaConverter::convert_input_schema(&schema).unwrap();
        assert_eq!(result["type"], "object");
        assert_eq!(result["properties"]["name"]["type"], "string");
    }

    #[test]
    fn test_inline_simple_ref() {
        let schema = json!({
            "type": "object",
            "properties": {
                "step": {"$ref": "#/$defs/Step"}
            },
            "$defs": {
                "Step": {
                    "type": "object",
                    "properties": {
                        "id": {"type": "integer"}
                    }
                }
            }
        });
        let result = SchemaConverter::convert_input_schema(&schema).unwrap();
        assert_eq!(result["properties"]["step"]["type"], "object");
        assert_eq!(
            result["properties"]["step"]["properties"]["id"]["type"],
            "integer"
        );
        assert!(result.get("$defs").is_none());
    }

    #[test]
    fn test_inline_nested_refs() {
        let schema = json!({
            "type": "object",
            "properties": {
                "outer": {"$ref": "#/$defs/Outer"}
            },
            "$defs": {
                "Outer": {
                    "type": "object",
                    "properties": {
                        "inner": {"$ref": "#/$defs/Inner"}
                    }
                },
                "Inner": {
                    "type": "object",
                    "properties": {
                        "value": {"type": "string"}
                    }
                }
            }
        });
        let result = SchemaConverter::convert_input_schema(&schema).unwrap();
        assert_eq!(
            result["properties"]["outer"]["properties"]["inner"]["properties"]["value"]["type"],
            "string"
        );
        assert!(result.get("$defs").is_none());
    }

    #[test]
    fn test_inline_ref_in_properties() {
        let schema = json!({
            "type": "object",
            "properties": {
                "config": {"$ref": "#/$defs/Config"}
            },
            "$defs": {
                "Config": {
                    "type": "object",
                    "properties": {
                        "enabled": {"type": "boolean"}
                    }
                }
            }
        });
        let result = SchemaConverter::convert_input_schema(&schema).unwrap();
        assert_eq!(
            result["properties"]["config"]["properties"]["enabled"]["type"],
            "boolean"
        );
    }

    #[test]
    fn test_inline_ref_in_array_items() {
        let schema = json!({
            "type": "object",
            "properties": {
                "steps": {
                    "type": "array",
                    "items": {"$ref": "#/$defs/Step"}
                }
            },
            "$defs": {
                "Step": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"}
                    }
                }
            }
        });
        let result = SchemaConverter::convert_input_schema(&schema).unwrap();
        assert_eq!(result["properties"]["steps"]["items"]["type"], "object");
        assert_eq!(
            result["properties"]["steps"]["items"]["properties"]["name"]["type"],
            "string"
        );
    }

    #[test]
    fn test_defs_stripped_after_inlining() {
        let schema = json!({
            "type": "object",
            "properties": {
                "x": {"$ref": "#/$defs/X"}
            },
            "$defs": {
                "X": {"type": "string"}
            }
        });
        let result = SchemaConverter::convert_input_schema(&schema).unwrap();
        assert!(result.get("$defs").is_none());
    }

    #[test]
    fn test_circular_ref_detected() {
        let schema = json!({
            "type": "object",
            "properties": {
                "node": {"$ref": "#/$defs/Node"}
            },
            "$defs": {
                "Node": {
                    "type": "object",
                    "properties": {
                        "child": {"$ref": "#/$defs/Node"}
                    }
                }
            }
        });
        let result = SchemaConverter::convert_input_schema(&schema);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Circular $ref detected"));
    }

    #[test]
    fn test_depth_exceeded() {
        // Build a chain of 34 $defs, each referencing the next
        let mut defs = serde_json::Map::new();
        for i in 0..34 {
            let next = if i < 33 {
                json!({"$ref": format!("#/$defs/D{}", i + 1)})
            } else {
                json!({"type": "string"})
            };
            defs.insert(format!("D{i}"), next);
        }
        let schema = json!({
            "type": "object",
            "properties": {
                "x": {"$ref": "#/$defs/D0"}
            },
            "$defs": Value::Object(defs)
        });
        let result = SchemaConverter::convert_input_schema(&schema);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("exceeded maximum depth"));
    }

    #[test]
    fn test_diamond_ref_allowed() {
        let schema = json!({
            "type": "object",
            "properties": {
                "a": {"$ref": "#/$defs/Shared"},
                "b": {"$ref": "#/$defs/Shared"}
            },
            "$defs": {
                "Shared": {
                    "type": "object",
                    "properties": {
                        "val": {"type": "integer"}
                    }
                }
            }
        });
        let result = SchemaConverter::convert_input_schema(&schema).unwrap();
        assert_eq!(
            result["properties"]["a"]["properties"]["val"]["type"],
            "integer"
        );
        assert_eq!(
            result["properties"]["b"]["properties"]["val"]["type"],
            "integer"
        );
    }

    #[test]
    fn test_ref_not_found() {
        let schema = json!({
            "type": "object",
            "properties": {
                "x": {"$ref": "#/$defs/Missing"}
            },
            "$defs": {}
        });
        let result = SchemaConverter::convert_input_schema(&schema);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Definition not found: Missing"));
    }

    #[test]
    fn test_unsupported_ref_format() {
        let schema = json!({
            "type": "object",
            "properties": {
                "x": {"$ref": "http://example.com/schema"}
            },
            "$defs": {}
        });
        let result = SchemaConverter::convert_input_schema(&schema);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Unsupported $ref format"));
    }

    #[test]
    fn test_preserves_additional_properties() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            },
            "required": ["name"],
            "additionalProperties": false
        });
        let result = SchemaConverter::convert_input_schema(&schema).unwrap();
        assert_eq!(result["required"], json!(["name"]));
        assert_eq!(result["additionalProperties"], json!(false));
    }

    #[test]
    fn test_convert_input_schema() {
        let schema = json!({
            "type": "object",
            "properties": {
                "query": {"type": "string"},
                "config": {"$ref": "#/$defs/Config"}
            },
            "required": ["query"],
            "$defs": {
                "Config": {
                    "type": "object",
                    "properties": {
                        "limit": {"type": "integer"}
                    }
                }
            }
        });
        let result = SchemaConverter::convert_input_schema(&schema).unwrap();
        assert_eq!(result["type"], "object");
        assert_eq!(result["properties"]["query"]["type"], "string");
        assert_eq!(
            result["properties"]["config"]["properties"]["limit"]["type"],
            "integer"
        );
        assert_eq!(result["required"], json!(["query"]));
        assert!(result.get("$defs").is_none());
    }

    #[test]
    fn test_convert_output_schema() {
        let schema = json!({
            "type": "object",
            "properties": {
                "result": {"type": "string"},
                "metadata": {"$ref": "#/$defs/Meta"}
            },
            "$defs": {
                "Meta": {
                    "type": "object",
                    "properties": {
                        "timestamp": {"type": "string"}
                    }
                }
            }
        });
        let result = SchemaConverter::convert_output_schema(&schema).unwrap();
        assert_eq!(result["type"], "object");
        assert_eq!(result["properties"]["result"]["type"], "string");
        assert_eq!(
            result["properties"]["metadata"]["properties"]["timestamp"]["type"],
            "string"
        );
        assert!(result.get("$defs").is_none());
    }

    #[test]
    fn test_original_not_mutated() {
        let schema = json!({
            "type": "object",
            "properties": {
                "x": {"$ref": "#/$defs/X"}
            },
            "$defs": {
                "X": {"type": "string"}
            }
        });
        let original = schema.clone();
        let _ = SchemaConverter::convert_input_schema(&schema).unwrap();
        assert_eq!(schema, original, "Original schema must not be mutated");
    }
}
