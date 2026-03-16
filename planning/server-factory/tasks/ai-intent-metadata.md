# Task: AI Intent Key Extraction and Description Enrichment

## Summary

Implement the logic that extracts AI intent metadata keys (`x-when-to-use`, `x-when-not-to-use`, `x-common-mistakes`, `x-workflow-hints`) from module metadata and appends them to the tool description string. This enriches tool descriptions so LLM agents can make better decisions about tool selection.

## Approach (TDD-first)

### Tests to write first

1. **test_no_metadata_no_suffix** — When metadata is `None` or empty, description is unchanged.
2. **test_when_to_use_appended** — Metadata with `x-when-to-use: "Use for reading files"` appends `\n\nWhen To Use: Use for reading files`.
3. **test_multiple_intents_appended** — Multiple intent keys are joined with newlines.
4. **test_intent_key_label_formatting** — `x-when-not-to-use` becomes `When Not To Use`, `x-common-mistakes` becomes `Common Mistakes`, `x-workflow-hints` becomes `Workflow Hints`.
5. **test_empty_intent_value_skipped** — Intent keys with empty string values are not appended.
6. **test_non_intent_metadata_ignored** — Metadata keys that are not in the AI intent set are ignored.

### Implementation steps

1. Define `AI_INTENT_KEYS: &[&str]` constant in `src/server/factory.rs`:
   ```rust
   const AI_INTENT_KEYS: &[&str] = &[
       "x-when-to-use",
       "x-when-not-to-use",
       "x-common-mistakes",
       "x-workflow-hints",
   ];
   ```
2. Implement `fn enrich_description(base: &str, metadata: Option<&HashMap<String, String>>) -> String`.
3. For each intent key present in metadata with a non-empty value, format as `"{Label}: {value}"` where label is derived by stripping `x-`, replacing `-` with spaces, and title-casing.
4. Join all intent parts with `\n` and append to base description after `\n\n`.
5. Integrate into `build_tool()` — call `enrich_description` before constructing the `Tool`.

### Note on metadata field

The `ModuleDescriptor` in apcore-rust currently lacks a `metadata: HashMap<String, String>` field. This task must either:
- Add `metadata: Option<HashMap<String, serde_json::Value>>` to `ModuleDescriptor` (if modifying apcore-rust is in scope), or
- Accept metadata as a separate parameter to `build_tool`, or
- Use `#[serde(flatten)] pub extra: HashMap<String, Value>` pattern.

Document the chosen approach in the PR.

## Files to modify

- Edit: `src/server/factory.rs`
- Possibly: `apcore-rust/src/registry/registry.rs` (add metadata field)

## Estimate

~2h

## Dependencies

- build-tool
