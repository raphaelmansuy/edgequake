# Observe - Iteration 140

## Focus: Workspace Page with Rebuild Actions (Item 4)

Verifying SPEC-032 requirement:

- **Item 4**: Workspace page displays features and allows changing embedding/LLM provider with rebuild

## Investigation

### Workspace Page

**File**: `edgequake_webui/src/app/(dashboard)/workspace/page.tsx`

- Lines: 733
- Imports: `RebuildEmbeddingsButton`, `RebuildKnowledgeGraphButton`

### Key Features

1. **Model Selectors** (lines 497, 546):

   - `LLMModelSelector` with `onChange={setSelectedLLM}`
   - `EmbeddingModelSelector` with `onChange={setSelectedEmbedding}`

2. **Change Detection** (lines 259-270):

   - `embeddingModelChanged` - detects if embedding model differs from workspace config
   - `llmModelChanged` - detects if LLM model differs from workspace config

3. **Change Warnings** (lines 504, 552):

   - LLM change: "Changing LLM model requires re-extracting entities from all documents."
   - Embedding change: "Changing embedding model requires rebuilding all document embeddings."

4. **Rebuild Actions** (lines 670, 680):

   - `RebuildEmbeddingsButton` - regenerates embeddings
   - `RebuildKnowledgeGraphButton` with `rebuildEmbeddings={true}`

5. **Pending Rebuild Indicators** (lines 657-661):
   - Shows which changes are pending
   - Prompts user to click rebuild button

## Findings

Item 4 is fully implemented:

- ✅ Workspace page exists at `/workspace`
- ✅ LLM model selection available
- ✅ Embedding model selection available
- ✅ Change detection with warnings
- ✅ Rebuild embeddings button
- ✅ Rebuild knowledge graph button
- ✅ Processing information displayed
