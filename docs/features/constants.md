# Feature: Constants

## Module Purpose
Shared constants used across the MCP bridge: error codes, registry events, and module ID validation pattern.

## Public API Surface

### Constants
- `REGISTRY_EVENTS: HashMap<&str, &str>` — {"REGISTER": "register", "UNREGISTER": "unregister"}
- `ERROR_CODES: [&str]` — all 18 error code strings
- `MODULE_ID_PATTERN: &str` — regex `^[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*)*$`

## Acceptance Criteria
- [ ] ERROR_CODES contains all 18 codes from Python implementation
- [ ] MODULE_ID_PATTERN matches valid module IDs (e.g., "image.resize")
- [ ] MODULE_ID_PATTERN rejects invalid IDs (uppercase, special chars, leading digits)
- [ ] REGISTRY_EVENTS maps event types correctly
