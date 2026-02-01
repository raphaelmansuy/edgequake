# Mission: Unify Document Ingestion Pipeline

## Task

Your mission is to unify the document ingestion, extraction, and knowledge graph building pipeline to handle both PDF and Markdown documents through a single, cohesive flow with consistent status tracking and error reporting.

YOU MUST RE-READ THIS ENTIRE MISSION FILE AT THE START OF EVERY OODA ITERATION.
YOU MUST  FULLY READ THIS ENTIRE MISSION FILE AT THE START OF EVERY OODA ITERATION.

## Context

- **Location**: `/Users/raphaelmansuy/Github/03-working/edgequake`

---

## Objectives

### 1. Unified Ingestion Flow

Ensure document upload, ingestion, extraction, and KG/embedding building is unified:

```
┌─────────────────────────────────────────────────────────────────────┐
│                    UNIFIED INGESTION PIPELINE                        │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌─────────┐                                                         │
│  │  PDF    │──► Store as PDF ──► Convert to Markdown ──┐             │
│  └─────────┘                                            │            │
│                                                         ▼            │
│                                        ┌───────────────────────────┐ │
│                                        │   Unified KG Pipeline     │ │
│                                        │                           │ │
│                                        │  • Chunking               │ │
│                                        │  • Entity Extraction      │ │
│                                        │  • Relationship Extraction│ │
│                                        │  • Graph Merging          │ │
│                                        │  • Embedding Generation   │ │
│                                        │  • Vector Storage         │ │
│                                        └───────────────────────────┘ │
│                                                         ▲            │
│  ┌─────────┐                                            │            │
│  │Markdown │────────────────────────────────────────────┘            │
│  └─────────┘                                                         │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### 2. Unified Status Tracking

Display progression of ingestion for both PDF and Markdown in Documents panel:

| Stage        | PDF | Markdown | Description                    |
| ------------ | --- | -------- | ------------------------------ |
| `uploading`  | ✓   | ✓        | File being uploaded            |
| `converting` | ✓   | -        | PDF → Markdown conversion      |
| `chunking`   | ✓   | ✓        | Document chunking              |
| `extracting` | ✓   | ✓        | Entity/relationship extraction |
| `embedding`  | ✓   | ✓        | Vector embedding generation    |
| `indexing`   | ✓   | ✓        | Graph/vector storage           |
| `completed`  | ✓   | ✓        | Successfully indexed           |
| `failed`     | ✓   | ✓        | Pipeline error                 |

### 3. Unified Pipeline Display

PDF conversion state and Markdown processing displayed in unified way:

- Business-informative status messages
- Clear progression indicators
- Meaningful error messages that explain what failed

### 4. Code Quality Principles

- **SRP (Single Responsibility Principle)**: Each module handles one concern
- **DRY (Don't Repeat Yourself)**: Shared logic between PDF and Markdown flows
- **KISS (Keep It Simple, Stupid)**: Minimize complexity

---

## Current State Analysis

### Backend (Rust)

- `edgequake-api/src/handlers/documents.rs`: Markdown/text upload handler
- `edgequake-api/src/handlers/pdf_upload.rs`: PDF-specific upload handler
- `edgequake-pipeline/src/progress.rs`: Pipeline progress tracking

### Frontend (React/TypeScript)

- `components/documents/document-manager.tsx`: Document list and upload UI
- `components/documents/status-badge.tsx`: Status visualization
- `components/documents/ingestion-progress-panel.tsx`: Real-time progress

---

## Process: OODA Loop (50 iterations minimum)

Execute iterative OODA cycles. Each iteration produces 4 files:

**You Must absolutely read your mission every iteration! It is vital to avoid alignment drift. You can forget previous iterations, but never forget your mission.**

Mission file: `./specs/002-unify-ingestion-pipeline.md`

You Must always produce the 4 files per iteration, as shown below:

1 - observe.md → Map the territory. Never make assumptions about code structure or function. Always verify against the actual codebase. When you don't know, go check the code or search on the web for answers and documentation
2 - orient.md → Analyze your findings and define possible solutions using First Principles as your north star. Assess risks and benefits of each approach.
3 - decide.md → Prioritize specific changes to be made based on signal value and impact.
4 - act.md → Implement the decided changes with precision, update the documentation, and reference specific file:line numbers and commit SHAs.

```
002-unify-ingestion-pipeline/ooda_loop/
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

1. **Re-read mission** every iteration: mission file `./specs/002-unify-ingestion-pipeline.md`
2. **Continue** from existing iterations—never restart
3. **Reference** real code: file paths, line numbers, commit SHAs
4. **Use ASCII diagrams** for architecture/flow visualization
5. **Split large files** for maintainability, Use Simple Responsibility Principle (SRP)
6. **Optimize** Rust build speed (latest toolchain)
7. **Document amendments** in WHY, high signal value, and precise terms in comments in the codebase. Use ASCII diagrams where applicable.
8. **You must perform tests** and deliver evidence that all tests are passing after your changes.

**YOU Must Read your mission every iteration! It is vital to avoid alignment drift. You can forget previous iterations, but never forget your mission.**

You must always map the territory you are documenting. Never make assumptions about code structure or function. Always verify against the actual codebase.

If you don't know make a search on the Web.

Always use First Principle Thinking as your north star.

---

## Deliverables

1. **Unified Backend Pipeline**
   - Single ingestion flow handling both PDF and Markdown
   - Shared progress tracking types
   - Unified error types

2. **Unified Frontend Components**
   - Consistent status display for all document types
   - Unified progress panel
   - Clear error messaging

3. **E2E Tests**
   - PDF upload → ingestion → KG extraction verified
   - Markdown upload → ingestion → KG extraction verified
   - Status progression displayed correctly
   - Error states handled properly

---

## ⚠️ CRITICAL SAFETY MANDATE ⚠️

**YOU MUST RE-READ THIS ENTIRE MISSION FILE AT THE START OF EVERY OODA ITERATION.**

Failure to re-read causes alignment drift → catastrophic safety issues → user frustration → system unreliability.

Mission file to read at each OODA Loop: `./specs/002-unify-ingestion-pipeline.md`
