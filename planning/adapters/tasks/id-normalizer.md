# Task: id-normalizer

## Goal

Implement `ModuleIDNormalizer` with regex-validated bijective dot-to-dash mapping for converting between apcore module IDs and MCP/OpenAI-compatible tool names.

## Files Involved

- `src/adapters/id_normalizer.rs` — Enhance existing implementation with regex validation
- `src/constants.rs` — Verify `MODULE_ID_PATTERN` regex
- `tests/adapters/id_normalizer_test.rs` or inline `#[cfg(test)]` module

## Steps (TDD-first)

1. **Write unit tests first**:
   - `test_normalize_simple` — `"image.resize"` -> `"image-resize"`
   - `test_normalize_multi_segment` — `"comfyui.image.resize.v2"` -> `"comfyui-image-resize-v2"`
   - `test_normalize_single_segment` — `"ping"` -> decide behavior (current regex requires dots; see risk note)
   - `test_normalize_invalid_uppercase` — `"Image.Resize"` -> error
   - `test_normalize_invalid_starts_with_number` — `"1module.test"` -> error
   - `test_normalize_invalid_empty` — `""` -> error
   - `test_normalize_invalid_special_chars` — `"module!.test"` -> error
   - `test_denormalize_simple` — `"image-resize"` -> `"image.resize"`
   - `test_denormalize_multi_segment` — `"comfyui-image-resize-v2"` -> `"comfyui.image.resize.v2"`
   - `test_denormalize_no_dash` — `"ping"` -> `"ping"` (no-op)
   - `test_roundtrip` — `denormalize(normalize(id)) == id` for valid IDs
   - `test_roundtrip_property` — Property-based test with valid IDs

2. **Compile the regex once** using `LazyLock`:
   ```rust
   use std::sync::LazyLock;
   use regex::Regex;
   static MODULE_ID_REGEX: LazyLock<Regex> = LazyLock::new(|| {
       Regex::new(crate::constants::MODULE_ID_PATTERN).expect("invalid MODULE_ID_PATTERN regex")
   });
   ```

3. **Implement `normalize`**:
   - Validate `module_id` against `MODULE_ID_REGEX`
   - If no match, return `Err(AdapterError::InvalidModuleId { ... })`
   - Replace `'.'` with `'-'`

4. **Implement `denormalize`**:
   - Replace `'-'` with `'.'`
   - No validation needed (denormalize is lenient — the MCP tool name comes from our own normalize)

5. **Address pattern discrepancy**:
   - Current `MODULE_ID_PATTERN` in `constants.rs` is `^[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*)+$` — requires at least one dot
   - Python version allows single-segment IDs (e.g., `"ping"`)
   - Decision: update pattern to `^[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*)*$` (change `+` to `*`) to match Python behavior
   - Document the change in a code comment

6. **Update function signatures**:
   - Change `normalize` to return `Result<String, AdapterError>`
   - Keep `denormalize` returning `String` (infallible)

7. **Run tests** — ensure all pass. Run `cargo clippy`.

## Acceptance Criteria

- [ ] `normalize` replaces dots with dashes for valid module IDs
- [ ] `normalize` returns `AdapterError::InvalidModuleId` for invalid IDs
- [ ] `denormalize` replaces dashes with dots
- [ ] Roundtrip `denormalize(normalize(id)) == id` holds for all valid IDs
- [ ] Regex is compiled once (static `LazyLock`)
- [ ] Single-segment IDs (e.g., `"ping"`) are handled correctly
- [ ] All tests pass, clippy clean

## Dependencies

- adapter-setup

## Estimated Time

1 hour
