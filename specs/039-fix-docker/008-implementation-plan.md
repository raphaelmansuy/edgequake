# SPEC-039 — Implementation Plan

**Target release:** v0.13.1 (patch)  
**Scope:** Storage bootstrap only — no API contract changes

---

## Phase 1 — Root fix (DONE)

- [x] Add `ensure_graph_labels()` + `ensure_age_label()` in `graph_lifecycle.rs`
- [x] Invoke from `create_graph()` after catalog graph exists
- [x] Idempotent `pg_class` check + race-safe "already exists" handling

---

## Phase 2 — Documentation

- [x] `specs/039-fix-docker/` cross-ref pack
- [x] README Docker quickstart: Mistral + Ollama `gemma4:e4b` tested commands
- [x] CHANGELOG v0.13.1 entry

---

## Phase 3 — Verification

- [x] Manual SQL proof on v0.13.0 stack (labels → ingest + query OK)
- [x] `cargo test -p edgequake-storage --lib`
- [x] Docker fresh-install E2E script (Mistral + Ollama `gemma4:e4b`)
- [x] Local image build + compose proof before tag

---

## Edge cases mitigated

| Edge case | Mitigation |
| --------- | ---------- |
| Graph already has labels (upgrade) | `EXISTS` check → no-op |
| Concurrent workers on first boot | Catch "already exists" → Ok |
| AGE not installed | `setup_age_session` fails earlier; fallback tables used |
| Wrong label names | Hard-coded SSOT: `Node`, `EDGE` (EdgeQuake convention) |
| `create_vlabel` on graph missing | Only called after `create_graph` succeeds |
| Index bootstrap before labels | `ensure_indexes` already skips missing tables; labels created first now |

---

## Rollback

Remove `ensure_graph_labels()` call — reverts to lazy label creation (breaks fresh Docker again). No migration required.

---

## Out of scope

- Enabling `EDGEQUAKE_NATIVE_GRAPH_WRITES=1` by default in Docker (separate SPEC-034 rollout)
- Quickstart wizard Mistral menu (already supports env-based compose)
