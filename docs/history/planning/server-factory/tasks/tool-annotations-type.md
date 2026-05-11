# Task: ToolAnnotations Struct and Mapping

## Summary

Implement the `AnnotationMapper` in `src/adapters/annotations.rs` so it converts `ModuleAnnotations` (apcore crate) to MCP `ToolAnnotations`, and determines `requires_approval` and `streaming` flags for `_meta`.

## Approach (TDD-first)

### Tests to write first

1. **test_readonly_maps_to_read_only_hint** — `ModuleAnnotations { readonly: true, .. }` produces `ToolAnnotations { read_only_hint: Some(true), .. }`.
2. **test_destructive_maps_to_destructive_hint** — `destructive: true` maps to `destructive_hint: Some(true)`.
3. **test_idempotent_maps_to_idempotent_hint** — `idempotent: true` maps to `idempotent_hint: Some(true)`.
4. **test_open_world_maps_to_open_world_hint** — `open_world: false` maps to `open_world_hint: Some(false)`.
5. **test_default_annotations_mapping** — Default `ModuleAnnotations` maps to `readOnlyHint: false`, `destructiveHint: false`, `idempotentHint: false`, `openWorldHint: true`.
6. **test_has_requires_approval_true** — `requires_approval: true` returns `true`.
7. **test_has_requires_approval_false** — `requires_approval: false` returns `false`.
8. **test_streaming_flag** — `streaming: true` is detectable for `_meta` inclusion.

### Implementation steps

1. Change `AnnotationMapper` methods to accept `&ModuleAnnotations` (typed) instead of `&Value`.
2. Implement `to_mcp_annotations(&ModuleAnnotations) -> ToolAnnotations`.
3. Implement `has_requires_approval(&ModuleAnnotations) -> bool`.
4. Add `is_streaming(&ModuleAnnotations) -> bool` helper.
5. Keep the existing `to_description_suffix` for backward compat but update signature.

### Mapping table

| apcore `ModuleAnnotations` | MCP `ToolAnnotations` |
|---|---|
| `readonly` | `readOnlyHint` |
| `destructive` | `destructiveHint` |
| `idempotent` | `idempotentHint` |
| `open_world` | `openWorldHint` |
| `requires_approval` | `_meta.requiresApproval` |
| `streaming` | `_meta.streaming` |

## Files to modify

- Edit: `src/adapters/annotations.rs`

## Estimate

~2h

## Dependencies

None (uses `apcore::module::ModuleAnnotations` from dependency crate)
