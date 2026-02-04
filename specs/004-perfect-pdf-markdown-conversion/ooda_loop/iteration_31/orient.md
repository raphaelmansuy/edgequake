# OODA-31 ORIENT: Analysis and Direction

## Priority Assessment

Based on observations, using First Principles to prioritize:

### Decision Framework

```
                    ┌─────────────────────────────────┐
                    │    Mission Success Criteria     │
                    ├─────────────────────────────────┤
                    │ 1. Speed: <1s per page         │
                    │ 2. Quality: 95%+ TPS, SFS      │
                    │ 3. Tests: Rapid feedback loop   │
                    └─────────────────────────────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        ▼                     ▼                     ▼
   ┌─────────┐          ┌─────────┐          ┌─────────┐
   │ Speed   │          │ Quality │          │ Testing │
   │ 12s→1s  │          │ 68%→95% │          │ Instant │
   │ Gap: 12x│          │ Gap: 27%│          │ Feedback│
   └─────────┘          └─────────┘          └─────────┘
```

### First Principles Analysis

**Why Speed First (P0)?**

1. **Developer velocity** - Slow tests = slow iteration = slow progress
2. **User experience** - 12s per PDF means users wait minutes for results
3. **CI/CD blocking** - Slow tests block PRs and deployments
4. **Compound effect** - Speed improvements benefit every subsequent iteration

**Why Micro-Tests Second (P1)?**

1. **Feedback loop** - Currently no way to test single features quickly
2. **Debugging efficiency** - Comprehensive tests take 175s, too slow for iterating
3. **Isolation** - Need to test font encoding, table detection etc. independently
4. **Mission requirement** - Spec mandates micro-tests for <0.1s feedback

**Why Quality Third (P2)?**

1. **Depends on speed** - Can't iterate on quality without fast feedback
2. **Measurable progress** - TPS/SFS metrics guide improvements
3. **27% gap** - Significant but achievable with algorithm improvements

### Root Cause Analysis: Speed

**Hypothesis: Content parsing is the bottleneck**

```
PDF Parsing Flow (current):
┌──────────────────────────────────────────────────────────────────┐
│ lopdf::Document::load()                                          │
│   └─► Parse entire PDF into memory                               │
│         └─► For each page:                                       │
│               ├─► get_page_fonts() - Parse ALL fonts per page   │
│               ├─► get_page_content() - Decompress content       │
│               ├─► ContentParser::parse() - Character extraction │
│               │     └─► For each char: decode_char() O(1)       │
│               ├─► TextGrouper::group_into_lines() O(n log n)    │
│               └─► LatticeEngine::detect_tables() O(n log n)     │
└──────────────────────────────────────────────────────────────────┘
```

**Likely hotspots:**

1. Font loading/parsing for each page (should cache)
2. Content decompression (unavoidable, but can parallelize)
3. Character decoding with CMap lookup (should cache ToUnicode)

### Strategic Direction

**Phase 1: Create Micro-Tests (OODA-31-32)**

- Create minimal test PDFs
- Establish instant feedback loop
- Enable focused debugging

**Phase 2: Speed Profiling (OODA-33-35)**

- Add timing instrumentation
- Identify actual hotspots
- Target O(n²) algorithms if any remain

**Phase 3: Speed Optimization (OODA-36-40)**

- Lazy font loading
- Font caching across pages
- Parallel page processing enhancement

**Phase 4: Quality Improvements (OODA-41-100)**

- Table detection refinement
- Reading order accuracy
- Font encoding edge cases

### Risk Assessment

| Risk                                   | Likelihood | Impact | Mitigation                            |
| -------------------------------------- | ---------- | ------ | ------------------------------------- |
| Micro-test PDFs don't cover edge cases | Medium     | Low    | Add more as discovered                |
| Speed optimization breaks quality      | High       | High   | Run comprehensive tests before commit |
| Parallel processing race conditions    | Low        | Medium | Use Rayon's safe parallelism          |
| Font caching memory bloat              | Low        | Medium | LRU cache with size limit             |

### Recommended Action for OODA-31

**Clean up compiler warnings first** (quick win):

- Fix 5 unused variable/code warnings
- Improves code quality
- Shows immediate progress
- Takes <5 minutes

Then proceed to create micro-tests in OODA-32.
