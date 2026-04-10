# OODA-16: Observe — SRP File Splitting

## Territory Map

### Largest files (production code, excluding tests):

| File                                          | Lines | Responsibilities                                                                      |
| --------------------------------------------- | ----- | ------------------------------------------------------------------------------------- |
| `edgequake-storage/.../graph/mod.rs`          | 1512  | GraphStorage trait impl (30+ methods, but all graph ops — single concern)             |
| `edgequake-core/workspace_service_impl.rs`    | 1492  | **5 concerns**: tenant CRUD, workspace CRUD, membership, metrics, quota + 3 row types |
| `edgequake-api/pipeline_progress_callback.rs` | 1326  | ~626 production + ~700 tests — border case                                            |
| `edgequake-storage/.../memory/vector.rs`      | 1282  | In-memory vector storage (single concern)                                             |
| `edgequake-api/.../text_insert.rs`            | 1086  | Text document insertion pipeline                                                      |

### Target: `workspace_service_impl.rs` (1492 lines)

**SRP violations identified:**
- TenantRow/WorkspaceRow/MembershipRow (DB deserialization types) mixed with service logic
- `normalize_entity_types()` standalone utility buried at line 1229
- Row type conversions (into_tenant/into_workspace/into_membership) are ~263 lines of data mapping
