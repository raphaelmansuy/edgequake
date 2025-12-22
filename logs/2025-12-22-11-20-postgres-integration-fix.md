# Task Log: PostgreSQL Integration Tests Fix

## Actions

- Fixed PostgreSQL adapter API to use `Storage::new(config)` pattern instead of `Storage::new(pool, namespace)`
- Added explicit `public` schema prefix to all table names to avoid AGE search_path issues
- Fixed SQL syntax errors in `upsert_node` and `upsert_edge` (removed invalid table references in UPDATE SET clauses)
- Added `prefix` field to storage structs for correct index naming
- Added `uuid` dev-dependency for test namespace generation
- Updated all 7 integration tests to use new API

## Decisions

- Used `public.eq_{prefix}_*` naming convention to avoid conflicts with Apache AGE's ag_catalog schema
- Replaced `{table_name}.properties || EXCLUDED.properties` with `EXCLUDED.properties` (full replacement instead of merge)
- Store table prefix in struct field rather than re-deriving from config

## Test Results

- PostgreSQL integration tests: 7/7 passing
- Storage unit tests: 25/25 passing
- E2E pipeline tests: 20/20 passing

## Next Steps

- Consider adding property merging logic if partial updates are needed
- Add more edge case tests for vector storage with different dimensions
- Consider adding connection pooling tests

## Lessons/Insights

- PostgreSQL search_path affects table creation when AGE extension sets it to ag_catalog
- Schema-qualified table names need careful handling in SQL - don't use in column references
