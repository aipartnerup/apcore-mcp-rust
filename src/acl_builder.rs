// Build an apcore `ACL` instance from a Config Bus `mcp.acl` section.
//
// Config Bus schema (YAML, shared across Python/TS/Rust bridges):
//
// ```yaml
// mcp:
//   acl:
//     default_effect: deny          # or "allow" — default "deny" (fail-secure)
//     rules:
//       - callers: ["role:admin"]
//         targets: ["sys.*"]
//         effect: allow
//         description: "Admins can reach system modules"
//       - callers: ["*"]
//         targets: ["sys.reload", "sys.toggle"]
//         effect: deny
//         conditions:
//           identity_types: ["human", "system"]
// ```
//
// Mirrors the Python `acl_builder.build_acl_from_config` contract. Invalid
// entries return an error so misconfiguration fails loudly at startup.

use apcore::{ACLRule, ACL};
use serde_json::Value;

use crate::apcore_mcp::APCoreMCPError;

const ALLOWED_EFFECTS: &[&str] = &["allow", "deny"];
const ALLOWED_RULE_KEYS: &[&str] = &["callers", "targets", "effect", "description", "conditions"];

/// Construct an `apcore::ACL` from a Config Bus `mcp.acl` value.
///
/// Returns `Ok(None)` when `acl_config` is `None`, `Value::Null`, or an empty
/// object (no rules / no default_effect). Returns `Err` on malformed entries.
pub fn build_acl_from_config(acl_config: Option<&Value>) -> Result<Option<ACL>, APCoreMCPError> {
    let Some(cfg) = acl_config else {
        return Ok(None);
    };
    if cfg.is_null() {
        return Ok(None);
    }

    let obj = cfg.as_object().ok_or_else(|| {
        APCoreMCPError::Config(format!(
            "mcp.acl must be a mapping with 'rules' and optional 'default_effect', \
             got {}",
            value_type_name(cfg)
        ))
    })?;

    // Validate `rules` type up-front before early-returning on empty config.
    let rules_val = obj.get("rules");
    if let Some(v) = rules_val {
        if !v.is_array() {
            return Err(APCoreMCPError::Config(format!(
                "mcp.acl.rules must be a list, got {}",
                value_type_name(v)
            )));
        }
    }

    let has_rules = rules_val.is_some_and(|v| v.as_array().is_some_and(|a| !a.is_empty()));
    let has_default = obj.contains_key("default_effect");
    if !has_rules && !has_default {
        return Ok(None);
    }

    let default_effect = obj
        .get("default_effect")
        .and_then(Value::as_str)
        .unwrap_or("deny")
        .to_string();
    if !ALLOWED_EFFECTS.contains(&default_effect.as_str()) {
        return Err(APCoreMCPError::Config(format!(
            "mcp.acl.default_effect must be 'allow' or 'deny', got {default_effect:?}"
        )));
    }

    let raw_rules = rules_val
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut rules: Vec<ACLRule> = Vec::with_capacity(raw_rules.len());
    for (idx, entry) in raw_rules.into_iter().enumerate() {
        let entry_obj = entry.as_object().ok_or_else(|| {
            APCoreMCPError::Config(format!(
                "mcp.acl.rules[{idx}] must be an object, got {}",
                value_type_name(&entry)
            ))
        })?;

        // Unknown keys → hard error.
        let extra: Vec<&str> = entry_obj
            .keys()
            .filter(|k: &&String| !ALLOWED_RULE_KEYS.contains(&k.as_str()))
            .map(String::as_str)
            .collect();
        if !extra.is_empty() {
            let mut sorted = extra.clone();
            sorted.sort_unstable();
            return Err(APCoreMCPError::Config(format!(
                "mcp.acl.rules[{idx}] got unexpected keys: {}",
                sorted.join(", ")
            )));
        }

        // Validate callers/targets/effect shape before handing to serde.
        let callers = entry_obj
            .get("callers")
            .and_then(Value::as_array)
            .filter(|a| !a.is_empty())
            .ok_or_else(|| {
                APCoreMCPError::Config(format!(
                    "mcp.acl.rules[{idx}] 'callers' must be a non-empty list"
                ))
            })?;
        let targets = entry_obj
            .get("targets")
            .and_then(Value::as_array)
            .filter(|a| !a.is_empty())
            .ok_or_else(|| {
                APCoreMCPError::Config(format!(
                    "mcp.acl.rules[{idx}] 'targets' must be a non-empty list"
                ))
            })?;
        let effect = entry_obj
            .get("effect")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                APCoreMCPError::Config(format!(
                    "mcp.acl.rules[{idx}] 'effect' must be 'allow' or 'deny'"
                ))
            })?;
        if !ALLOWED_EFFECTS.contains(&effect) {
            return Err(APCoreMCPError::Config(format!(
                "mcp.acl.rules[{idx}] 'effect' must be 'allow' or 'deny', got {effect:?}"
            )));
        }

        if let Some(conds) = entry_obj.get("conditions") {
            if !conds.is_null() && !conds.is_object() {
                return Err(APCoreMCPError::Config(format!(
                    "mcp.acl.rules[{idx}] 'conditions' must be an object or null"
                )));
            }
        }

        // Reconstruct rule via serde — snake_case field names already match.
        let callers_vec: Vec<String> = callers
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();
        let targets_vec: Vec<String> = targets
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();
        let description = entry_obj
            .get("description")
            .and_then(Value::as_str)
            .map(String::from);
        let conditions = entry_obj
            .get("conditions")
            .filter(|v| !v.is_null())
            .cloned();

        rules.push(ACLRule {
            callers: callers_vec,
            targets: targets_vec,
            effect: effect.to_string(),
            description,
            conditions,
        });
    }

    Ok(Some(ACL::new(rules, default_effect, None)))
}

fn value_type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn none_or_null_returns_none() {
        assert!(build_acl_from_config(None).unwrap().is_none());
        assert!(build_acl_from_config(Some(&Value::Null)).unwrap().is_none());
        assert!(build_acl_from_config(Some(&json!({}))).unwrap().is_none());
    }

    #[test]
    fn builds_acl_with_default_effect_deny() {
        let cfg = json!({
            "default_effect": "deny",
            "rules": [
                {"callers": ["role:admin"], "targets": ["sys.*"], "effect": "allow"}
            ]
        });
        let acl = build_acl_from_config(Some(&cfg)).unwrap().unwrap();
        assert_eq!(acl.rules().len(), 1);
    }

    #[test]
    fn default_effect_defaults_to_deny_when_omitted() {
        let cfg = json!({
            "rules": [
                {"callers": ["*"], "targets": ["public.*"], "effect": "allow"}
            ]
        });
        let acl = build_acl_from_config(Some(&cfg)).unwrap().unwrap();
        assert_eq!(acl.rules().len(), 1);
    }

    #[test]
    fn rule_with_description_and_conditions() {
        let cfg = json!({
            "rules": [
                {
                    "callers": ["role:admin"],
                    "targets": ["sys.*"],
                    "effect": "allow",
                    "description": "admin access",
                    "conditions": {"identity_types": ["human"]}
                }
            ]
        });
        let acl = build_acl_from_config(Some(&cfg)).unwrap().unwrap();
        let rule = &acl.rules()[0];
        assert_eq!(rule.description.as_deref(), Some("admin access"));
        assert!(rule.conditions.is_some());
    }

    #[test]
    fn invalid_default_effect_errors() {
        let cfg = json!({"default_effect": "maybe", "rules": []});
        let err = build_acl_from_config(Some(&cfg)).unwrap_err();
        assert!(
            format!("{err}").contains("default_effect must be"),
            "got: {err}"
        );
    }

    #[test]
    fn missing_callers_errors() {
        let cfg = json!({
            "rules": [{"targets": ["x.*"], "effect": "allow"}]
        });
        let err = build_acl_from_config(Some(&cfg)).unwrap_err();
        assert!(
            format!("{err}").contains("'callers' must be a non-empty list"),
            "got: {err}"
        );
    }

    #[test]
    fn missing_targets_errors() {
        let cfg = json!({
            "rules": [{"callers": ["*"], "effect": "allow"}]
        });
        let err = build_acl_from_config(Some(&cfg)).unwrap_err();
        assert!(
            format!("{err}").contains("'targets' must be a non-empty list"),
            "got: {err}"
        );
    }

    #[test]
    fn invalid_effect_errors() {
        let cfg = json!({
            "rules": [{"callers": ["*"], "targets": ["*"], "effect": "maybe"}]
        });
        let err = build_acl_from_config(Some(&cfg)).unwrap_err();
        assert!(
            format!("{err}").contains("'effect' must be 'allow' or 'deny'"),
            "got: {err}"
        );
    }

    #[test]
    fn unknown_rule_keys_error() {
        let cfg = json!({
            "rules": [
                {"callers": ["*"], "targets": ["*"], "effect": "allow", "bogus": true}
            ]
        });
        let err = build_acl_from_config(Some(&cfg)).unwrap_err();
        assert!(format!("{err}").contains("unexpected keys"), "got: {err}");
    }

    #[test]
    fn non_object_top_level_errors() {
        let cfg = json!("deny");
        let err = build_acl_from_config(Some(&cfg)).unwrap_err();
        assert!(format!("{err}").contains("must be a mapping"), "got: {err}");
    }

    #[test]
    fn rules_non_array_errors() {
        let cfg = json!({"rules": "oops"});
        let err = build_acl_from_config(Some(&cfg)).unwrap_err();
        assert!(
            format!("{err}").contains("rules must be a list"),
            "got: {err}"
        );
    }
}
