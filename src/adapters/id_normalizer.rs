//! ModuleIDNormalizer — converts between apcore dot-separated module IDs
//! and MCP/OpenAI dash-separated tool names.
//!
//! The mapping is bijective under the assumption that module IDs never contain
//! literal dashes (which the MODULE_ID_PATTERN enforces).

use crate::adapters::AdapterError;
use crate::constants::is_valid_module_id;

/// Normalizes and denormalizes module IDs for MCP/OpenAI compatibility.
///
/// apcore uses dot-separated IDs (e.g. `namespace.tool_name`), while
/// OpenAI function names require `^[a-zA-Z0-9_-]+$`. MCP tool names
/// use dot-notation directly, so this normalizer is only needed for
/// the OpenAI format.
pub struct ModuleIDNormalizer;

impl ModuleIDNormalizer {
    /// Normalize an apcore module ID to an OpenAI-compatible tool name (dot -> dash).
    ///
    /// Validates the input against [`MODULE_ID_PATTERN`] before converting.
    ///
    /// # Errors
    ///
    /// Returns [`AdapterError::InvalidModuleId`] if `module_id` does not match
    /// the canonical module ID pattern.
    ///
    /// # Examples
    ///
    /// ```
    /// # use apcore_mcp::adapters::ModuleIDNormalizer;
    /// assert_eq!(ModuleIDNormalizer::normalize("image.resize").unwrap(), "image-resize");
    /// assert_eq!(ModuleIDNormalizer::normalize("ping").unwrap(), "ping");
    /// assert!(ModuleIDNormalizer::normalize("INVALID").is_err());
    /// ```
    pub fn normalize(module_id: &str) -> Result<String, AdapterError> {
        if !is_valid_module_id(module_id) {
            return Err(AdapterError::invalid_module_id(module_id));
        }
        Ok(module_id.replace('.', "-"))
    }

    /// Denormalize an OpenAI tool name back to an apcore module ID (dash -> dot).
    ///
    /// No validation is performed — the tool name is assumed to originate from
    /// a prior call to [`normalize`](Self::normalize). For untrusted input use
    /// [`denormalize_checked`](Self::denormalize_checked).
    ///
    /// # Examples
    ///
    /// ```
    /// # use apcore_mcp::adapters::ModuleIDNormalizer;
    /// assert_eq!(ModuleIDNormalizer::denormalize("image-resize"), "image.resize");
    /// assert_eq!(ModuleIDNormalizer::denormalize("ping"), "ping");
    /// ```
    pub fn denormalize(tool_name: &str) -> String {
        tool_name.replace('-', ".")
    }

    /// Bijection-guarded denormalize for untrusted input.
    ///
    /// Runs the dash → dot replacement, then re-validates the result against
    /// [`MODULE_ID_PATTERN`](crate::constants::MODULE_ID_PATTERN). Returns
    /// `None` if the resulting candidate is not a valid module ID.
    ///
    /// Mirrors `try_denormalize` in apcore-mcp-python and `tryDenormalize`
    /// in apcore-mcp-typescript.
    ///
    /// # Examples
    ///
    /// ```
    /// # use apcore_mcp::adapters::ModuleIDNormalizer;
    /// assert_eq!(
    ///     ModuleIDNormalizer::denormalize_checked("image-resize"),
    ///     Some("image.resize".to_string())
    /// );
    /// assert_eq!(ModuleIDNormalizer::denormalize_checked("ping"), Some("ping".to_string()));
    /// // Invalid candidates after the dash→dot replacement are rejected:
    /// assert_eq!(ModuleIDNormalizer::denormalize_checked("Image-Resize"), None);
    /// assert_eq!(ModuleIDNormalizer::denormalize_checked(""), None);
    /// assert_eq!(ModuleIDNormalizer::denormalize_checked("-leading"), None);
    /// ```
    pub fn denormalize_checked(tool_name: &str) -> Option<String> {
        let candidate = tool_name.replace('-', ".");
        if !is_valid_module_id(&candidate) {
            return None;
        }
        Some(candidate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- normalize: valid inputs ----

    #[test]
    fn test_normalize_simple() {
        assert_eq!(
            ModuleIDNormalizer::normalize("image.resize").unwrap(),
            "image-resize"
        );
    }

    #[test]
    fn test_normalize_multi_segment() {
        assert_eq!(
            ModuleIDNormalizer::normalize("comfyui.image.resize.v2").unwrap(),
            "comfyui-image-resize-v2"
        );
    }

    #[test]
    fn test_normalize_single_segment() {
        // Single-segment IDs like "ping" are valid per MODULE_ID_PATTERN
        assert_eq!(ModuleIDNormalizer::normalize("ping").unwrap(), "ping");
    }

    #[test]
    fn test_normalize_with_underscores() {
        assert_eq!(
            ModuleIDNormalizer::normalize("my_module.v2_helper").unwrap(),
            "my_module-v2_helper"
        );
    }

    // ---- normalize: invalid inputs ----

    #[test]
    fn test_normalize_invalid_uppercase() {
        let result = ModuleIDNormalizer::normalize("Image.Resize");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Image.Resize"));
    }

    #[test]
    fn test_normalize_invalid_starts_with_number() {
        assert!(ModuleIDNormalizer::normalize("1module.test").is_err());
    }

    #[test]
    fn test_normalize_invalid_empty() {
        assert!(ModuleIDNormalizer::normalize("").is_err());
    }

    #[test]
    fn test_normalize_invalid_special_chars() {
        assert!(ModuleIDNormalizer::normalize("module!.test").is_err());
    }

    #[test]
    fn test_normalize_invalid_leading_dot() {
        assert!(ModuleIDNormalizer::normalize(".leading.dot").is_err());
    }

    #[test]
    fn test_normalize_invalid_trailing_dot() {
        assert!(ModuleIDNormalizer::normalize("trailing.dot.").is_err());
    }

    #[test]
    fn test_normalize_invalid_double_dot() {
        assert!(ModuleIDNormalizer::normalize("double..dot").is_err());
    }

    #[test]
    fn test_normalize_invalid_contains_dash() {
        assert!(ModuleIDNormalizer::normalize("my-module.resize").is_err());
    }

    // ---- denormalize ----

    #[test]
    fn test_denormalize_simple() {
        assert_eq!(
            ModuleIDNormalizer::denormalize("image-resize"),
            "image.resize"
        );
    }

    #[test]
    fn test_denormalize_multi_segment() {
        assert_eq!(
            ModuleIDNormalizer::denormalize("comfyui-image-resize-v2"),
            "comfyui.image.resize.v2"
        );
    }

    #[test]
    fn test_denormalize_no_dash() {
        assert_eq!(ModuleIDNormalizer::denormalize("ping"), "ping");
    }

    // ---- denormalize_checked (bijection-guarded) ----

    #[test]
    fn test_denormalize_checked_valid() {
        assert_eq!(
            ModuleIDNormalizer::denormalize_checked("image-resize"),
            Some("image.resize".to_string())
        );
        assert_eq!(
            ModuleIDNormalizer::denormalize_checked("ping"),
            Some("ping".to_string())
        );
    }

    #[test]
    fn test_denormalize_checked_rejects_uppercase() {
        // Dash→dot replacement still leaves invalid casing — must reject.
        assert_eq!(
            ModuleIDNormalizer::denormalize_checked("Image-Resize"),
            None
        );
    }

    #[test]
    fn test_denormalize_checked_rejects_empty() {
        assert_eq!(ModuleIDNormalizer::denormalize_checked(""), None);
    }

    #[test]
    fn test_denormalize_checked_rejects_leading_dash() {
        // After replacement: ".leading" — fails MODULE_ID_PATTERN.
        assert_eq!(ModuleIDNormalizer::denormalize_checked("-leading"), None);
    }

    #[test]
    fn test_denormalize_checked_rejects_trailing_dash() {
        // After replacement: "trailing." — fails MODULE_ID_PATTERN.
        assert_eq!(ModuleIDNormalizer::denormalize_checked("trailing-"), None);
    }

    #[test]
    fn test_denormalize_checked_rejects_double_dash() {
        // After replacement: "double..dash" — fails MODULE_ID_PATTERN.
        assert_eq!(
            ModuleIDNormalizer::denormalize_checked("double--dash"),
            None
        );
    }

    #[test]
    fn test_denormalize_checked_rejects_special_chars() {
        assert_eq!(ModuleIDNormalizer::denormalize_checked("bad!name"), None);
    }

    #[test]
    fn test_denormalize_checked_normalize_inverse() {
        // For every validly-normalized id, denormalize_checked recovers it.
        let ids = [
            "image.resize",
            "comfyui.image.resize.v2",
            "ping",
            "my_mod.v2",
        ];
        for id in ids {
            let normalized = ModuleIDNormalizer::normalize(id).unwrap();
            assert_eq!(
                ModuleIDNormalizer::denormalize_checked(&normalized),
                Some(id.to_string())
            );
        }
    }

    // ---- roundtrip ----

    #[test]
    fn test_roundtrip() {
        let ids = [
            "image.resize",
            "comfyui.image.resize.v2",
            "ping",
            "core.utils.string_ops",
            "a",
            "my_module.v2_helper",
        ];
        for id in ids {
            let normalized = ModuleIDNormalizer::normalize(id).unwrap();
            let denormalized = ModuleIDNormalizer::denormalize(&normalized);
            assert_eq!(denormalized, id, "roundtrip failed for '{id}'");
        }
    }

    #[test]
    fn test_roundtrip_property() {
        // Property-based: generate valid IDs from known-good segments
        let segments = ["core", "image", "resize", "v2", "my_mod", "a1b2"];
        for &a in &segments {
            // Single segment
            let id = a.to_string();
            let rt = ModuleIDNormalizer::denormalize(&ModuleIDNormalizer::normalize(&id).unwrap());
            assert_eq!(rt, id);

            // Two segments
            for &b in &segments {
                let id = format!("{a}.{b}");
                let rt =
                    ModuleIDNormalizer::denormalize(&ModuleIDNormalizer::normalize(&id).unwrap());
                assert_eq!(rt, id);
            }
        }
    }
}
