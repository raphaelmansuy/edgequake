# Mission: Reliable Document Ingestion Pipeline

## Task

Your mission is to ensure the document upload, embedding, and knowledge graph (KG) building pipeline is fully functional, robust, and follows SRP (Single Responsibility Principle) and DRY (Don't Repeat Yourself) principles.

Don't take snapshot using playwright because it saturates memory.

FULLY READ THIS MISSION FILE AT THE START OF EVERY OODA ITERATION TO AVOID ALIGNMENT DRIFT.

Fully execute 50 OODA iterations minimum, producing the required 4 files per iteration.

Fully test using playwrigtht use e2e ingestion with several document with using gpt-5-nano, to prove the ingestion pipeline works end to end. Ensure all edge cases are handled, including large files, corrupted files, timeouts, and partial failures.

### Key Objectives:

1. **Test Document Upload**: Use Playwright browser automation to upload test PDF documents and verify the complete ingestion pipeline works
2. **Fix Stuck Documents**: Identify and resolve any documents that get stuck during processing
3. **Remove In-Memory Providers**: Eliminate all in-memory storage providers to ensure consistency - only PostgreSQL should be used
4. **Remove Dead/Duplicate Code**: Clean up unused code and eliminate duplication
5. **Update LLM Configuration**: Replace `gpt-4o-mini` with `gpt-5-nano` as the default OpenAI model (gpt-4o-mini quota exceeded)
6. **Ensure Robustness**: The pipeline should handle errors gracefully and recover from failures

## Context

- **Location**: `/Users/raphaelmansuy/Github/03-working/edgequake`
- **Test Documents**: `/Users/raphaelmansuy/Github/03-working/edgequake/zz-explore/EMILE_FREY/*.pdf`
- **Backend**: Rust-based API at `http://localhost:8080`
- **Frontend**: Next.js WebUI at `http://localhost:3000`
- **Database**: PostgreSQL with Apache AGE for graph storage

### LLM Model Update (CRITICAL)

**`gpt-4o-mini` is deprecated/quota exceeded. Use `gpt-5-nano` instead.**

```bash
# Working gpt-5-nano example:
curl https://api.openai.com/v1/responses \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -d '{
    "model": "gpt-5-nano",
    "input": "write a haiku about ai",
    "store": true
  }'
```

---

## Process: OODA Loop (50 iterations minimum)

Execute iterative OODA cycles. Each iteration produces 4 files:

**You Must absolutely read your mission every iteration! It is vital to avoid alignment drift. You can forget previous iterations, but never forget your mission.**

**Mission file**: `./specs/001-reliable-ingestion-mission.md`

You Must always produce the 4 files per iteration, as shown below:

1. `observe.md` → Map the territory. Never make assumptions about code structure or function. Always verify against the actual codebase. When you don't know, go check the code or search on the web for answers and documentation
2. `orient.md` → Analyze your findings and define possible solutions using First Principles as your north star. Assess risks and benefits of each approach.
3. `decide.md` → Prioritize specific changes to be made based on signal value and impact.
4. `act.md` → Implement the decided changes with precision, update the documentation, and reference specific file:line numbers and commit SHAs.

```
001-reliable-ingestion-mission/ooda/
├── iteration_01/
│   ├── observe.md   # Data gathered: code, business rules, workflows
│   ├── orient.md    # Analysis of findings vs. current docs
│   ├── decide.md    # Prioritized action plan
│   └── act.md       # Changes made, with file:line references + commit hashes
├── iteration_02/
│   └── observe.md
│   └── orient.md
│   └── decide.md
│   └── act.md
├── iteration_03/
│   └── ...
└── summary.md       # Cross-iteration insights
```

### Per-Iteration Requirements

| Step        | Output                                                     |
| ----------- | ---------------------------------------------------------- |
| **Observe** | Code analysis, feature inventory, dependency mapping       |
| **Orient**  | Gap analysis, documentation quality assessment             |
| **Decide**  | Specific changes prioritized by signal value               |
| **Act**     | Implementation with commit (`OODA-XX: <decision summary>`) |

### Constraints

1. **Re-read mission** every iteration: mission file `./specs/001-reliable-ingestion-mission.md`
2. **Continue** from existing iterations—never restart
3. **Reference** real code: file paths, line numbers, commit SHAs
4. **Use ASCII diagrams** for architecture/flow visualization
5. **Split large files** for maintainability, Use Single Responsibility Principle (SRP)
6. **Optimize** Rust build speed (latest toolchain)
7. **Document amendments** in WHY, high signal value, and precise terms in comments in the codebase. Use ASCII diagrams where applicable.
8. **You must perform tests** and deliver evidence that all tests are passing after your changes.

**YOU Must Read your mission every iteration! It is vital to avoid alignment drift. You can forget previous iterations, but never forget your mission.**

You must always map the territory you are documenting. Never make assumptions about code structure or function. Always verify against the actual codebase.

If you don't know, make a search on the Web.

Always use First Principle Thinking as your north star.

---

## ⚠️ CRITICAL SAFETY MANDATE ⚠️

**YOU MUST RE-READ THIS ENTIRE MISSION FILE AT THE START OF EVERY OODA ITERATION.**

Failure to re-read causes alignment drift → catastrophic safety issues → user frustration → system unreliability.

---

## Success Criteria

- [ ] Document upload via UI works end-to-end
- [ ] Document processing completes (not stuck)
- [ ] Knowledge graph is populated with entities and relationships
- [ ] No in-memory providers remain in the codebase
- [ ] `gpt-5-nano` is the default OpenAI model
- [ ] All tests pass
- [ ] No dead code or duplicate code
- [ ] SRP and DRY principles are followed
- [ ] Ensure no hardcoded models in the codebase
- [ ] The ingestion pipeline is robust and recovers from errors
- [ ] Edge case handling is implemented for large files, timeouts, and partial failures
- [ ] Ensure gpt-5-nano works for ingestion
- [ ] Document that Memory mode is only for test and NEVER for during Makefile dev or production runs. Make test that makefile dev fails if DATABASE_URL is not set.
- [ ] Document the best way to run EdgeQuake in dev mode during testing session. (Use your experience from this mission to write the best possible doc)
- [ ] Ensure delete document works fully (including PDF storage cleanup)
- [ ] Ensure 2 documents can be ingested in parallel without issues
- [ ] Ensure ingestion works with both Ollama and OpenAI LLM providers
- [ ] Ensure query works with both Ollama and OpenAI LLM providers
- [ ] Ensure query works for document uploaded via the UI
- [ ] Ensure Mock Provider is never displayed as an option in the UI or API during Makefile dev or production runs.
- [ ] Ensure chunk size is adapted based on embedding model and llm model context length. It must be dynamic to always find the best chunk size based on model capabilities.
- [ ] Ensure High Signal comments are added in the codebase for every change made during this mission, explaining WHY the change was made, with precise terms. Use high value ASCI diagrams where applicable.
- [ ] Ensure health API make it easy to know all parts of the applied configuration (llm provider, embedding provider, models used, database connection status, pdf storage status, etc.)
- [ ] Audit pipeline processing code to ensure all errors are properly handled and propagated using Result<T, Error> types. No panics allowed.
- [ ] Ensure comprehensive logging is in place for debugging ingestion issues.
- [ ] Ensure it is impossible to have silent failures in the ingestion pipeline. All errors must be logged and propagated.
- [ ] Ensure it always possible to cancel the status of document stuck in processing state via the API or UI.
- [ ] Ensure no code duplication exists for the ingestion pipeline and query pipeline. Shared logic must be refactored into common modules. SRP and DRY principles must be followed.
- [ ] Ensure the queuing system used for document processing is robust and can handle retries, backoffs, and failures gracefully and is managed using database persistence.

--> The OpenAI API quota is exceeded demonstrate an issue, switch to the latest embedding and llm models available on openai. Use the cheapest possible models that work well for document ingestion. It is vital to ensure the ingestion pipeline works end to end with openai models as well as ollama models.

---

## Model Pricing Reference (OODA-10)

### OpenAI Cheapest Models (2026-02)

| Type          | Model                    | Price                           | Notes                                 |
| ------------- | ------------------------ | ------------------------------- | ------------------------------------- |
| **LLM**       | `gpt-5-nano`             | $0.05/1M input, $0.40/1M output | 3x cheaper than gpt-4o-mini ✅        |
| **Embedding** | `text-embedding-3-small` | $0.02/1M tokens                 | 5x cheaper than ada-002, 1536 dims ✅ |

### Ollama Default Models (No API costs)

| Type          | Model            | Dimension | Notes                        |
| ------------- | ---------------- | --------- | ---------------------------- |
| **LLM**       | `gemma3:12b`     | N/A       | 128K context, vision support |
| **Embedding** | `embeddinggemma` | 768       | Good for local development   |

### Troubleshooting Quota Exceeded

If you see "You exceeded your current quota":

1. **Check OpenAI dashboard**: https://platform.openai.com/usage
2. **Verify billing status**: https://platform.openai.com/account/billing
3. **Use Ollama for development**: No API costs, works locally
4. **Switch provider via env var**:

   ```bash
   # Use Ollama (default)
   EDGEQUAKE_DEFAULT_LLM_PROVIDER=ollama

   # Use OpenAI (requires OPENAI_API_KEY)
   EDGEQUAKE_DEFAULT_LLM_PROVIDER=openai
   ```

Test with this cURL command to confirm OpenAI works:

```bash

Account to use for openai:

curl https://api.openai.com/v1/responses \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer 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" \
  -d '{
    "model": "gpt-5-nano",
    "input": "write a haiku about ai",
    "store": true
  }'


  ENSURE TO TEST WITH OPENAI TO PROVE IT WORKS END TO END. THIS IS CRITICAL !!!!



Update to use gpt-4.1-nano instead of gpt-5-nano
```
