# OODA-15: Decide

1. Fix `pdf_storage_impl.rs` and `pdf_list_query.rs`: `parse().unwrap()` → `parse().unwrap_or_default()` for DB status
2. Fix `entity.rs` and `relationship.rs`: `from_f64().unwrap()` → clamp NaN/Inf then convert
3. Fix `reranking.rs`: verify caller guard or add proper error
4. Fix `community.rs`: HashMap unwrap → defensive default
5. Add `// WHY expect:` to safe middleware and regex sites
6. Fix `in_memory.rs` RwLock poison recovery (20+ sites)
7. Run tests across affected crates
