# E2E Proof — P1 ConversationStorage Trait (DIP)

**Date:** 2026-06-02  
**Spec:** SPEC-017 / STORE-SOLID-D-001

## Problem

`ConversationServiceImpl` in `edgequake-core` depended directly on `PostgresConversationStorage`, violating DIP and blocking memory-backed conversation tests.

## Fix

| Component | Path |
|-----------|------|
| `ConversationStorage` trait | `src/conversation_storage.rs` |
| Shared row types | `src/conversation_types.rs` |
| Memory impl | `adapters/memory/conversation.rs` |
| Postgres impl | `adapters/postgres/conversation.rs` (trait delegates to inherent methods) |
| Service wiring | `edgequake-core/conversation_service_impl.rs` → `Arc<dyn ConversationStorage>` |

## Contract test

Shared fixture: `tests/support/conversation_contract.rs`

```bash
cargo test -p edgequake-storage --test conversation_backend_contract
# With DB:
cargo test -p edgequake-storage --test conversation_backend_contract --features postgres
```

## Verified

- Create folder → conversation → message → share → list (memory) — **PASS**
- `cargo build -p edgequake-core --features postgres` — **PASS**

## Not yet proven

- Postgres conversation contract in CI (requires `POSTGRES_PASSWORD`)
- `InMemoryConversationService` in API still used for memory mode (separate from storage trait)
