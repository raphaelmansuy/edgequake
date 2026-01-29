# Mission: EdgeQuake High-Signal Documentation

## Task

Your mission is to create the perfect high-signal documentation for EdgeQuake - an advanced Retrieval-Augmented Generation (RAG) framework implemented in Rust with graph-based knowledge representation.


Ensure we have a high signal README.md at the root of the project that links to all major documentation sections.

Ensure we have an Apache2 license file at the root of the project.

Ensure we have contributing guidelines at the root of the project that explain that the project is 100% automated by a SOTA coding agent called edgecode created by Raphaël Mansuy. (search about Raphaël MANSUY. This project follows a Specification Driven Development approach where all changes must be specified in detail in the specs/ directory before being implemented by edgecode. edgecode is not public yet but will be soon. For now, all contributions must go through Raphaël MANSUY directly.)

Ensure we have a code of conduct file at the root of the project.

## Context

- **Location**: `docs/` directory with organized subdirectories
- **Source of Truth**: Rust crates in `edgequake/crates/`
- **Reference**: Legacy docs in `archive/docs/` (outdated but useful)
- **Web UI**: React 19 + TypeScript in `edgequake_webui/`

---

## Objectives

### 1. Quick Start & Installation

- Easy actionable path to install, compile, and use EdgeQuake
- ASCII diagrams for visual clarity
- Step-by-step instructions with verification commands

### 2. Architecture Documentation

- Module roles and responsibilities using ASCII diagrams
- Crate dependencies and interactions
- Data flow through the system

### 3. Innovation Deep-Dives

- Detailed articles on algorithms and innovations
- State machines, sequence diagrams, ERD
- First Principles Thinking approach
- References to LightRAG and other key papers
- Comparison with competing solutions
- Future improvement analysis

### 4. Code-First Approach

- Use source code as north star
- Web search for research articles and comparisons
- Always verify documentation against actual code

### 5. Continuous Improvement

- Double-check all documentation against code
- Update docs when discrepancies found
- Cross-reference related documents

---

## Process: OODA Loop (50 iterations minimum)

Execute iterative OODA cycles. Each iteration produces 4 files:

**⚠️ CRITICAL: Re-read this mission file at the START of EVERY iteration!**

Mission file: `specs/004-documentation-mission.md`

### Directory Structure

```
docs/
├── getting-started/
│   ├── installation.md
│   ├── quick-start.md
│   └── first-ingestion.md
├── architecture/
│   ├── overview.md
│   ├── crates/
│   │   ├── edgequake-core.md
│   │   ├── edgequake-llm.md
│   │   ├── edgequake-storage.md
│   │   ├── edgequake-api.md
│   │   ├── edgequake-pipeline.md
│   │   └── edgequake-query.md
│   └── data-flow.md
├── concepts/
│   ├── graph-rag.md
│   ├── entity-extraction.md
│   ├── knowledge-graph.md
│   └── hybrid-retrieval.md
├── deep-dives/
│   ├── lightrag-algorithm.md
│   ├── entity-normalization.md
│   ├── relationship-extraction.md
│   └── query-engine.md
├── comparisons/
│   ├── vs-lightrag-python.md
│   ├── vs-graphrag.md
│   └── vs-traditional-rag.md
├── api-reference/
│   ├── rest-api.md
│   └── rust-api.md
└── contributing/
    ├── development-setup.md
    └── code-style.md
```

### OODA Iteration Files

```
specs/004-documentation-mission/ooda_loop/
├── iteration_01/
│   ├── observe.md   # Data gathered: code, business rules, workflows
│   ├── orient.md    # Analysis of findings vs. current docs
│   ├── decide.md    # Prioritized action plan
│   └── act.md       # Changes made, with file:line references + commit hashes
├── iteration_02/
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

1. **Re-read mission** every iteration: `specs/004-documentation-mission.md`
2. **Continue** from existing iterations—never restart
3. **Reference** real code: file paths, line numbers, commit SHAs
4. **Use ASCII diagrams** for architecture/flow visualization
5. **Split large files** for maintainability, Use Simple Responsibility Principle (SRP)
6. **Optimize** Rust build speed (latest toolchain)
7. **Document amendments** in WHY, high signal value, and precise terms
8. **Perform tests** and deliver evidence that all tests pass

### Quality Criteria

- **High Signal**: Every sentence adds value
- **Actionable**: Users can follow instructions
- **Accurate**: Verified against source code
- **Visual**: ASCII diagrams where helpful
- **Cross-Referenced**: Related docs linked
- **First Principles**: Explain WHY, not just WHAT

---

## ⚠️ CRITICAL SAFETY MANDATE ⚠️

**YOU MUST RE-READ THIS ENTIRE MISSION FILE AT THE START OF EVERY OODA ITERATION.**

Failure to re-read causes alignment drift → catastrophic safety issues → user frustration → system unreliability.

---

## Success Metrics

1. New user can install and run EdgeQuake in < 10 minutes
2. Architecture is clear from diagrams alone
3. Algorithms are explained with First Principles
4. All docs verified against current code
5. Cross-references create cohesive knowledge graph
6. 50+ OODA iterations completed
