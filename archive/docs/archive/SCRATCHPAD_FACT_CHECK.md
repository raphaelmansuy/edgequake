# Documentation Fact-Check Scratchpad

## 0001-quick-start.md Findings

### ✅ VERIFIED CORRECT:

1. Project description as Graph-Enhanced RAG in Rust - CORRECT
2. Prerequisites (Rust, Node.js, PostgreSQL) - CORRECT
3. Basic build commands - CORRECT
4. EdgeQuake uses OpenAI with gpt-4o-mini by default - CORRECT (see orchestrator.rs L109)
5. Embedding model text-embedding-3-small with 1536 dimensions - CORRECT (see orchestrator.rs L113)
6. Memory storage constructors (MemoryKVStorage, MemoryVectorStorage, MemoryGraphStorage) - CORRECT
7. REST API endpoints exist at /api/v1/\* - CORRECT (see routes.rs)
8. Query modes: Naive, Local, Global, Hybrid, Mix, Bypass - CORRECT (types/query.rs has all 6)
9. Default query mode is Hybrid - CORRECT (types/query.rs L15)
10. production_pipeline.rs example exists - CORRECT

### ⚠️ NEEDS CORRECTION:

1. **QueryResult.answer field**: Doc shows `response.answer` but actual field is `response.response`
   - Location: edgequake/crates/edgequake-core/src/types/query.rs L100 shows `pub response: String`
2. **API Binary name**: Doc says `cargo run --bin edgequake-api` but actual binary is `edgequake`

   - Location: edgequake/Cargo.toml L22-24 shows `name = "edgequake"`

3. **Environment Variables WRONG**: Doc shows EDGEQUAKE\_\* prefixed vars but actual code uses:

   - `OPENAI_API_KEY` (not EDGEQUAKE_LLM_API_KEY) - edgequake/src/main.rs L26
   - `HOST` (not EDGEQUAKE_HOST) - edgequake/src/main.rs L69
   - `PORT` (not EDGEQUAKE_PORT) - edgequake/src/main.rs L70-73
   - `WORKER_THREADS` - edgequake/src/main.rs L49-51

4. **PostgresStorage unified interface WRONG**: Doc shows `PostgresStorage::connect()` but actual code has separate adapters

   - Code has: PostgresKVStorage, PgVectorStorage, PostgresAGEGraphStorage (storage/lib.rs L49-52)
   - Not a unified PostgresStorage

5. **Duplicate QueryMode definition in codebase** (not doc issue, code issue):
   - config.rs L183-194: Only 5 modes (missing Mix)
   - types/query.rs L4-24: All 6 modes (correct)
   - The types/query.rs version is exported and used

---

## 0002-architecture-overview.md Findings

### ✅ VERIFIED CORRECT:

1. Crate structure (edgequake-core, api, llm, storage, pipeline, query) - CORRECT
2. EdgeQuake orchestrator struct and methods - CORRECT (orchestrator.rs)
3. LLMProvider and EmbeddingProvider traits - CORRECT (llm/src/traits.rs)
4. Storage traits (KVStorage, VectorStorage, GraphStorage) - CORRECT
5. WebUI uses Next.js 16.1.0, React 19.2.3, Tailwind 4, Zustand, TanStack Query - CORRECT
6. Config structure (StorageConfig, LlmConfig, PipelineConfig, QueryConfig, ApiConfig) - CORRECT
7. Default chunk size 1200 tokens, overlap 100 tokens - CORRECT (config.rs L109-110)

### ⚠️ NEEDS CORRECTION:

1. **QueryMode in edgequake-query only has 5 modes** (no Bypass):

   - modes.rs has: Naive, Local, Global, Hybrid, Mix
   - config.rs has: Naive, Local, Global, Hybrid, Bypass (no Mix)
   - types/query.rs has: Naive, Local, Global, Hybrid, Mix, Bypass (all 6)
   - The doc references both Bypass and all 6 modes which is correct per types/query.rs

2. **edgequake-query crate location**: The doc says modes.rs is in edgequake-query but shows types/query.rs examples

   - The primary QueryMode is in edgequake-core/src/types/query.rs (exported)

3. **API Routes missing /live endpoint**: Doc doesn't mention /live but code has it

4. **Add code file references** for better navigation

### 🔍 Minor fixes needed:

- Fix API routes section to include all routes (auth, users, api-keys, etc.)
- Clarify which QueryMode definition is canonical (types/query.rs)
