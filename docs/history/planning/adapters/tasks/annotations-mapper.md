# Task: annotations-mapper

## Goal

Implement `AnnotationMapper` that converts apcore `ModuleAnnotations` to MCP tool annotations, generates description suffixes with safety warnings, and checks for approval requirements.

## Files Involved

- `src/adapters/annotations.rs` — Full implementation replacing the current stub
- `tests/adapters/annotations_test.rs` or inline `#[cfg(test)]` module

## Steps (TDD-first)

1. **Write unit tests first**:
   - `test_to_mcp_annotations_none` — `None` input returns defaults (`readOnlyHint: false`, `destructiveHint: false`, `idempotentHint: false`, `openWorldHint: true`, `title: null`)
   - `test_to_mcp_annotations_readonly` — Annotations with `readonly: true` maps to `readOnlyHint: true`
   - `test_to_mcp_annotations_destructive` — `destructive: true` maps to `destructiveHint: true`
   - `test_to_mcp_annotations_all_set` — All fields set, verify complete mapping
   - `test_to_description_suffix_none` — `None` returns empty string
   - `test_to_description_suffix_destructive` — Contains "DESTRUCTIVE" warning
   - `test_to_description_suffix_requires_approval` — Contains "REQUIRES APPROVAL" warning
   - `test_to_description_suffix_non_default_values` — Contains `[Annotations: ...]` block
   - `test_to_description_suffix_no_changes` — Default annotations return empty string
   - `test_has_requires_approval_none` — Returns `false`
   - `test_has_requires_approval_true` — Returns `true`
   - `test_has_requires_approval_false` — Returns `false`

2. **Replace stub with implementation**:
   - Change `AnnotationMapper` to take `Option<&ModuleAnnotations>` instead of `&Value`
   - Import `apcore::module::ModuleAnnotations`
   - Implement `to_mcp_annotations`:
     - Return `serde_json::Value` object with MCP hint keys
     - Map: `readonly` -> `readOnlyHint`, `destructive` -> `destructiveHint`, `idempotent` -> `idempotentHint`, `open_world` -> `openWorldHint`, `title` -> `null`
   - Implement `to_description_suffix`:
     - Build warnings list for `destructive` and `requires_approval`
     - Build parts list for any annotation differing from `DEFAULT_ANNOTATIONS`
     - Format as `"\n\n" + warnings + "\n\n[Annotations: parts]"`
   - Implement `has_requires_approval`:
     - Return `annotations.requires_approval` or `false` if `None`

3. **Update `DEFAULT_ANNOTATIONS`** constant to align with `ModuleAnnotations::default()`:
   ```rust
   const DEFAULT_ANNOTATIONS: ModuleAnnotations = ModuleAnnotations { ... };
   ```
   Or compare field-by-field against known defaults.

4. **Run tests** — ensure all pass. Run `cargo clippy`.

## Acceptance Criteria

- [ ] `to_mcp_annotations` maps all five MCP hint fields correctly
- [ ] `to_mcp_annotations(None)` returns sensible defaults
- [ ] `to_description_suffix` generates destructive warning text
- [ ] `to_description_suffix` generates approval warning text
- [ ] `to_description_suffix` includes non-default annotation values in `[Annotations: ...]` block
- [ ] `to_description_suffix(None)` returns empty string
- [ ] `has_requires_approval` returns correct boolean
- [ ] All tests pass, clippy clean

## Dependencies

- adapter-setup

## Estimated Time

2 hours
