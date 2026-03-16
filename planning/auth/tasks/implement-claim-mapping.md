# Task: implement-claim-mapping

## Goal

Complete the `ClaimMapping` struct to match the Python implementation, adding `id_claim`, `type_claim`, `roles_claim`, and `attrs_claims` fields with proper defaults.

## Files Involved

- `src/auth/jwt.rs` — rewrite `ClaimMapping`

## Steps (TDD-first)

1. **Write tests** for `ClaimMapping::default()` asserting:
   - `id_claim == "sub"`
   - `type_claim == "type"`
   - `roles_claim == "roles"`
   - `attrs_claims == None`
2. **Rewrite the `ClaimMapping` struct** with fields:
   - `id_claim: String` (default: `"sub"`)
   - `type_claim: String` (default: `"type"`)
   - `roles_claim: String` (default: `"roles"`)
   - `attrs_claims: Option<Vec<String>>` (default: `None`)
3. **Derive** `Debug, Clone, Serialize, Deserialize` and implement `Default`.
4. **Run tests** and verify they pass.

## Acceptance Criteria

- [ ] `ClaimMapping` has all four fields matching Python's dataclass
- [ ] `Default` impl provides correct values
- [ ] Struct derives `Debug, Clone, Serialize, Deserialize`
- [ ] Unit tests pass

## Dependencies

- align-identity-type

## Estimated Time

30 minutes
