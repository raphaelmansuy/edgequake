# Issue #216 — Root Cause Analysis

**GitHub:** [#216](https://github.com/raphaelmansuy/edgequake/issues/216)

## Symptom (fact)

Entity types set only at workspace creation; no UI/API to update on existing workspace.

## 5 WHY

| # | Why | Evidence |
|---|-----|----------|
| 1 | Why can't users edit? | Workspace settings page: entity types card is read-only badges |
| 2 | Why read-only? | No `EntityTypeSelector` in edit mode; no `entity_types` in save payload |
| 3 | Does DB support it? | Yes — `metadata.entity_types` JSON on create ([workspace_service_impl.rs:442](edgequake/crates/edgequake-core/src/workspace_service_impl.rs)) |
| 4 | Does update API support it? | **No** — `UpdateWorkspaceRequest` lacked `entity_types` until this fix |
| 5 | Why workaround (SQL) works? | Direct metadata update; processor reads `workspace_entity_types()` at ingest |

## Fix summary

Add `entity_types` to core/API update path + editable `EntityTypeSelector` on workspace page (future ingestions only).
