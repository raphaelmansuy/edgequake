# Mission: Bullet-Proof Document Ingestion Process

## Task

Your mission is to **investigate, identify root causes, and implement a bulletproof document ingestion pipeline** that reliably processes documents of all sizes (small: <10KB, medium: 10-100KB, large: >100KB) with both OpenAI and Ollama LLM providers. The system must be fast, smooth, reliable, and production-ready.

## Context

- **Location**: EdgeQuake document ingestion pipeline
  - Backend: `edgequake/crates/edgequake-api/src/handlers/documents.rs`
  - Pipeline: `edgequake/crates/edgequake-pipeline/`
  - LLM Layer: `edgequake/crates/edgequake-llm/`
  - Test Documents: `zz-explore/test_docs/`
    - `aws_2601.08734v1.extracted.md` (86,408 bytes)
    - `scienti_2601.16282v1.extracted.md` (123,909 bytes)

- **Current Issues Observed**:
  1. Document upload requests **hang indefinitely** for large documents (121KB)
  2. Network connection errors with Ollama provider
  3. Timeout configuration may still be insufficient despite 600s increase
  4. No streaming progress indicators for long-running operations
  5. No graceful degradation or partial success handling

- **Success Criteria**:
  - ✅ Both test documents process successfully with Ollama
  - ✅ Both test documents process successfully with OpenAI
  - ✅ Processing time is reasonable (<5 minutes for 121KB)
  - ✅ Clear progress indicators during processing
  - ✅ Robust error handling with actionable error messages
  - ✅ No silent failures or indefinite hangs
  - ✅ Comprehensive test suite proving reliability
  - ✅ Production-ready monitoring and observability

---

## ⚠️ CRITICAL SAFETY MANDATE ⚠️

**YOU MUST RE-READ THIS ENTIRE MISSION FILE AT THE START OF EVERY OODA ITERATION.**

Failure to re-read causes alignment drift → catastrophic safety issues → user frustration → system unreliability.

**Before each iteration, execute**:

```bash
cat /Users/raphaelmansuy/Github/03-working/edgequake/specs/002-bullet-proof-ingestion-process.md
```

---

## Process: OODA Loop (Minimum 50 Iterations)

Execute iterative OODA cycles. Each iteration produces 4 files:

```
specs/002-bullet-proof-ingestion-process/ooda_loop/
├── iteration_01/
│   ├── observe.md   # Data gathered: code, logs, system state
│   ├── orient.md    # Analysis using First Principles
│   ├── decide.md    # Prioritized action plan
│   └── act.md       # Implementation with file:line + commit SHA
├── iteration_02/
│   ├── observe.md
│   ├── orient.md
│   ├── decide.md
│   └── act.md
├── iteration_03/
│   └── ...
└── summary.md       # Cross-iteration insights
```

### Per-Iteration Requirements

| Step        | Output                                                                 |
| ----------- | ---------------------------------------------------------------------- |
| **Observe** | System state, logs, code analysis, performance metrics, error patterns |
| **Orient**  | Root cause analysis using First Principles, dependency mapping         |
| **Decide**  | Specific changes prioritized by signal value and risk                  |
| **Act**     | Implementation + tests + commit (`OODA-XX: <decision summary>`)        |

### Iteration Output Structure

Each `observe.md` **MUST** contain:

1. **Current System State**
   - Backend health check output
   - Ollama service status
   - Active processes (PIDs)
   - Environment variables
2. **Code Investigation**
   - Files read with line numbers
   - Function call chains
   - Dependency relationships
3. **Test Results**
   - Upload attempts (success/failure)
   - Log excerpts (errors, warnings, timing)
   - Network traces if applicable
4. **Measurements**
   - Document sizes
   - Processing times (if successful)
   - Memory/CPU usage
   - Network latency

Each `orient.md` **MUST** contain:

1. **First Principles Analysis**
   - What is the fundamental requirement? (e.g., "Transform text → knowledge graph")
   - What are the immutable constraints? (e.g., LLM API rate limits, memory limits)
   - What assumptions can we challenge? (e.g., "Must process entire document at once")
2. **Root Cause Hypothesis**
   - Evidence supporting hypothesis
   - Evidence contradicting hypothesis
   - Alternative explanations
3. **Risk Assessment**
   - What could break if we proceed?
   - What is the blast radius of changes?
   - Rollback strategy
4. **Solution Candidates**
   - Option A: Description, pros, cons, estimated effort
   - Option B: Description, pros, cons, estimated effort
   - Option C: Description, pros, cons, estimated effort

Each `decide.md` **MUST** contain:

1. **Chosen Solution** (with clear rationale)
2. **Implementation Plan**
   - File changes with line number ranges
   - New files to create
   - Tests to write/modify
   - Documentation updates
3. **Validation Criteria** (specific, measurable)
4. **Rollback Plan** (if changes fail)

Each `act.md` **MUST** contain:

1. **Changes Made**
   - File paths with line numbers
   - Code diffs (before/after snippets)
   - Commit SHA
2. **Tests Run**
   - Test commands executed
   - Test results (pass/fail)
   - Evidence (log excerpts, screenshots)
3. **Verification**
   - Health checks
   - Manual upload tests
   - Performance measurements
4. **Next Iteration Focus**

### Constraints

1. **Re-read mission** every iteration: `/Users/raphaelmansuy/Github/03-working/edgequake/specs/002-bullet-proof-ingestion-process.md`
2. **Continue** from existing iterations—never restart numbering
3. **Reference** real code: file paths, line numbers, commit SHAs
4. **Use ASCII diagrams** for architecture/flow visualization
5. **Split large files** for maintainability (Single Responsibility Principle)
6. **Optimize** Rust build speed (incremental builds, latest toolchain)
7. **Document WHY** in code comments using ASCII diagrams where helpful
8. **Perform tests** after every change and provide evidence
9. **Never assume** code structure—always verify by reading the actual file
10. **Search the web** when uncertain about libraries, best practices, or error messages

### First Principles Thinking Framework

For every decision, ask:

1. **What is the fundamental truth?** (e.g., "LLM calls are slow and can fail")
2. **What can we eliminate?** (e.g., "Do we need synchronous processing?")
3. **What can we simplify?** (e.g., "Can we chunk documents instead of processing whole?")
4. **What is the minimal viable solution?** (e.g., "Prove one 121KB document works")
5. **How do we measure success objectively?** (e.g., "Processing completes in <5min")

### Error Investigation Process

When encountering errors:

1. **Reproduce** the exact error with minimal test case
2. **Isolate** the failing component (API → Pipeline → LLM → Storage)
3. **Trace** the execution path with line-by-line code reading
4. **Search** for similar issues (GitHub, StackOverflow, official docs)
5. **Hypothesize** root cause with evidence
6. **Test** hypothesis with targeted fix
7. **Verify** fix solves problem without breaking existing functionality

### Testing Strategy

For each change:

1. **Unit Tests**: Isolated component testing
2. **Integration Tests**: End-to-end pipeline with mock LLM
3. **Manual Tests**: Real document upload with real LLM
4. **Performance Tests**: Measure processing time, memory, CPU
5. **Failure Tests**: Timeout scenarios, network errors, invalid input

### Deliverables

**Immediate** (First 10 Iterations):

- [ ] Reproduce the hang issue with 121KB document
- [ ] Identify exact line where processing stalls
- [ ] Implement timeout/progress logging
- [ ] Verify small document (86KB) processes successfully

**Short-Term** (Iterations 11-30):

- [ ] Fix root cause of large document hang
- [ ] Add streaming progress indicators
- [ ] Implement graceful timeout handling
- [ ] Test with both Ollama and OpenAI
- [ ] Document configuration best practices

**Long-Term** (Iterations 31-50+):

- [ ] Implement chunked processing for large documents
- [ ] Add comprehensive error handling
- [ ] Create monitoring dashboard
- [ ] Write production deployment guide
- [ ] Implement automated regression tests

### Success Metrics

Track these metrics every iteration:

| Metric                     | Target      | Current | Status |
| -------------------------- | ----------- | ------- | ------ |
| 86KB doc processing time   | < 2 minutes | ?       | ❌     |
| 121KB doc processing time  | < 5 minutes | HANGS   | ❌     |
| Ollama success rate        | 100%        | 0%      | ❌     |
| OpenAI success rate        | 100%        | ?       | ❌     |
| Entity extraction accuracy | > 90%       | ?       | ❌     |
| Timeout error rate         | 0%          | ?       | ❌     |
| Test coverage              | > 80%       | ?       | ❌     |
| Documentation completeness | 100%        | 50%     | 🟡     |

### Communication Rules

1. **Start each iteration** with: "Iteration XX - Reading mission file..."
2. **Document assumptions** explicitly in `orient.md`
3. **Show evidence** for every claim (log excerpts, code snippets)
4. **Use code fences** with language tags for syntax highlighting
5. **Create ASCII diagrams** for complex flows:
   ```
   User → API → Pipeline → LLM → Storage
                    ↓
                Timeout?
                    ↓
              Circuit Breaker
   ```
6. **Commit messages** follow convention: `OODA-XX: Brief summary of change`

### Archive Strategy

After every 10 iterations:

1. Create `summary_XX.md` consolidating insights
2. Update success metrics table
3. Document lessons learned
4. Identify patterns/recurring issues

---

## Initial Observations (Pre-Iteration)

**Test Documents**:

- `aws_2601.08734v1.extracted.md`: 86,408 bytes (84KB)
- `scienti_2601.16282v1.extracted.md`: 123,909 bytes (121KB)

**System State** (2026-01-28 14:45):

- Backend: Running (PID 36558) with Ollama provider
- Ollama: Running (localhost:11434) with 50+ models
- Frontend: Running (port 3000)
- Database: PostgreSQL (edgequake-postgres container)

**Observed Issues**:

1. Upload request for 121KB document **hangs indefinitely** (no response, no timeout)
2. Previous timeout fix (120s → 600s) may not be applied correctly
3. No progress indicators during processing
4. Backend logs show successful processing of small test docs but fail on production docs

**Hypothesis**:

- Timeout may not be applied at HTTP layer (only LLM layer)
- Document chunking may be failing for large docs
- Ollama may be slower than expected for entity extraction
- Memory exhaustion or resource contention

---

## Iteration Checklist

Before starting each iteration:

- [ ] Read mission file: `cat specs/002-bullet-proof-ingestion-process.md`
- [ ] Check previous iteration's `act.md` for context
- [ ] Verify system state (health checks, logs)
- [ ] Plan investigation focus (max 2 hours per iteration)

During iteration:

- [ ] Create all 4 files (`observe.md`, `orient.md`, `decide.md`, `act.md`)
- [ ] Run tests and document results
- [ ] Commit changes with `OODA-XX:` prefix
- [ ] Update success metrics table

After iteration:

- [ ] Verify changes didn't break existing functionality
- [ ] Document any new questions/blockers
- [ ] Plan next iteration focus

---

## CRITICAL REMINDERS

1. **Always verify code structure** - Never assume function signatures or file locations
2. **Search the web** when encountering unfamiliar errors or libraries
3. **Test incrementally** - Verify each small change before proceeding
4. **Document WHY** - Future maintainers need to understand reasoning
5. **Use First Principles** - Question every assumption
6. **Measure everything** - Use metrics to guide decisions
7. **Re-read mission** - Alignment drift is the #1 failure mode

---

## Start Command

```bash
# Iteration 01 - Begin investigation
mkdir -p specs/002-bullet-proof-ingestion-process/ooda_loop/iteration_01
cd specs/002-bullet-proof-ingestion-process/ooda_loop/iteration_01
cat /Users/raphaelmansuy/Github/03-working/edgequake/specs/002-bullet-proof-ingestion-process.md
```

**First action**: Reproduce the 121KB document hang and capture exact error/timeout behavior.

---

**Mission File Path**: `/Users/raphaelmansuy/Github/03-working/edgequake/specs/002-bullet-proof-ingestion-process.md`

**Last Updated**: 2026-01-28 14:50:00
