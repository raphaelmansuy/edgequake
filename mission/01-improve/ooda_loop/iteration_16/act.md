# OODA-16: Act — SRP Row Type Extraction

## Commit
`db875fc7` on `feat/edgequake-v0.9.9`

## Changes

| File                                           | Change                                                                                                      |
| ---------------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| `edgequake-core/src/workspace_row_types.rs`    | **NEW** (312 lines) — TenantRow, WorkspaceRow, MembershipRow + parse_plan/parse_role/normalize_entity_types |
| `edgequake-core/src/workspace_service_impl.rs` | Removed 288 lines of row types, now imports from workspace_row_types                                        |
| `edgequake-core/src/lib.rs`                    | Added `mod workspace_row_types` declaration                                                                 |

## Metrics
- workspace_service_impl.rs: 1492 → 1204 lines (-19%)
- 1147 tests pass, 0 clippy warnings
