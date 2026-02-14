# OODA-31: Decide

## Plan

1. Fix `provenance.rs` lineage URL
2. Add `get_raw()` to `client.rs`
3. Create `lineage.rs` with 4 methods
4. Create `settings.rs` with 2 methods
5. Register modules in `mod.rs` and add accessors to `client.rs`
6. Fix existing test path regex
7. Add 10 new wiremock tests (7 lineage + 3 settings)
8. Run `cargo test` — all must pass
