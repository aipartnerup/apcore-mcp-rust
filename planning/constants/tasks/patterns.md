# Task: patterns

## Goal

Define the `MODULE_ID_PATTERN` constant and a `module_id_regex()` helper that returns a compiled, thread-safe `Regex` instance.

## Files Involved

- `src/constants.rs` -- add/update `MODULE_ID_PATTERN` and `module_id_regex()`

## Steps

1. **Write tests first** (TDD) -- add to the `tests` module in `src/constants.rs`:
   ```rust
   #[test]
   fn module_id_pattern_valid() {
       let re = module_id_regex();
       // Dot-separated
       assert!(re.is_match("image.resize"));
       assert!(re.is_match("core.utils.string_ops"));
       // Single segment (no dot) -- valid per Python reference
       assert!(re.is_match("core"));
       assert!(re.is_match("a"));
       // Underscores and digits allowed (not leading)
       assert!(re.is_match("my_module.v2_helper"));
   }

   #[test]
   fn module_id_pattern_invalid() {
       let re = module_id_regex();
       assert!(!re.is_match(""));                   // empty
       assert!(!re.is_match("Image.Resize"));       // uppercase
       assert!(!re.is_match("2fast.module"));        // leading digit
       assert!(!re.is_match("my-module.resize"));    // hyphen
       assert!(!re.is_match(".leading.dot"));        // leading dot
       assert!(!re.is_match("trailing.dot."));       // trailing dot
       assert!(!re.is_match("double..dot"));         // double dot
       assert!(!re.is_match("has space.mod"));       // space
   }

   #[test]
   fn module_id_pattern_string() {
       // Verify the raw pattern is accessible
       assert!(MODULE_ID_PATTERN.starts_with('^'));
       assert!(MODULE_ID_PATTERN.ends_with('$'));
   }
   ```

2. **Run tests -- expect compile failure** (function not yet defined):
   ```bash
   cargo test -- constants
   ```

3. **Implement** in `src/constants.rs`:
   ```rust
   /// Regex pattern for valid module IDs.
   ///
   /// Module IDs are lowercase dot-separated segments: `core`, `image.resize`,
   /// `core.utils.string_ops`. Each segment starts with a letter and may contain
   /// lowercase letters, digits, and underscores.
   ///
   /// Matches the Python reference: `^[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*)*$`
   pub const MODULE_ID_PATTERN: &str = r"^[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*)*$";

   /// Returns a compiled [`Regex`] for [`MODULE_ID_PATTERN`].
   ///
   /// The regex is compiled once and cached for the lifetime of the process.
   pub fn module_id_regex() -> &'static Regex {
       static RE: LazyLock<Regex> =
           LazyLock::new(|| Regex::new(MODULE_ID_PATTERN).expect("MODULE_ID_PATTERN is valid regex"));
       &RE
   }
   ```

   **Note:** The existing stub uses `+` quantifier (`(\.[a-z][a-z0-9_]*)+$`) which requires at least one dot segment. Change to `*` to match the Python reference, which allows single-segment IDs like `"core"`.

4. **Run tests -- expect all to pass**:
   ```bash
   cargo test -- constants
   ```

## Acceptance Criteria

- [ ] `MODULE_ID_PATTERN` uses `*` quantifier (matches Python reference)
- [ ] `module_id_regex()` returns a compiled `Regex`
- [ ] Valid IDs like `"image.resize"` and `"core"` match
- [ ] Invalid IDs (uppercase, leading digit, special chars, empty) do not match
- [ ] The `Regex` is compiled only once (LazyLock)
- [ ] `MODULE_ID_PATTERN` raw string is publicly accessible

## Dependencies

- **Depends on:** setup
- **Required by:** integration

## Estimated Time

10 minutes
