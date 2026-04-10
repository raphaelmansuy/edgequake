# OODA-17: Act — Edge Case Tests for Workspace Utils

## Commit
`642e8926` on `feat/edgequake-v0.9.9`

## Changes

| File                        | Change                                                         |
| --------------------------- | -------------------------------------------------------------- |
| `workspace_utils.rs`        | **NEW** — Pure parsing functions + 14 edge case tests          |
| `workspace_row_types.rs`    | Removed pure functions, now imports from workspace_utils       |
| `workspace_service_impl.rs` | Updated imports to use workspace_utils                         |
| `lib.rs`                    | Added `mod workspace_utils` (always compiled, no feature gate) |

## Test Evidence
- 14 new tests: 8 normalize_entity_types, 3 parse_plan, 3 parse_role
- Core crate: 124 → 138 tests
- Workspace total: 1147 → 1161 tests, 0 failures
