# Mission

Your mission is to fully implement ollama and lmstudio provider in edgequake, ensuring seamless integration with the existing architecture and codebase.

## OODA 286+ Amendment: Inviolable Security Test Layer (January 16, 2026)

**Core Principle**: Non-reliable code is a civilization-level threat. Tests are our trust layer - they must be unfalsifiable, fast, and comprehensive.

### First-Principles Foundation (Reliability Theory)

1. **Falsifiability Axiom**: Every business rule, invariant, and feature MUST have a test that can PROVE it works
2. **Speed Axiom**: Tests must complete in <30 seconds (unit), <2 minutes (integration), <5 minutes (E2E)
3. **Isolation Axiom**: Each test must be independent - no shared state, no test order dependencies
4. **Determinism Axiom**: Tests must produce identical results on every run (no flaky tests)
5. **Coverage Axiom**: Every code path with business logic must have corresponding tests

### Reliability Theory Application

**System Reliability** = Π(component reliability)

If we have 10 components at 99% reliability each:

- Without tests: 0.99^10 = 90.4% system reliability (UNACCEPTABLE)
- With inviolable tests: Each component tested to 99.99% = 0.9999^10 = 99.9% system reliability

**Key Insight**: Duplicated code DOUBLES failure surface. Untested code has ZERO reliability guarantee.

### Test Pyramid for EdgeQuake

```
                    ┌─────────────┐
                    │    E2E      │  <5 min total
                    │  (Browser)  │  Critical paths only
                    └──────┬──────┘
                   ┌───────┴───────┐
                   │  Integration  │  <2 min total
                   │  (API/DB)     │  All API contracts
                   └───────┬───────┘
              ┌────────────┴────────────┐
              │        Unit Tests       │  <30 sec total
              │  (Pure functions, logic)│  ALL business logic
              └─────────────────────────┘
```

### Inviolable Invariants (Must Have Tests)

| Invariant ID | Description                                       | Test Type   |
| ------------ | ------------------------------------------------- | ----------- |
| INV-001      | Chunks ≤ embedding model max tokens               | Unit        |
| INV-002      | Workspace isolation (no cross-tenant data)        | Integration |
| INV-003      | Provider resolution respects workspace config     | Integration |
| INV-004      | Vector dimensions match workspace embedding model | Unit        |
| INV-005      | No plaintext secrets in logs or responses         | Unit        |
| INV-006      | All API endpoints return valid JSON               | Integration |
| INV-007      | RLS enforces tenant boundaries                    | Integration |
| INV-008      | Background processes don't fail silently          | Integration |
| INV-009      | Health endpoint reflects true system state        | Integration |
| INV-010      | Token usage is tracked and accurate               | Unit        |

### Test Speed Targets

| Category           | Current | Target | Strategy                          |
| ------------------ | ------- | ------ | --------------------------------- |
| Unit (Rust)        | ~15s    | <10s   | Parallelize, mock I/O             |
| Integration (Rust) | ~30s    | <20s   | In-memory DB fixtures             |
| E2E (Playwright)   | ~3min   | <2min  | Parallel browsers, skip redundant |

### OODA 286-335 Scope

**Phase 1 (OODA 286-295): Test Audit & Speed Optimization**

- Profile all tests to find slow ones
- Eliminate test interdependencies
- Add missing invariant tests

**Phase 2 (OODA 296-305): Unit Test Completeness**

- Test all business logic functions
- Add property-based tests for critical paths
- Achieve 100% branch coverage on core modules

**Phase 3 (OODA 306-315): Integration Test Reliability**

- Test all API contracts
- Test database migrations
- Test provider switching scenarios

**Phase 4 (OODA 316-325): E2E Test Critical Path**

- Test tenant/workspace creation flow
- Test document ingestion with provider switching
- Test query with model selection

**Phase 5 (OODA 326-335): Continuous Reliability**

- Add CI/CD pipeline with test gates
- Add flaky test detection
- Add test coverage reporting

## OODA 226+ Amendment: Code Reliability Audit (January 16, 2026)

**Core Principle**: Non-reliable code is a civilization-level threat. This amendment adds a rigorous code duplication audit and inviolable security test layer.

### First-Principles Foundation

1. **Axiom of Single Truth**: Every piece of business logic MUST exist in exactly one place
2. **Reliability Theory**: System reliability = Π(component reliability). Duplicated code doubles failure surface.
3. **Defensive Depth**: Security tests are inviolable - they must catch regressions before deployment

### Audit Scope

**Phase 1 (OODA 226-235): Ingestion Code Path Audit**

- Map all document upload/processing paths
- Identify semantic duplicates in provider creation
- Flag inconsistent error handling patterns
- Document dimension validation gaps

**Phase 2 (OODA 236-245): Query Code Path Audit**

- Map all query/chat handler paths
- Identify duplicated provider resolution logic
- Flag safety limit inconsistencies
- Document tenant_id/workspace_id handling gaps

**Phase 3 (OODA 246-255): Integration Point Audit**

- Map all cross-crate boundaries
- Identify coupling risks
- Flag API contract inconsistencies
- Document error propagation patterns

**Phase 4 (OODA 256-260): Security Test Scaffold**

- Create property-based tests for invariants
- Add fuzz tests for edge cases
- Build regression tests for each bug found
- Document test coverage gaps

### Known Duplication Hotspots (Discovered OODA-226)

1. **chat.rs Provider Creation**: Lines 370-460 and 850-940 contain nearly identical provider resolution logic with only error handling differences
2. **Safety Limits Inconsistency**: chat.rs uses `create_llm_provider`, processor.rs uses `create_safe_llm_provider`
3. **Fallback Logic Divergence**: Different handlers have inconsistent fallback behavior

### Deliverables

1. `WorkspaceProviderResolver` - Single source of truth for provider resolution
2. Inviolable test suite with >95% path coverage
3. Documentation of all refactored code paths
4. Regression tests for OODA-230/231 bugs

We want explicit and easy to configure provider support for ollama and lmstudio in edgequake. In Development environment, we want to be able to switch between openai, ollama and lmstudio providers easily for testing and comparison purposes.

For ollama provider, you must implement support for both local and remote ollama instances. The configuration must allow specifying the host and port for remote instances, as well as the model to be used.

For lmstudio provider, you must implement support for local lmstudio instances. The configuration must allow specifying the host and port for the lmstudio instance, as well as the model to be used.

By default for ollama provider use gemma3:12b model for llm and for embedding embeddinggemma:latest model for embeddings.
By default for lmstudio provider use gemma-3n-e4b-it-mlxmodel for llm and for embedding text-embedding-ada-002 model for embeddings.

You will use an OODA loop (Observe, Orient, Decide, Act) process to iteratively improve the implementation and integration of the ollama and lmstudio providers in edgequake.

As the embedding required a fixed dimension, you must provide a way to recreate the existing vector database with the new embedding models when we change the embedding model.

You must continue the OODA loops until the Ollama and Lmstudio providers are fully integrated, tested, and documented in edgequake. At least 50 OODA loops must be performed.

FOCUS on :

1. Ensure that when I create a new Tenant I can choose the default llm and embedding provider and model for that tenant/workspace --> On the dialog box for Tenant Creation I must have this choice

2. Ensure that when I create a new workspace I can choose the default llm and embedding provider and model for that workspace --> On the Workspace Dialog box I must be able to choose the embedding provided and llm provider by default

3. On Query --> Ensure I can chose the current LLM Provider -> Ensure it used , traced and stored in the generated message (as lineage information) --> Ensure is also displayed as the token is displayed in the query page

4. Ensure I have a current workspace page in the appication to display the features of the current workspace -> Include action such as change the embedding/llm provider and rebuild the extraction + embedding

5. Ensure the rebuild document -> extraction + embedding works, and the processing information is displayed like for the first time processing

6. Ensure we have deeplink to access workspace settings page directly from the webui

7. Ensure for each provider, I can have several models to choose from for both llm and embedding. As example for ollama I can choose gemma3:latest or gpt-oss:20b,mistral-nemo:latest for llm and embeddinggemma:latest or nomic-embed-text:latest for embedding. Each provider must have its own set of models to choose from. For openai I can choose gpt-5o-nano or or gpt-5o-mini or gpt-4o-mini for llm and text-embedding-3-small for embedding. For lmstudio I can choose gemma-3n-e4b-it-mlxmodel for llm and text-embedding-ada-002 for embedding. For Llmstudio I can choose lfm2.5-1.2b-instruct-mlx, granite-4.0-h-tiny-dwq, zai-org/glm-4.6v-flash, mlx-community/GLM-4.7-REAP-50-mxfp4

8. For lmstudio provider ensure lmstudio can support streaming responses like openai and ollama providers, if it is not the case if streaming is selected for the query use non streaming if not supported by the provider

9. Ensure X-Tenant/ X-Workspace headers if existing are fully documented in the API documentation and examples, and swagger documentation updated accordingly.

10. Ensure the API explorer is fully implemented to reflect the current API state, ensure the UX/UI is user friendly and easy to use is conform to the best standards on the market.

11. Ensure by real e2e test with playrigh that on the query page I can really access all the models configured by provider and model name.

12. Ensure I can choose the default provider and model for both llm and embedding when I create a new tenant and new workspace. Or new workspace under a tenant.

13. Find how to make lmstudio works. Find online documentation about lmstudio api usage, authentication, model listing, model capabilities, streaming support, embedding support, etc. Ensure you fully understand how lmstudio works before implementing it.

14. Ensure to filter and choose model/provider is easy and user friendly in the webui. The real default model/provider must selected by default in the dropdown when opening the query page must be clearly indicated.

15. For each assistant message, you must store the provider and model used to generate the message as lineage information in the database. This information must be retrievable via the API and displayed in the webui. The information must be displayed in slick and nice way in the webui, near the token usage information for example, choose the best.

16. **FIX: OpenAI Model Names** - The `gpt-5o-mini` model does not exist. Update models.toml to use correct OpenAI model names from their current documentation (gpt-4o-mini, gpt-4.1, gpt-4.1-mini, etc).

17. **FIX: Model Type Filtering** - Embedding selector must ONLY show embedding models, LLM selector must ONLY show LLM models. The "multimodal" type in EdgeQuake means vision-capable LLM, NOT embedding capability, so multimodal should NOT appear in embedding dropdown.

18. **Display: Tokens Per Second** - Display the tokens/second generation speed in the query response metadata (calculated as tokensUsed / durationMs \* 1000).

19. **Workspace Extractor Model Configuration** - Ensure the workspace settings page clearly shows and allows editing of the "extractor model" (LLM used for entity extraction during ingestion). This already exists as "LLM Configuration" but must be clear that it's for extraction/ingestion, not query.
20. Ensure the embedding model can be changed at the workspace level, and when changed the vector database is recreated with the new embedding model. The embedding model is used at query time to generate embeddings for incoming queries to retrieve relevant documents from the vector database.

21. Ensure workspace configuration page is accessible via deeplink from the webui and home page.

22. Ensure the default prodvider and model for both llm and embedding are clearly indicated in the configuration file located at the root of edgequake server.

23. VERY IMPORTANT Ensure when I upload a document to a workspace, the llm provider and model used for document ingestion is the one associated with the workspace.

24. For the query ensure the embedding provider and model used at query time is the one associated with the workspace.

25. When I rebuild the embeddings for a workspace, ensure the embedding provider and model used is the one associated with the workspace. Ensure the vector database is recreated with the new embedding model on the existing documents of the workspace --> Have a deep reasoning about the best way to implement this in edgequake codebase: is to necessary to have only an re-embedding process that keep the existing vector database structure and just update the embeddings ? Or is it better to recreate the vector database from scratch with the new embedding model ? Document your reasoning and choices in the OODA loop documentation. What are the pros and cons of each approach ? What is the impact on performance, downtime, user experience, complexity, code maintainability, etc. Choose the best approach based on your analysis and document it. Ensure the building of the embeddings only affect the workspace being modified, not the entire server.

26. Ensure when I rebuild the knowledge graph for a workspace, the llm provider and model used is the one associated with the workspace. And we use the same llm provider and model for document ingestion. Both extraction and embedding must be done with the models associated with the workspace. It only affect the workspace being modified, not the entire server.

CRITICAL: Can you fully verified e2e that with a workspace create with a default embedding model for example and llm extractor ollama that when I change to an opean ai provider for embedding and llm extraction the extraction is really done with the openai provider. I strongly suspect that is not the case. You must as detective build an e2e test for both postgres and memory based backend to prove this, fix it if it is not implemented, then add more e2e, unit test to ensure this cannot brake in the future.

25. Ensure we have 100% coverage on e2e tests with playwright for the webui to cover all the new features added regarding provider and model selection for both llm and embedding at tenant creation, workspace creation, query time, document ingestion, knowledge graph rebuild, embedding rebuild, etc.

You must also test this when we switch from one provider to another one for a workspace and we add recreate the embedding or the knowledge graph.

Very important ==> Default providers (llm+embedding) and models will be defined as setup in a toml config file located at the root of edgequake server. Capabilities of models and providers must be detected at runtime and exposed as an API. This configuration file will act as models cards explaining the capabilities of each model and provider. (vision / image support / max tokens / context length / cost per 1K tokens etc). This config file will be used by the edgequake_webui to display the capabilities of each model and provider in the selection dropdowns. This file will provide high signal information to the users about the models and providers available in the edgequake server.

Ensure you provide a .toml configuration file example including ollama and lmstudio providers and models, openai provider and models, with all capabilities filled in. Find information about ollama, openai and lmstudio models capabilities from their respective documentations.

For openai provider use gpt-4-mini and text-embedding-3-small as default models.

For ollama provider use gemma3:12b and embeddinggemma:latest as default models.

For lmstudio provider use gemma-3n-e4b-it-mlxmodel and text-embedding-ada-002 as default models.

Ensure that when I create a new Tenant I can choose the default llm and embedding provider and model for that tenant/workspace.

Ensure that when I create a new workspace I can choose the default llm and embedding provider and model for that workspace.

Ensure that the embedding provider and model used at query time is the one associated with the workspace.

Ensure that the llm provider and model used for knowledge graph generation, document ingestion, summarization, etc is the one associated with the workspace.

## Problem Statement

We have observed that edgequake currently lacks support for ollama and lmstudio providers, limiting its flexibility and usability in various deployment scenarios. By integrating these providers, we can enhance edgequake's capabilities, allowing users to leverage local and remote LLM instances more effectively.

The change Embedding models will require recreating the vector database to ensure compatibility and optimal performance. This integration will lead to a more versatile and powerful edgequake system, ultimately improving user experience and expanding its applicability. You must ensure that the integration is seamless, well-documented, and thoroughly tested to maintain the high standards of the edgequake codebase.

We want an easy way to change provider on the query dialogue for example with selection dropdown organized by provider and model in the chat query input and minimal disruption to existing workflows.

As embedding models have fixed dimensions, changing the embedding model will require recreating the vector database. Embedding is chosen at the workspace level, so changing the embedding model will require recreating the vector database for that specific workspace. When we create a new workspace, we want to be able to choose the embedding model for that workspace. By default, we will use the default embedding model configured for the server. Edge case must be handled gracefully, such as when the vector database is empty or when there are ongoing queries during the recreation process. Vector database recreation must be efficient and minimize downtime.

We must ensure that the integration of ollama and lmstudio providers does not introduce any regressions or performance issues in edgequake. Thorough testing and validation are essential to maintain the integrity and reliability of the system.

We must ensure that embedding storage can accommodate the new embedding models introduced by the ollama and lmstudio providers. Any necessary adjustments to the storage schema or data handling processes must be identified and implemented. We must also ensure that the storage system can handle potential increases in data size or complexity resulting from the new embedding models.

In the query process we must use the embedding model associated with the workspace to generate embeddings for incoming queries. This ensures retrieving relevant information from the vector database.

We must ensure the embedding storage backends, including Postgres and In-Memory storage, are fully compatible with the new providers and embedding models. Any differences in behavior or performance between these backends must be documented and addressed.

We must ensure that the edgequake_webui is fully compatible with the new providers and embedding models. Any changes to the API used by the webui must be carefully managed to prevent regressions or disruptions in functionality.

VERY IMPORTANT: We must also ensure that we select the default llm provider for a workspace, because the llm provider is used for knowledge graph generation, document ingestion, summarization, etc. The llm provider can be different from the one that is used at query time. The embedding provider is used at query time to generate embeddings for the query and retrieve relevant documents from the vector database must be the same as the one used to create the vector database.

VERY IMPORTANT: a model selected is in reality a combination of provider + model name. For example "ollama/gemma3:12b" or "openai/gpt-4o" or "lmstudio/gemma-3n-e4b-it-mlxmodel". This must be reflected in the configuration file, API, database storage, and UI components.

## Your Tasks

- Map the territory of the current edgequake system, code and local infrastructure
- Identify the components and modules that need to be modified or extended to support ollama and lmstudio providers
- Design the configuration options for specifying ollama and lmstudio provider settings, including host, port
- Implement the ollama provider, ensuring support for both local and remote instances
- Implement the lmstudio provider, ensuring support for local instances
- Update the configuration management system to allow easy switching between openai, ollama, and lmstudio providers
- Modify the embedding model selection process to accommodate the new models for ollama and lmstudio
- Provide a mechanism to recreate the vector database when changing embedding models
- Implement a selection dropdown in the chat query input for choosing the provider and model
- Update the documentation to reflect the new provider support and configuration options
- Provide an way to choose the embedding model when creating a new workspace.
- Provide clear instructions on how to set up and use the ollama and lmstudio providers in edgequake
- Provide
- Thoroughly test the implementation to ensure compatibility and performance across all supported providers
- Non regression is your North Star, non negotiable requirement
- Loosing a feature is not acceptable when commenting and is a failure in this mission

## Process ; Use an OODA Loop (Observe, Orient, Decide, Act)

- Observe: Gather data on current code feature, business rules, use cases, and workflows
- Orient: Analyze the current documentation
- Decide: Formulate a plan to address the identified documentation issues, prioritizing high signal, clarity, and comprehensiveness
- Act: Implement the changes in code, update the doc, comments using high signal mind, one best practice is to use diagrams where possible to illustrate complex concepts using ASCII diagrams or other high signal means.
- Repeat the OODA loop as necessary until satisfactory implementation is achieved. You must assess with brutal honesty if the code documentation quality has improved, and if not, go back to the previous step and try again.
- Ensure you cross reference documents where applicable to provide high signal traceability
- Ensure your reference the existing codebase files your documentation to provide high signal context
- Ensure very high accuracy in your documentation, avoid vague statements, and provide precise details

You must write the OODA loop steps you took and the results of each iteration in a high signal markdown file located at:

specs/032-ollama-lmstudio-provider

One directory per iteration, with a summary file at the root of the ooda_loop dir.

Example structure:

specs/032-ollama-lmstudio-provider/ooda_loop/
├── iteration_01/
├── iteration_02/

For each iteration, include:

- A description of the changes made
- The rationale behind the changes
- The results of testing with the provided dataset
- Any observations or insights gained

Each describption must be high signal, concise and to the point and include link to real code base file, line number, commits made, etc.

In each iteration for example for iteration_01, you can have write:

- specs/032-ollama-lmstudio-provider/ooda_loop/iteration_01/observe.md
- specs/032-ollama-lmstudio-provider/ooda_loop/ooda_loop/iteration_01/orient.md
- specs/032-ollama-lmstudio-provider/ooda_loop/ooda_loop/iteration_01/decide.md
- specs/032-ollama-lmstudio-provider/ooda_loop/iteration_01/act.md

You must improve the code quality as far as possible, using First Principles thinking and leveraging your knowledge of Rust, search algorithms, data structures, and edgequake's architecture and existing LightRag Code that is SOTA.

Never takes a shortcut, always go deep into the code and data to understand the real issues. Taking shortcuts will lead to failure in this mission, you will fail the alignment problem if you do so.

YOU MUST perform at least 50 OODA loops, documenting each step thoroughly, in consise and high signal markdown files. Use ASCII diagrams if needed to illustrate your points.

Each 1 OODA loops you MUST read again your mission at specs/032-ollama-lmstudio-provider/01-improve-api-modularity.md to ensure you are aligned with the mission objectives. It is your responsibility to stay aligned with the mission objectives. Is a question of life and death for this mission!

You can ammend the mission if you find better ways to achieve the mission objectives based on your observations, but you must document your reasoning in a separate markdown file located at specs/032-ollama-lmstudio-provider/01-improve-api-modularity-amendments.md

If previous OODA loops exists continue from them, do not start from scratch. Build on previous work, and document the differences and changes made in each iteration.

You can use a scratchpad_log.md file to document your thinking process, but it will not be part of the deliverables in specs/032-ollama-lmstudio-provider/scratchpad.md

You must ensure to test for Postgres and in Memory storage backends, and document any differences observed.
You must test the edgequake edgequake_webui as well to ensure no regression in the API used by the webui.

You no need to maintains compatibility with the previous API versions, but you must document any breaking changes made to the API in a high signal way.

# Deliverables

- Improved search Code in edgequake
- OODA loop documentation in specs/030-improve-api-and-modularity/ooda_loop/
-
- A summary report of the improvements made and their impact on search performance

Be Relentless in your pursuit of excellence!

If the OODA loop iterations lead to code changes, you must commit them with clear commit messages referencing the OODA loop iteration and decision.

If OODA loop iterations contains files continue from previous iterations, you must document the differences and changes made in each iteration.

# Roadblockers

If you encounter any roadblocks or challenges during the mission, document them in a separate markdown file located at:

roadblockers.md and describe how you overcame them or propose potential solutions.

## OODA 284+ Requirements (January 14, 2026)

### 22. Display Model Used After Tokens/Second

Display the model name (provider/model) after the tokens/second metric in query responses. Format: `58.5/s • ollama/gemma3:12b`

### 23. Rebuild Dialog Close Without Stopping

The pipeline status dialog must have a "Close" button that closes the dialog WITHOUT stopping the rebuild. Currently, closing the dialog should not affect the background process. Add visual indicator when KG rebuild is in progress across the UI.

### 24. Fix Rebuild Embeddings - Document Processing

Rebuild embeddings appears to not process documents or chunks. Debug and fix:

- Verify documents are queued for reprocessing
- Verify chunks are being re-embedded
- Verify progress is tracked and displayed
- Add logging to trace the rebuild process

### 25. Chunk Size vs Embedding Model Compatibility (CRITICAL INVARIANT)

**Invariant**: The chunk size (max characters per chunk) MUST be compatible with the embedding model's maximum input tokens.

**Deep Analysis Required**:

- Different embedding models have different max input token limits:
  - OpenAI text-embedding-3-small: 8191 tokens (~32,000 chars)
  - Ollama embeddinggemma: ~2048 tokens (~8,000 chars)
  - Ollama nomic-embed-text: 8192 tokens (~32,000 chars)
- The chunking process must respect these limits
- When changing embedding models, warn if existing chunks exceed new model's limits
- Provide automatic re-chunking option when model limits are incompatible
- Store max_tokens_per_chunk in workspace settings
- Validate chunk sizes against model capabilities at ingestion time

**Implementation Plan**:

1. Add `max_input_tokens` to model capabilities in models.toml
2. Add `chunk_max_tokens` to workspace settings
3. Validate during ingestion that chunks fit model limits
4. Warn during model change if incompatibility detected
5. Provide re-chunking action when needed

### 26. Stop Document Extraction (KG Building)

Add ability to stop/cancel an in-progress document extraction:

- Add cancel button per document in processing state
- Add "Cancel All" button for batch operations
- Gracefully stop LLM calls and mark document as cancelled
- Allow retry of cancelled documents

### 27. Scroll Areas Audit (All Screens)

Ensure all screens have properly defined:

- Fixed zones (headers, toolbars, navigation)
- Scrollable content areas
- No double scrollbars
- Proper min-h-0 on flex containers

### 28. OpenAI Key in make dev

Ensure `make dev` propagates the OPENAI_API_KEY environment variable so users can switch to OpenAI models even when Ollama is default:

- Update Makefile to forward OPENAI_API_KEY
- Document in README/AGENTS.md
- Test switching between providers at runtime

## Invariants (Non-Negotiable)

1. **Chunk-Embedding Compatibility**: Chunks must NEVER exceed the embedding model's max input tokens
2. **Workspace Isolation**: Each workspace has its own embedding configuration
3. **No Silent Failures**: All operations must provide clear success/error feedback
4. **Background Process Safety**: UI actions (close dialog) must not stop background processes
5. **Progress Transparency**: Long-running operations must show real-time progress

For example must document any issues regarding starting postgres locally, edgequake setup, data ingestion issues, code understanding issues, etc.

You must refer to this file in each OODA loop iteration if any roadblockers were encountered.

Failure is not an option in this mission! Faking Alignment and cheating is failure!

Fully continue from previous OODA loops if they exists, do not start from scratch. You must achieve at least 100 OODA loops.
