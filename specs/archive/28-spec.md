# Mission: update workspace and document management specs

## Task

Your mission is:

- Ensure 500 workspace by tenant by default
- Ensure up to 50mb by document uploaded - Ensure it works
- Ensure I can delete a workspace 
- Ensure when a document is deleted from a workspace, all associated embeddings and knowledge graph data are also removed

## Context

- **Location**:  src/crates/

---

## Process: OODA Loop (50 iterations minimum)

Execute iterative OODA cycles. Each iteration produces 4 files:

You Must absolutely read your mission every iteration! It is vital to avoid alignment drift. You can forget previous iterations, but never forget your mission.

Mission file specs/28-spec.md


You Must always produce the 4 files per iteration, as shown below:

1 - observe.md -> Map the territor. Never make assumptions about code structure or function. Always verify against the actual codebase. When you don't know, go check the code or search on the web for answers and documentation
2 - orient.md -> Analyze your findings and define possible solutions using First Principles as your north star. Assess risks and benefits of each approach.
3 - decide.md -> Prioritize specific changes to be made based on signal value and impact.
4 - act.md -> Implement the decided changes with precision, update the documentation, and reference specific file:line numbers and commit SHAs.

````
update_edgequake/ooda_loop/
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


### Very important:

- Ensure 500 workspace by tenant by default
- Ensure up to 50mb by document uploaded - Ensure it works
- Ensure I can delete a workspace 
- Ensure when a document is deleted from a workspace, all associated embeddings and knowledge graph data are also removed


An updated and tested codebase reflecting the changes specified in the mission, with all tests passing. Ensure to provide evidence of successful test runs and e2e testing where applicable.