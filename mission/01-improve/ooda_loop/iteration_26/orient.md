# OODA-26 Orient: WHY Comments Are Architecture Documentation

## Analysis

WHY comments serve two purposes:
1. **Prevent Chesterton's fence violations** — future developers won't remove "unnecessary" code
2. **Encode design rationale** — first principles behind the structure

### Key Design Decisions to Document

| File | Decision | WHY |
|------|----------|-----|
| context.rs | Separate collections for chunks/entities/rels | Different token budgets and truncation strategies per type |
| context.rs | Incremental token_count on add_chunk | Avoid O(n) recount; enables early budget-exceeded detection |
| context.rs | `to_context_string` sections order: entities → rels → chunks | Graph context first (BR0102), chunks as supporting evidence |
| context_filter.rs | Strict chunk filter, lenient entity/rel filter | Entities often span multiple documents; removing them loses cross-document knowledge |
| error.rs | Separate InvalidQuery vs ConfigError | InvalidQuery = user input problem, ConfigError = system setup problem |
| vector_filter.rs | Type stored as string in metadata | JSON metadata is storage-agnostic; string comparison avoids enum versioning issues |

## Risk Assessment

- LOW risk: Adding comments doesn't change behavior
- MEDIUM value: High-signal WHY comments accelerate onboarding
- Adding edge case tests is pure upside (no risk, catches regressions)
