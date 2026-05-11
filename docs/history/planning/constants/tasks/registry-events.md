# Task: registry-events

## Goal

Implement the `RegistryEvent` enum with `Register` and `Unregister` variants, replacing the current `LazyLock<HashMap>` approach with a type-safe enum.

## Files Involved

- `src/constants.rs` -- add `RegistryEvent` enum, remove old `REGISTRY_EVENTS` static

## Steps

1. **Write tests first** (TDD) -- add to the `tests` module in `src/constants.rs`:
   ```rust
   #[test]
   fn registry_event_display() {
       assert_eq!(RegistryEvent::Register.to_string(), "register");
       assert_eq!(RegistryEvent::Unregister.to_string(), "unregister");
   }

   #[test]
   fn registry_event_from_str() {
       assert_eq!("register".parse::<RegistryEvent>().unwrap(), RegistryEvent::Register);
       assert_eq!("unregister".parse::<RegistryEvent>().unwrap(), RegistryEvent::Unregister);
   }

   #[test]
   fn registry_event_serde_round_trip() {
       for event in RegistryEvent::iter() {
           let json = serde_json::to_string(&event).unwrap();
           let parsed: RegistryEvent = serde_json::from_str(&json).unwrap();
           assert_eq!(parsed, event);
       }
   }

   #[test]
   fn registry_event_key() {
       assert_eq!(RegistryEvent::Register.key(), "REGISTER");
       assert_eq!(RegistryEvent::Unregister.key(), "UNREGISTER");
   }
   ```

2. **Run tests -- expect compile failure**:
   ```bash
   cargo test -- constants
   ```

3. **Implement the enum** in `src/constants.rs`:
   ```rust
   /// Registry lifecycle events.
   ///
   /// The wire value is lowercase (`"register"`, `"unregister"`).
   /// Use [`RegistryEvent::key()`] for the uppercase protocol key.
   #[derive(
       Debug, Clone, Copy, PartialEq, Eq, Hash,
       Display, EnumString, EnumIter,
       Serialize, Deserialize,
   )]
   #[strum(serialize_all = "lowercase")]
   #[serde(rename_all = "lowercase")]
   pub enum RegistryEvent {
       Register,
       Unregister,
   }

   impl RegistryEvent {
       /// Returns the uppercase protocol key (e.g., `"REGISTER"`).
       pub const fn key(&self) -> &'static str {
           match self {
               Self::Register => "REGISTER",
               Self::Unregister => "UNREGISTER",
           }
       }
   }
   ```

4. **Remove the old `REGISTRY_EVENTS` static HashMap** and update any references in other files.

5. **Run tests -- expect all to pass**:
   ```bash
   cargo test -- constants
   ```

6. **Search for usages of the old `REGISTRY_EVENTS` HashMap** and update callers:
   ```bash
   grep -rn "REGISTRY_EVENTS" src/
   ```

## Acceptance Criteria

- [ ] `RegistryEvent::Register` displays as `"register"`
- [ ] `RegistryEvent::Unregister` displays as `"unregister"`
- [ ] `key()` returns `"REGISTER"` and `"UNREGISTER"` respectively
- [ ] `FromStr` parses lowercase strings correctly
- [ ] serde round-trips correctly
- [ ] Old `REGISTRY_EVENTS` HashMap is removed
- [ ] No compile errors in dependent modules

## Dependencies

- **Depends on:** setup
- **Required by:** integration

## Estimated Time

10 minutes
