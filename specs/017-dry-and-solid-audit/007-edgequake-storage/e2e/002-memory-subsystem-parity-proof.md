# E2E Proof — P1 Memory PDF & Conversation Adapters

**Date:** 2026-06-02  
**Spec:** SPEC-017 / STORE-SOLID-L-002, STORE-SOLID-D-001

## Problem (pre-remediation)

- `PdfDocumentStorage` trait existed but only `PostgresPdfStorage` implemented it.
- Conversation persistence was postgres-only (`PostgresConversationStorage`), blocking upper-layer tests without DB.

## Fix

| Adapter | Path | Trait / API |
|---------|------|-------------|
| `MemoryPdfStorage` | `src/adapters/memory/pdf.rs` | `PdfDocumentStorage` |
| `MemoryConversationStorage` | `src/adapters/memory/conversation.rs` | Mirrors postgres CRUD surface |
| Shared row types | `src/conversation_types.rs` | `ConversationRow`, `MessageRow`, `FolderRow` |

## Tests

```bash
cargo test -p edgequake-storage --test memory_subsystem_parity
cargo test -p edgequake-storage --lib adapters::memory::pdf
cargo test -p edgequake-storage --lib adapters::memory::conversation
```

## Verified behaviors

1. **PDF deduplication** — second upload with same checksum returns `StorageError::Conflict`.
2. **FK semantics** — `link_pdf_to_document` fails until `ensure_document_record` creates the document row.
3. **Conversation lifecycle** — create conversation → message → share_id → message count == 1.

## Acceptance

Memory adapters satisfy the same contracts as postgres for test/dev substitution — **verified** (memory tests green; postgres integration unchanged).
