# OODA-27 Observe

Files missing WHY comments in storage, core, pipeline, and tasks crates:
- `storage/error.rs` — 12 variants, 11 tests, no WHY
- `core/error.rs` — Error + QueryError enums, 10 tests, no WHY
- `pipeline/cache.rs` — LLM cache trait + MemoryCache impl, needs WHY on cache key strategy
- `pipeline/validation.rs` — 20 edge cases documented in table, needs WHY on strict/lenient split
- `tasks/queue.rs` — Channel-based task queue, needs WHY on Mutex<Receiver> pattern
