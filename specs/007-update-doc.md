## Critical: Session Persistence

Maintain `docs/craftpad.md` as your working scratchpad throughout this task. Update it immediately upon each discovery—do not batch updates. Your session may be interrupted or context compressed at any time.

### Scratchpad Structure
```
# EdgeQuake Documentation Sync - Working Notes

## Last Updated: [timestamp]
## Current Phase: [discovery|analysis|update|review]
## Current File: [path]

### Completed
- [file]: [status]

### Findings
#### Outdated
- [file:section]: [issue] → [correction needed]

#### Missing
- [topic]: [source file reference]

#### To Archive
- [file]: [reason]

### Pending Actions
- [ ] [action item]
```

---

## Task

Synchronize documentation in `docs/` with the current implementation of EdgeQuake WebUI and Backend.

---

## Context

| Component | Stack | Location |
|-----------|-------|----------|
| WebUI | Next.js, React, TypeScript | `./edgequake_webui/` |
| Backend | Rust | `./edgequake/` |
| Documentation | Markdown | `./docs/` |

**WebUI** provides knowledge graph exploration, document management, and query execution interfaces.

**Backend** serves as the core API handling data processing, storage, and retrieval.

---

## Process

### Phase 1: Inventory
1. List all files in `docs/`, `./edgequake/`, and `./edgequake_webui/`.
2. Map documentation files to their corresponding source components.
3. Record inventory in `craftpad.md`.

### Phase 2: Analysis
For each source file:
1. Extract: API endpoints, components, data models, configuration, dependencies.
2. Compare against corresponding documentation.
3. Log discrepancies immediately to `craftpad.md` under **Findings**.

**Outdated Criteria:**
- References removed/renamed functions, endpoints, or components
- Describes deprecated configuration options
- Shows incorrect API signatures or response formats
- Lists outdated dependencies or version requirements

### Phase 3: Archival
For documentation files meeting any criteria below:
1. Move to `docs/archive/` (create if needed).
2. Prepend to file: `<!-- Archived: [date] - Reason: [reason] -->`
3. Log action in `craftpad.md`.

**Archive Criteria:**
- Documents features no longer in codebase
- Superseded by newer documentation
- References removed components with no replacement

### Phase 4: Updates
For each documentation file requiring changes:
1. Update `craftpad.md` with current file and phase.
2. Revise outdated sections using source code as truth.
3. Add missing documentation for undocumented features.
4. Remove references to archived/deprecated items.
5. Verify code examples execute correctly against current implementation.
6. Very important: Ensure you document with high precision algorithms used in the source code.

**Style Requirements:**
- Maintain existing tone and formatting conventions
- Use present tense for current functionality
- Include source file references for technical claims

### Phase 5: Validation
1. Cross-check all updated files against `craftpad.md` findings.
2. Verify no dead links or references to archived content.
3. Confirm `craftpad.md` shows all items resolved.

### Phase 6: Commit
Commit with message format:
```
docs(edgequake): synchronize with current implementation

Updated:
- [file]: [changes]

Archived:
- [file]: [reason]

Added:
- [file]: [coverage]
```

---

## Constraints

- **Read-only** on `./edgequake/` and `./edgequake_webui/`
- Do not delete documentation files—archive instead
- Flag (don't guess) ambiguous implementation details in `craftpad.md`

---

## Completion Criteria

- [ ] All source files analyzed
- [ ] All documentation files reviewed
- [ ] Outdated files archived with reason
- [ ] Active documentation reflects current implementation
- [ ] `craftpad.md` shows no unresolved findings
- [ ] Changes committed with descriptive message
