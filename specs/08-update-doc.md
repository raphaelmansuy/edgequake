# ⚠️ DEPRECATED - SEE v3.0

**This specification has critical flaws discovered during execution:**

1. **Incomplete Reading**: "Distributed sampling" (head/mid/tail) misses content in large files
2. **One-Directional**: Only verifies docs→code, misses undocumented features
3. **No Code Discovery**: Doesn't systematically extract features from codebase first
4. **Coverage Blind**: No mechanism to detect missing documentation

**→ USE UPDATED SPEC: `specs/08-update-doc-v3.md`**

The v3.0 spec adds:

- Phase 0: Code Discovery (build ground truth catalog)
- Bidirectional verification (docs→code AND code→docs)
- Coverage matrix to track documentation gaps
- Automated extraction patterns instead of manual sampling
- Reconciliation phase to add missing docs

---

# Original Specification (v2.0 - Flawed)

````markdown
---
title: "Documentation Sync - Agent Protocol"
description: "Rigorous process for synchronizing docs with code using evidence-based verification"
version: "1.1.0"
process_version: "2.0.0"
last_modified: "2025-12-25"
maintainers:
  - name: "Documentation Team"
    contact: "docs@edgequake.dev"
schema: "edgequake/docs/process-v2"
---

# ⚠️ CRITICAL AGENT INSTRUCTIONS

**Primary Directive:** You are a skeptical auditor. You must never assume documentation is correct. You must never assume you know the content of a file without reading it in the current session.

**The Golden Rule:** Every claim in the documentation must be backed by a specific line of code or configuration in the `docs/craftpad.md` evidence log.

---

## 1. Setup & Context

### Workspace Context

To understand the system boundaries before inventorying, visualize the architecture.

### Directory Map

| Component    | Stack    | Location            |
| :----------- | :------- | :------------------ |
| **Frontend** | [stack]  | `./edgequake_webui` |
| **Backend**  | [stack]  | `./edgequake`       |
| **Docs**     | Markdown | `./docs/`           |

### The Scratchpad (`docs/craftpad.md`)

You must create or clear `docs/craftpad.md` immediately. This file is your short-term memory and audit trail. **Do not batch updates.** Update the file after every single command or discovery.

#### Scratchpad Template

```markdown
# Documentation Sync - Working Notes

**Status**: [Phase Name] | **Last Updated**: [Timestamp]

## 1. File Inventory & Read Proof

| File            | Lines | Read Status | Verification Hash (Head+Mid+Tail) |
| :-------------- | :---- | :---------- | :-------------------------------- |
| `docs/intro.md` | 50    | ✅ Full     | [Snippet of content]              |

## 2. Findings Log

| Doc ID | Claim               | Source of Truth (File:Line) | Status      | Action     |
| :----- | :------------------ | :-------------------------- | :---------- | :--------- |
| `F01`  | "Port is 8080"      | `config.rs:12`              | ✅ Verified | None       |
| `F02`  | "Endpoint /v1/user" | `routes.rs`                 | ❌ Mismatch | Update Doc |

## 3. Ambiguities & Blockers

- [ ] [Topic]: [Question]
```
````

---

## 2. The "Full-Read" Protocol (Mandatory)

**Constraint:** You cannot verify what you have not read. For **EVERY** documentation file you process, you must execute the following sequence. Failure to do so is a process violation.

1. **Count:** Run `wc -l docs/[filename].md`.
2. **Read:**

- _If < 300 lines:_ Read the full file.
- _If > 300 lines (Distributed Sampling):_
- Read lines 1-50 (`head -50`).
- Read middle 50 lines (`sed`).
- Read last 50 lines (`tail -50`).
- **Crucial:** Scan for all headers (`grep "^##"`) to ensure no sections are skipped.

3. **Log:** Record the file name, line count, and a brief snippet of the content in `docs/craftpad.md` to prove you read it.

---

## 3. Execution Phases

### Phase 1: Inventory & Mapping

**Goal:** Establish the scope and link docs to code.

1. List all files in `docs/`.
2. **Execute the "Full-Read" Protocol** for every file.
3. Map each doc to its source component.

- _Example:_ `docs/api.md` -> `edgequake/backend/api/`

4. **Gate Check:** Do not proceed until `docs/craftpad.md` lists every doc file with a corresponding line count and read-proof.

### Phase 2: Analysis & Fact Extraction

**Goal:** Extract testable claims from the documentation.

1. Select a document.
2. Extract **"Facts"** (verifiable assertions). A Fact is:

- **Endpoint:** URL paths, methods (GET/POST).
- **Config:** Env vars, default values, flags.
- **Logic:** Algorithms, data states, permissions.

3. Record these Facts in the `docs/craftpad.md` Findings Log.
4. **Gate Check:** Every document must have at least one extracted Fact or be marked as "Purely Informational" (e.g., philosophy/intro).

### Phase 3: Verification (The "Truth" Loop)

**Goal:** Validate Facts against the Codebase.

For every Fact in your Findings Log:

1. **Search:** Use `grep`, `find`, or `@workspace` search to find the authoritative code.

- _Tip:_ Search for unique strings (variable names, exact error messages, route paths).

2. **Compare:**

- **Verified:** Code matches Doc exactly.
- **Mismatch:** Code behaves differently than Doc (e.g., Doc says "Port 80", Code says "8080").
- **Missing:** Feature exists in Doc but is deleted in Code (Candidate for Archival).
- **Gap:** Feature exists in Code but is missing in Doc.

3. **Evidence:** You must paste the `File:LineNumber` of the code evidence into the scratchpad.

### Phase 4: Updates & Archival

**Goal:** Synchronize the state.

1. **Archive:** If a document refers to code that no longer exists, move it to `docs/archive/` and add a deprecated header.
2. **Update:** Edit the documentation files to match the findings from Phase 3.

- _Style:_ Use present tense.
- _Precision:_ If the code uses a specific variable name, reference it.

3. **Validate Code Blocks:** If the doc contains code snippets or examples, you must mentally "compile" them or verify them against current syntax.

### Phase 5: Final Validation Gate

**Goal:** Ensure 100% convergence.

1. **Re-read** the modified documentation files.
2. **Check Links:** Ensure no internal links are broken.
3. **Craftpad Review:**

- Are there any "Pending" items in the Findings Log?
- Is there Code Evidence for every change made?

4. **Commit:** Create a commit message referencing the work done.

---

## 4. Troubleshooting & Edge Cases

| Issue               | Protocol                                                                                                          |
| ------------------- | ----------------------------------------------------------------------------------------------------------------- |
| **Ambiguous Code**  | If code is complex (e.g., dynamic config), do not guess. Mark as "Needs Human Review" in scratchpad.              |
| **Massive Files**   | If a file is too large for context, rely strictly on `grep` specific headers and the Distributed Sampling method. |
| **No Source Found** | If you cannot find the code for a doc claim, assume the Doc is outdated. Mark as "Potential Zombie Doc".          |

---

## 5. Completion Checklist

To declare this task done, you must affirm:

- [ ] I have generated a `docs/craftpad.md` file.
- [ ] I have executed the **Full-Read Protocol** on all docs.
- [ ] I have verified technical claims against actual code files (not just memory).
- [ ] I have updated `docs/craftpad.md` with evidence of these checks.
- [ ] I have synced the text to match the code.

**End of Process.**
