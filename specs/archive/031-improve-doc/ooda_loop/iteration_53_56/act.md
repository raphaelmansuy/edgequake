# OODA Loop Iteration 53-56: Archive Cleanup

## Observe

The docs/ directory contained many historical implementation docs mixed with core documentation:

- Working notes (craftpad.md, deep-reflection-doc-sync.md)
- Completion summaries (TASK_COMPLETION_SUMMARY.md)
- SOTA comparison docs (sota-\*.md, edgequake-vs-lightrag-sota.md)
- Performance benchmarks (layout-performance-benchmark.md)
- Implementation status (source-citations-status.md, test-plan-source-citations.md)

**Before cleanup**: 28 files in docs/
**After cleanup**: 12 core files

## Orient

These historical docs are valuable for reference but clutter the main documentation:

- New users get confused by mixing guides with implementation notes
- Historical docs get stale faster than core docs
- Core documentation should be discoverable at a glance

## Decide

Move all non-core documentation to docs/archive/:

1. Working notes and scratchpads
2. Task completion summaries
3. Historical SOTA comparisons
4. Performance benchmarks
5. Implementation status reports

Keep in main docs/:

1. README.md - main entry point
2. Numbered guides (0001-0009) - core documentation
3. Registries (features.md, business_rules.md, use_cases.md) - reference

## Act

### Files Moved to Archive

1. `craftpad.md` - working notes
2. `deep-reflection-doc-sync.md` - process reflection
3. `TASK_COMPLETION_SUMMARY.md` - completion summary
4. `SQL_OPTIMIZATION_BREAKTHROUGH.md` - historical implementation
5. `sota-implementation-summary.md` - SOTA implementation record
6. `sota-graph-query-comparison.md` - SOTA comparison
7. `graph-optimization-sse-streaming.md` - optimization notes
8. `layout-performance-benchmark.md` - benchmark results
9. `cost-tracking-sota-evaluation.md` - evaluation metrics
10. `source-citations-status.md` - implementation status
11. `test-plan-source-citations.md` - test plan
12. `full-stack-integration.md` - integration notes
13. `QUICK_REFERENCE.md` - quick reference for old work
14. `edgequake-vs-lightrag-sota.md` - comparison doc
15. `production-llm-integration.md` - integration guide

### Final Directory Structure

```
docs/
├── README.md                      # Entry point
├── 0001-quick-start.md           # Getting started
├── 0002-architecture-overview.md  # System design
├── 0003-api-reference.md         # REST API
├── 0004-storage-backends.md      # Storage config
├── 0005-llm-integration.md       # LLM providers
├── 0006-deployment-guide.md      # Production deployment
├── 0007-configuration-reference.md # All config options
├── 0008-multi-tenancy.md         # Namespace isolation
├── 0009-algorithms-reference.md  # Core algorithms
├── features.md                    # FEAT0001-XXXX registry
├── business_rules.md              # BR0001-XXXX registry
├── use_cases.md                   # UC0001-XXXX registry
└── archive/                       # Historical docs (37 files)
```

## Result

- **12 core files** in docs/ (from 28)
- **37 archived files** in docs/archive/
- Clean, navigable documentation structure
- Historical docs preserved but not cluttering main view

## Next Steps

- Update README.md if any links are broken
- Add archive/README.md explaining historical docs
