# OODA Loop 67: PostgreSQL pg_trgm Schema Fix

## Date: 2026-01-06 10:50

## Observe
- Testing query validation logs revealed "Peugeot E-3008" was being dropped
- Direct psql query showed trigram matching WORKS: 86.7% similarity to "PEUGEOT 3008"
- But search_labels() was returning no results for similar queries

## Orient
**Root Cause**: Schema mismatch for pg_trgm extension

The `pg_trgm` extension was installed in the `ag_catalog` schema (because Apache AGE uses it), but the SQL queries were calling:
- `similarity()` without schema → function not found in default search path
- `%` operator without schema → operator lookup failed

The search_labels fallback to ILIKE prefix was also silently failing because the trigram step never returned results.

## Decide
Fix: Use explicit schema prefixes for all pg_trgm functions and operators:
```sql
-- Before (broken)
WHERE node_id % 'Peugeot E-3008'
ORDER BY similarity(node_id, 'Peugeot E-3008')

-- After (fixed)
WHERE node_id OPERATOR(ag_catalog.%) 'Peugeot E-3008'
ORDER BY ag_catalog.similarity(node_id, 'Peugeot E-3008')
```

## Act
1. Modified `search_labels()` in postgres/graph.rs:
   - Changed `%` to `OPERATOR(ag_catalog.%)`
   - Changed `similarity()` to `ag_catalog.similarity()`
2. Added debug tracing to diagnose keyword validation flow
3. Tested with extended query suite

## Results

### Direct Database Test
```
postgres=# SELECT label, similarity FROM nodes 
           WHERE node_id OPERATOR(ag_catalog.%) 'Peugeot E-3008';
     label     |    sim    
--------------+-----------
 PEUGEOT 3008 | 0.8666667   ← Now matches!
 PEUGEOT 308  |    0.6875
 PEUGEOT 2008 | 0.5555556
```

### Extended Test Suite
- **11/11 EXCELLENT** (unchanged, fix was already working)
- **100.0/100 average score**

### Keyword Validation Logs
```
dropped=["STLA Medium", "autoroute"] kept=["BYD Seal U", "batterie LFP", "E-3008"] ✓
dropped=["BYD Atto 3"] kept=["Peugeot"] ✓
```

## Key Insight
The `pg_trgm` extension location depends on how the database is initialized. In Apache AGE environments, it may be in `ag_catalog` rather than `public`. Always use explicit schema prefixes for extension functions/operators.

## Commit
```
fix(storage): Use explicit ag_catalog schema for pg_trgm operators
```

## Next Steps
- OODA 68: Analyze patterns in dropped keywords to identify data gaps vs schema issues
- Consider adding entity aliasing (e.g., "BYD Atto 3" → "BYD ATTO 3" or "Atto 3")
