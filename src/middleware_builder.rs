// Build apcore middleware instances from Config Bus `mcp.middleware` entries.
//
// Config Bus schema (YAML):
//
// ```yaml
// mcp:
//   middleware:
//     - type: retry
//       max_retries: 3
//       strategy: exponential
//       base_delay_ms: 100
//       max_delay_ms: 5000
//       jitter: true
//     - type: logging
//       log_inputs: true
//       log_outputs: true
//       log_errors: true
//     - type: error_history
//       max_entries_per_module: 50
//       max_total_entries: 5000
// ```
//
// Mirrors the Python `middleware_builder.build_middleware_from_config` and the
// TypeScript `buildMiddlewareFromConfig` contracts. Each entry's `type` selects
// a built-in apcore middleware; remaining keys are deserialised into the
// corresponding constructor. Unknown `type` returns an error so misconfiguration
// fails loudly at startup.

use apcore::{
    ErrorHistory, ErrorHistoryMiddleware, LoggingMiddleware, Middleware, RetryConfig,
    RetryMiddleware,
};
use serde_json::Value;

use crate::apcore_mcp::APCoreMCPError;

/// Construct apcore middleware instances from a Config Bus value.
///
/// `entries` is typically the JSON array returned by
/// `Config::get("mcp.middleware")`. Returns an empty `Vec` when `entries` is
/// `None` or an empty array.
pub fn build_middleware_from_config(
    entries: Option<&Value>,
) -> Result<Vec<Box<dyn Middleware>>, APCoreMCPError> {
    let array = match entries {
        Some(Value::Array(arr)) if !arr.is_empty() => arr,
        _ => return Ok(Vec::new()),
    };

    let mut out: Vec<Box<dyn Middleware>> = Vec::with_capacity(array.len());

    for (idx, entry) in array.iter().enumerate() {
        let obj = entry.as_object().ok_or_else(|| {
            APCoreMCPError::Config(format!(
                "mcp.middleware[{idx}] must be an object with a 'type' key"
            ))
        })?;

        let mw_type = obj
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                APCoreMCPError::Config(format!("mcp.middleware[{idx}] missing required 'type' key"))
            })?
            .to_string();

        let mw: Box<dyn Middleware> = match mw_type.as_str() {
            "retry" => {
                // Build a RetryConfig from the entry, stripping the `type` key.
                let mut kwargs = obj.clone();
                kwargs.remove("type");
                let cfg: RetryConfig = if kwargs.is_empty() {
                    RetryConfig::default()
                } else {
                    // `max_retries` is required by the struct; fall back to default if missing.
                    if !kwargs.contains_key("max_retries") {
                        kwargs.insert(
                            "max_retries".to_string(),
                            Value::from(RetryConfig::default().max_retries),
                        );
                    }
                    serde_json::from_value(Value::Object(kwargs)).map_err(|e| {
                        APCoreMCPError::Config(format!(
                            "mcp.middleware[{idx}] (retry) invalid config: {e}"
                        ))
                    })?
                };
                Box::new(RetryMiddleware::new(cfg))
            }
            "logging" => {
                // LoggingMiddleware::new(log_inputs, log_outputs, log_errors)
                let log_inputs = obj
                    .get("log_inputs")
                    .and_then(Value::as_bool)
                    .unwrap_or(true);
                let log_outputs = obj
                    .get("log_outputs")
                    .and_then(Value::as_bool)
                    .unwrap_or(true);
                let log_errors = obj
                    .get("log_errors")
                    .and_then(Value::as_bool)
                    .unwrap_or(true);
                let extra: Vec<&str> = obj
                    .keys()
                    .filter(|k: &&String| {
                        !matches!(
                            k.as_str(),
                            "type" | "log_inputs" | "log_outputs" | "log_errors"
                        )
                    })
                    .map(String::as_str)
                    .collect();
                if !extra.is_empty() {
                    return Err(APCoreMCPError::Config(format!(
                        "mcp.middleware[{idx}] (logging) got unexpected keys: {}",
                        extra.join(", ")
                    )));
                }
                Box::new(LoggingMiddleware::new(log_inputs, log_outputs, log_errors))
            }
            "error_history" => {
                let max_per = obj
                    .get("max_entries_per_module")
                    .and_then(Value::as_u64)
                    .map(|n| n as usize);
                let max_total = obj
                    .get("max_total_entries")
                    .and_then(Value::as_u64)
                    .map(|n| n as usize);
                let extra: Vec<&str> = obj
                    .keys()
                    .filter(|k: &&String| {
                        !matches!(
                            k.as_str(),
                            "type" | "max_entries_per_module" | "max_total_entries"
                        )
                    })
                    .map(String::as_str)
                    .collect();
                if !extra.is_empty() {
                    return Err(APCoreMCPError::Config(format!(
                        "mcp.middleware[{idx}] (error_history) got unexpected keys: {}",
                        extra.join(", ")
                    )));
                }
                let history = match (max_per, max_total) {
                    (Some(p), Some(t)) => ErrorHistory::with_limits(p, t),
                    (Some(p), None) => ErrorHistory::new(p),
                    (None, Some(t)) => ErrorHistory::with_limits(50, t),
                    (None, None) => ErrorHistory::new(50),
                };
                Box::new(ErrorHistoryMiddleware::new(history))
            }
            other => {
                return Err(APCoreMCPError::Config(format!(
                    "mcp.middleware[{idx}] unknown type '{other}'. \
                     Known built-in types: retry, logging, error_history"
                )));
            }
        };

        out.push(mw);
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn empty_or_missing_returns_empty_vec() {
        assert!(build_middleware_from_config(None).unwrap().is_empty());
        assert!(build_middleware_from_config(Some(&json!([])))
            .unwrap()
            .is_empty());
    }

    #[test]
    fn builds_retry_with_defaults() {
        let entries = json!([{"type": "retry"}]);
        let mws = build_middleware_from_config(Some(&entries)).unwrap();
        assert_eq!(mws.len(), 1);
        assert_eq!(mws[0].name(), "retry");
    }

    #[test]
    fn builds_retry_with_custom_config() {
        let entries = json!([{
            "type": "retry",
            "max_retries": 5,
            "base_delay_ms": 200
        }]);
        let mws = build_middleware_from_config(Some(&entries)).unwrap();
        assert_eq!(mws.len(), 1);
        assert_eq!(mws[0].name(), "retry");
    }

    #[test]
    fn builds_logging_with_defaults() {
        let entries = json!([{"type": "logging"}]);
        let mws = build_middleware_from_config(Some(&entries)).unwrap();
        assert_eq!(mws.len(), 1);
        assert_eq!(mws[0].name(), "logging");
    }

    #[test]
    fn builds_error_history_with_shorthand_keys() {
        let entries = json!([{
            "type": "error_history",
            "max_entries_per_module": 25,
            "max_total_entries": 500
        }]);
        let mws = build_middleware_from_config(Some(&entries)).unwrap();
        assert_eq!(mws.len(), 1);
        assert_eq!(mws[0].name(), "error_history");
    }

    #[test]
    fn preserves_order_across_multiple_entries() {
        let entries = json!([
            {"type": "retry"},
            {"type": "logging"}
        ]);
        let mws = build_middleware_from_config(Some(&entries)).unwrap();
        assert_eq!(mws[0].name(), "retry");
        assert_eq!(mws[1].name(), "logging");
    }

    #[test]
    fn unknown_type_returns_error() {
        let entries = json!([{"type": "bogus"}]);
        let err = build_middleware_from_config(Some(&entries)).unwrap_err();
        assert!(
            format!("{err}").contains("unknown type 'bogus'"),
            "got: {err}"
        );
    }

    #[test]
    fn missing_type_returns_error() {
        let entries = json!([{"max_retries": 3}]);
        let err = build_middleware_from_config(Some(&entries)).unwrap_err();
        assert!(
            format!("{err}").contains("missing required 'type'"),
            "got: {err}"
        );
    }

    #[test]
    fn non_object_entry_returns_error() {
        let entries = json!(["retry"]);
        let err = build_middleware_from_config(Some(&entries)).unwrap_err();
        assert!(format!("{err}").contains("must be an object"), "got: {err}");
    }

    #[test]
    fn error_history_rejects_unknown_keys() {
        let entries = json!([{
            "type": "error_history",
            "bogus_key": true
        }]);
        let err = build_middleware_from_config(Some(&entries)).unwrap_err();
        assert!(
            format!("{err}").contains("unexpected keys: bogus_key"),
            "got: {err}"
        );
    }
}
