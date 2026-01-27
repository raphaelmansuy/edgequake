# Mission: Study document add/delete process on EdgeQuake

## Task

Your mission is study the document add/delete process on EdgeQuake, identify any gaps or inefficiencies, and propose improvements to enhance reliability and performance. You will document your findings and recommendations in a structured format.

- What happens when a document is added? KG and embeddings creation?
- What happens when a document is deleted? KG and embeddings deletion?
- What are the current limitations or issues with the existing process?
- What improvements can be made to ensure data integrity and performance?
- What is the impact of the KG --> add node/edge, delete node/edge?, update node/edge?, relationships, etc. Pruning? Reference counting to avoid dangling nodes/edges? Reference tracking to avoid deleting shared nodes/edges? Reference tracking to avoid deleting shared embeddings if applicables ? Reference tracking to avoid deleting shared chunks if applicable? Reference tracking on relationships on KG?

How to ensure that all edge cases are handled correctly when adding or deleting documents? How to ensure no dangling data remains, no shared data is deleted? How to ensure perfect safety and reliability when deleting documents under all circumstances? How to ensure lineage and provenance of data is maintained correctly? Important to ensure data integrity.

Harden and optimize the document add/delete process for performance, reliability, and data integrity.

Harden and optimize the knowledge graph operations for performance, reliability, and data integrity when adding or deleting documents.

Harden and optimize the embedding storage operations for performance, reliability, and data integrity when adding or deleting documents.

- Ensure metric likes number of Entities, Relationships, Embeddings per document, Relaltions, Entity Types are tracked and logged in specific database table and integrated in edgequake web ui. 

We want to monitor Documents numbers, Entities numbers, Relationships numbers, Embeddings numbers per workspace and per tenant over time.

Ensure the Postgres schema  is updated accordingly to support these metrics. Ensure the initialization scripts are updated accordingly and are indopendent of the existing schema.
Ensure to have function that verify the integrity of schema agains the version of edgequake running.

Ensure first perfect safety and reliability when deleting documents under all circumstances. Then optimize for performance query / insertion / deletion.


Ensure the comments in code is high signal value, precise, and documents the WHY behind decisions. Use ASCII diagrams where applicable.

Impact of reprocessing a document must be fully studied, and handled correctly. Ensure we have envisaged all edge cases and handled them correctly. Ensure we have full test coverage of all edge cases for chunk reprocessing, KG reprocessing, embedding reprocessing. What happens when reprocessing a document that was partially processed, failed processing, or is in the middle of processing? Ensure no dangling data remains, no shared data is deleted. Reference counting/tracking must be implemented where applicable.


Ensure e2e test are conducted also with ollama provider with real llm such as gemma3:latest  and embeddinggemma:latest for embeddings. Ensure the create/update/delete and query document process is fully reliable with ollama provider. Ensure all mode of query works as expected: LLM-only, embedding-only, hybrid.


Additions:

Ensure we monitor the sizeof the workspace. Size of the Knowledge Graph, size of the embeddings storage, size of the KV storage per workspace over time. By Workspace, by Tenant.

Ensure we have a reprocessing mechanism for failed documents. Ensure deleting a failed document cleans up all partial data.

Ensure deleting a document being processed cleans up all partial data and does not interfere with ongoing processing.

## Context

- **Location**:  edgequake/

---

## Process: OODA Loop (50 iterations minimum)

Execute iterative OODA cycles. Each iteration produces 4 files:

You Must absolutely read your mission every iteration! It is vital to avoid alignment drift. You can forget previous iterations, but never forget your mission.

Mission file specs/033-study-delete-document/003-study-document.md 


You Must always produce the 4 files per iteration, as shown below:

1 - observe.md -> Map the territor. Never make assumptions about code structure or function. Always verify against the actual codebase. When you don't know, go check the code or search on the web for answers and documentation
2 - orient.md -> Analyze your findings and define possible solutions using First Principles as your north star. Assess risks and benefits of each approach.
3 - decide.md -> Prioritize specific changes to be made based on signal value and impact.
4 - act.md -> Implement the decided changes with precision, update the documentation, and reference specific file:line numbers and commit SHAs.

````
specs/033-study-delete-document/ooda_loop/
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

1. **Re-read mission** every iteration: mission file {Spec_file_path}
2. **Continue** from existing iterations—never restart
3. **Reference** real code: file paths, line numbers, commit SHAs
4. **Use ASCII diagrams** for architecture/flow visualization
5. **Split large files** for maintainability, Use Simple Responsability Principle (SRP)
6. **Optimize** Rust build speed (latest toolchain)
7. **Document amendments** in WHY, high signal value, and precise terms in comments in the codebase. Use ASCII diagrams where applicable.
8. **You must perform tests** and deliver evidence that all tests are passing after your changes.

YOU Must Read your mission every iteration! It is vital to avoid alignment drift. You can forget previous iterations, but never forget your mission.

You must always map the territory you are documenting. Never make assumptions about code structure or function. Always verify against the actual codebase.

If you don't know make a search on the Web.

Always use First Principle Thinking as your north star.

### Deliverables

A set of high signal consolidated documents in specs/033-study-delete-document/docs updated after each iteration, culminating in a comprehensive summary.md that captures all insights and changes made. Use ASCII diagrams where applicable.


Read the mission every iteration! It is vital to avoid alignment drift. You can forget previous iterations, but never forget your mission.


### Very important:

Check the codebase, do not make assumptions. Make deep code analysis. Search on the web if you don't know something. Always use First Principle Thinking as your north star to decide the best approach.

Safety, accuracy, recall, precision reliability are your top priorities for edgequake.


- Update edgequake code to implement the decided changes. Ensure to have comprehensive test coverage for all modifications. Comprehensive Edge cases must implemented in tests to ensure reliability. This critical to ensure data integrity and performance.


- You must ensure and prove perfect safety when deleting documents that are partially processed, in the middle of processing, or failed processing. No dangling data must remain. No shared data must be deleted. Reference counting/tracking must be implemented where applicable. You must ensure perfect safety and reliability when deleting documents under all circumstances.

Ensure it working with postgres provider and memory provider for all storage layers (KV, Vector, Graph).

Ensure there is reprocessing mechanism for failed documents. Ensure deleting a failed document cleans up all partial data.

Ensure deleting a document being processed cleans up all partial data and does not interfere with ongoing processing.

You must also test in depth the query process after document deletion to ensure no dangling references or errors occur.

You must deliver evidence that all tests are passing after your changes.


You must deeply study the consequence of deleting documents with shared concepts and relationships in the KG. You must implement reference tracking/counting to avoid deleting shared nodes/edges/relationships. You must implement reference tracking/counting to avoid deleting shared embeddings if applicable. You must implement reference tracking/counting to avoid deleting shared chunks if applicable.

Commit once you believe  you have a stable and reliable implementation of the decided changes.

Amend the mission if you think is necessary: if you have suggestion to improve the create/update /delete document process, do it. But always re-read the mission every iteration to avoid alignment drift.