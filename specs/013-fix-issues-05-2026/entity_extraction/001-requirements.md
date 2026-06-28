# Requirements — Entity types strict limit checkbox

## User story

As a workspace admin, I want a checkbox **“Limit extraction to listed types (classify others as OTHER)”** so that:

- **Checked (strict, default):** Future ingestions only use types from my list; anything else is normalized to `OTHER` (or `CONCEPT` / first type per policy).
- **Unchecked (permissive):** The list guides the LLM but does not force a catch-all `OTHER`; the model may emit domain-specific types (e.g. `MACHINE`, `PHONE`) and the server keeps them (normalized casing only).

Applies to **future ingestions** only; existing graph nodes are unchanged until **Rebuild Knowledge Graph**.

## Functional requirements

| ID | Requirement |
|----|-------------|
| FR-01 | Workspace stores `entity_types_strict` in metadata (`true` default when key absent). |
| FR-02 | UI checkbox on Entity Types card (edit mode), bound to `entity_types_strict`. |
| FR-03 | API exposes `entity_types_strict` on workspace GET/create/update. |
| FR-04 | Strict `true`: LLM prompt mandates allow-list; post-parse remaps unknown types per #217. |
| FR-05 | Strict `false`: LLM prompt lists types as guidance only; no “use OTHER when nothing fits”; post-parse does **not** remap to `OTHER`/`CONCEPT`/first. |
| FR-06 | Both modes: exact allow-list match still canonicalizes spelling (e.g. `person` → `PERSON`). |
| FR-07 | Both modes: token normalization (`UPPER_SNAKE_CASE`) always applied. |

## Edge cases

| Case | Strict ON | Strict OFF |
|------|-----------|------------|
| Unknown type `TELEPHONE_NUMBER`, list has `OTHER` | → `OTHER` | → `TELEPHONE_NUMBER` |
| Unknown type, list has no `OTHER` | → `CONCEPT` or first type | → `TELEPHONE_NUMBER` |
| `OTHER` in list, LLM emits `OTHER` voluntarily | `OTHER` | `OTHER` |
| Empty `entity_types` in metadata | Server default 9 types + strict flag | Same |
| `entity_types` cleared on update ([]) | Remove metadata key; strict unchanged unless sent | Same |
| `entity_types_strict` omitted on PUT | Do not change stored strict flag | Same |
| `entity_types_strict: false` on PUT | Persist `false` in metadata | — |
| `entity_types_strict: true` on PUT | Remove metadata key (default) | — |
| Workspace created without flag | Strict `true` | — |
| `OTHER` removed from chips while strict ON | Unknown types fall back to `CONCEPT`/first | N/A |
| Permissive + empty type list in UI | Server defaults as **guidance** only | No forced OTHER |
| Alias overlap (`PHONE` vs `TELEPHONE_NUMBER`) | Substring alias → `PHONE` if in list | Same alias rules |
| Mock / zero-token extraction | Unrelated; no entities regardless of strict | Same |
| Rebuild not run after toggle | Old nodes keep old types | Expected |

## Non-goals

- Retroactive relabel of existing graph nodes on toggle.
- Hiding `OTHER` from the chip list when strict is off (user may still add it as a normal type).
- Per-tenant or server-global default for strict (workspace-only).

## Acceptance criteria

1. Checkbox visible in workspace Entity Types edit mode; persists via API.
2. Unit tests: `enforce_entity_type` strict vs permissive matrix.
3. API e2e: PUT `entity_types_strict: false` round-trips.
4. Playwright: toggle visible, screenshot in `implementation/screenshots/`.
