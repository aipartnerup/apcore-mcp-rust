# Task: deep-merge

## Goal

Implement a `deep_merge` function that recursively merges two `serde_json::Value` objects, with recursion depth capped at 32. This is a direct port of the Python `_deep_merge` function used to accumulate streaming chunks.

## Files Involved

- `src/server/router.rs` — Add `deep_merge` function and `DEEP_MERGE_MAX_DEPTH` constant

## Steps (TDD-first)

1. **Write unit tests first**:
   - `test_deep_merge_flat_objects` — Two flat objects merge keys correctly, overlay wins on conflict
   - `test_deep_merge_nested_objects` — Nested dicts are recursively merged
   - `test_deep_merge_overlay_overwrites_non_object` — When base has a string and overlay has a dict for the same key, overlay wins
   - `test_deep_merge_base_dict_overlay_scalar` — When base has a dict and overlay has a scalar, overlay wins
   - `test_deep_merge_empty_base` — Empty base returns overlay
   - `test_deep_merge_empty_overlay` — Empty overlay returns base unchanged
   - `test_deep_merge_both_empty` — Two empty objects produce empty object
   - `test_deep_merge_depth_cap` — At depth 32, merge is flat (no recursion into nested dicts)
   - `test_deep_merge_depth_31_still_recurses` — At depth 31, recursion still happens
   - `test_deep_merge_non_object_inputs` — Non-object values (arrays, strings) return overlay
   - `test_deep_merge_three_levels_deep` — Three-level nested merge produces correct result
   - `test_deep_merge_array_not_merged` — Arrays are overwritten, not concatenated (matches Python behavior)

2. **Define the constant**:
   ```rust
   const DEEP_MERGE_MAX_DEPTH: usize = 32;
   ```

3. **Implement `deep_merge`**:
   ```rust
   fn deep_merge(base: &Value, overlay: &Value, depth: usize) -> Value {
       match (base, overlay) {
           (Value::Object(base_map), Value::Object(overlay_map)) => {
               if depth >= DEEP_MERGE_MAX_DEPTH {
                   // Flat merge: overlay keys win
                   let mut merged = base_map.clone();
                   for (k, v) in overlay_map {
                       merged.insert(k.clone(), v.clone());
                   }
                   return Value::Object(merged);
               }
               let mut merged = base_map.clone();
               for (k, v) in overlay_map {
                   let existing = merged.get(k);
                   let new_val = match existing {
                       Some(existing_val) => deep_merge(existing_val, v, depth + 1),
                       None => v.clone(),
                   };
                   merged.insert(k.clone(), new_val);
               }
               Value::Object(merged)
           }
           _ => overlay.clone(),
       }
   }
   ```

4. **Run tests** — ensure all pass. Run `cargo clippy`.

## Acceptance Criteria

- [ ] `DEEP_MERGE_MAX_DEPTH` constant is 32
- [ ] Recursively merges `Value::Object` entries
- [ ] Non-object values are overwritten by overlay
- [ ] Recursion stops at depth 32, falling back to flat merge
- [ ] Arrays are overwritten, not concatenated
- [ ] Empty inputs handled correctly
- [ ] Function is `pub(crate)` (not part of public API)
- [ ] All tests pass, clippy clean

## Dependencies

- none

## Estimated Time

1 hour
