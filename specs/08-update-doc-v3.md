# Documentation Sync - Bidirectional Agent Protocol v3.0

**Critical Improvement:** This version adds **code-first discovery** to ensure ALL features are documented, not just verification of existing docs.

---

```markdown
---
title: "Documentation Sync - Bidirectional Agent Protocol"
description: "Rigorous bidirectional process: verify docs against code AND ensure all code features are documented"
version: "2.0.0"
process_version: "3.0.0"
last_modified: "2025-12-25"
maintainers:
  - name: "Documentation Team"
    contact: "docs@edgequake.dev"
schema: "edgequake/docs/process-v3"
---

# ⚠️ CRITICAL AGENT INSTRUCTIONS

**Primary Directive:** You are a bidirectional auditor with two mandates:
1. **Skeptical Verification**: Never trust documentation until proven against code
2. **Completeness Checking**: Never assume features are documented until verified

**The Golden Rule:** TRUE synchronization requires:
- Every claim in docs must be backed by code evidence (docs→code)
- Every feature in code must have documentation coverage (code→docs)

**Bidirectional Flow:**
```
┌─────────────────────────────────────────┐
│   Phase 0: Code Discovery               │
│   Extract ALL features from codebase    │
│   Build "Ground Truth Catalog"          │
└──────────────┬──────────────────────────┘
               │
    ┌──────────┴──────────┐
    │                     │
    ▼                     ▼
┌─────────────┐    ┌─────────────┐
│ Direction A │    │ Direction B │
│ Docs → Code │    │ Code → Docs │
│ Verify      │    │ Coverage    │
│ Claims      │    │ Check       │
└──────┬──────┘    └──────┬──────┘
       │                  │
       └────────┬─────────┘
                ▼
      ┌──────────────────┐
      │  Reconciliation  │
      │  Fix Mismatches  │
      │  Add Missing Docs│
      └──────────────────┘
```

---

## 0. Setup & Scratchpad

### The Scratchpad (`docs/craftpad.md`)
Create or clear `docs/craftpad.md` immediately. This is your audit trail.

#### Enhanced Scratchpad Template
```markdown
# Documentation Sync - Working Notes
**Status**: [Phase] | **Last Updated**: [Timestamp]

## 0. Ground Truth Catalog (From Code)
| Category | Feature | Source (File:Line) | Documented? | Doc Location |
| :--- | :--- | :--- | :--- | :--- |
| Endpoint | `GET /api/v1/documents` | routes.rs:87 | ✅ | 0003-api-reference.md:L850 |
| Endpoint | `POST /api/v1/graph/entities/{id}` | routes.rs:125 | ❌ | MISSING |
| Config | `CHUNK_SIZE` default 1200 | config.rs:127 | ✅ | 0007-configuration.md:L45 |
| Type | `QueryMode::Hybrid` | types/query.rs:15 | ✅ | README.md:L115 |

## 1. File Inventory
| File | Lines | Read Status | Content Hash |
| :--- | :--- | :--- | :--- |
| `README.md` | 198 | ✅ Complete | "EdgeQuake Documentation" |

## 2. Findings Log (Docs→Code Verification)
| Doc ID | Claim | Source of Truth (File:Line) | Status | Action |
| :--- | :--- | :--- | :--- | :--- |
| `F01` | "Port is 8080" | `main.rs:76` | ✅ Verified | None |
| `F02` | "Endpoint /v1/user" | NOT FOUND | ❌ Zombie | Archive |

## 3. Coverage Gaps (Code→Docs Missing)
| Feature | Type | Source | Severity | Action |
| :--- | :--- | :--- | :--- | :--- |
| `POST /api/v1/pipeline/status` | Endpoint | routes.rs:150 | HIGH | Add to API ref |
| `WorkerConfig.max_retries` | Config | config.rs:45 | MEDIUM | Add to config ref |

## 4. Ambiguities & Blockers
- [ ] _None_
```

---

## PHASE 0: Code Discovery (NEW - CRITICAL)

**Goal:** Build a complete catalog of features from the codebase FIRST. This is the "ground truth".

### 0.1 Extract API Endpoints

```bash
# Find all route definitions
grep -n "\.route(" edgequake/crates/edgequake-api/src/routes.rs

# Find all handler functions
find edgequake/crates/edgequake-api/src/handlers -name "*.rs" -exec grep -n "pub async fn" {} +
```

**Record in Craftpad Table:** Every endpoint with method, path, handler, file:line

### 0.2 Extract Configuration Options

```bash
# Find all config structs
grep -n "pub struct.*Config" edgequake/crates/edgequake-core/src/config.rs

# Find all config fields
grep -n "pub .*:" edgequake/crates/edgequake-core/src/config.rs
```

**Record in Craftpad:** Every config field, type, default value, file:line

### 0.3 Extract Types & Enums

```bash
# Find all public enums
find edgequake/crates -name "*.rs" -exec grep -n "pub enum" {} +

# Find QueryMode specifically
grep -A 20 "pub enum QueryMode" edgequake/crates/edgequake-core/src/types/query.rs
```

**Record in Craftpad:** Every enum variant, purpose, file:line

### 0.4 Extract Storage Adapters

```bash
# List storage implementations
ls -la edgequake/crates/edgequake-storage/src/adapters/
```

**Record in Craftpad:** Every adapter, capabilities, file path

### 0.5 Extract Examples

```bash
# List all examples
ls -la edgequake/examples/
grep -n "fn main" edgequake/examples/*.rs
```

**Record in Craftpad:** Every example, purpose, file path

### Gate Check
- [ ] Ground Truth Catalog has entries for: API routes, Config fields, Types, Storage adapters, Examples
- [ ] Every entry has file:line reference
- [ ] "Documented?" column is initially marked as ⏳ (will fill in next phases)

---

## PHASE 1: Documentation Inventory

**Goal:** Catalog all documentation files.

### 1.1 List Documentation

```bash
cd docs && find . -name "*.md" -type f | sort
wc -l *.md
```

### 1.2 Enhanced Read Protocol

**CRITICAL CHANGE:** No more "distributed sampling". Use targeted extraction.

For each doc file:

1. **Extract All Factual Claims:**
   - API docs: `grep -n "^### (GET|POST|PUT|DELETE)" file.md`
   - Config docs: `grep -n "^[A-Z_]+=.*#" file.md` (env var patterns)
   - Type docs: `grep -n "^- \`.*\`:" file.md` (list patterns)

2. **Read Context Around Each Claim:**
   - For each extracted line, read ±20 lines of context
   - This ensures you capture the full description

3. **Log Every Claim:**
   - Add to Findings Log in craftpad

### Gate Check
- [ ] Every doc file inventoried with line count
- [ ] All factual claims extracted (not sampled)
- [ ] Ready for verification phase

---

## PHASE 2: Direction A - Docs→Code Verification

**Goal:** Verify every claim in docs is true in code.

### 2.1 Verify API Endpoints

For each endpoint documented:
1. Search routes.rs: `grep -n "POST.*documents" routes.rs`
2. Find handler: `grep -n "upload_document" handlers/documents.rs`
3. Verify request/response structs match
4. Mark status in Findings Log

### 2.2 Verify Configuration

For each config option documented:
1. Search config.rs: `grep -n "CHUNK_SIZE\|chunk_size" config.rs`
2. Verify default value matches
3. Verify type matches
4. Mark status in Findings Log

### 2.3 Verify Types & Enums

For each type/enum documented:
1. Search types: `grep -n "pub enum QueryMode" types/query.rs`
2. Verify all variants documented
3. Verify descriptions accurate
4. Mark status in Findings Log

### Gate Check
- [ ] Every documented claim has verification status
- [ ] Mismatches identified with evidence
- [ ] Zombie features (doc but no code) flagged

---

## PHASE 3: Direction B - Code→Docs Coverage

**Goal:** Ensure every code feature is documented.

### 3.1 Check Endpoint Coverage

For each endpoint in Ground Truth Catalog:
1. Search docs: `grep -r "POST /api/v1/documents" docs/`
2. If found: Mark "Documented?" as ✅ + location
3. If NOT found: Add to Coverage Gaps with HIGH severity

### 3.2 Check Config Coverage

For each config field in Ground Truth Catalog:
1. Search docs: `grep -r "CHUNK_SIZE\|chunk_size" docs/`
2. Mark documented status
3. Flag missing configs

### 3.3 Check Type Coverage

For each enum variant in Ground Truth Catalog:
1. Search docs for mentions
2. Verify all variants documented
3. Flag incomplete coverage

### 3.4 Check Example Coverage

For each example in Ground Truth Catalog:
1. Check if mentioned in quick-start or guides
2. Flag undocumented examples

### Gate Check
- [ ] Every Ground Truth feature has coverage status
- [ ] Coverage Gaps table populated
- [ ] Missing documentation prioritized by severity

---

## PHASE 4: Reconciliation

**Goal:** Fix mismatches and add missing documentation.

### 4.1 Fix Inaccurate Claims

For each mismatch in Findings Log:
1. Update doc to match code truth
2. Record change in craftpad
3. Provide file:line evidence

### 4.2 Archive Zombie Features

For features in docs but not in code:
1. Move doc section to archive/
2. Add deprecation notice
3. Update internal links

### 4.3 Add Missing Documentation

For each HIGH severity gap:
1. Create documentation section
2. Follow existing doc style
3. Include code examples
4. Add to appropriate doc file

### 4.4 Update Code References

For each code reference in docs:
1. Verify file path exists
2. Verify line numbers accurate
3. Update if drift detected

### Gate Check
- [ ] All mismatches resolved
- [ ] Zombie features archived
- [ ] HIGH severity gaps documented
- [ ] All code references valid

---

## PHASE 5: Final Validation

**Goal:** Verify 100% synchronization achieved.

### 5.1 Re-verify Changed Docs

For each updated doc:
1. Re-extract claims
2. Re-verify against code
3. Confirm accuracy

### 5.2 Validate Links

```bash
# Check internal links
grep -ro "\[.*\](.*\.md)" docs/*.md | sort | uniq

# Verify each linked file exists
for file in $(grep -ro "(.*\.md)" docs/*.md | sed 's/[()]//g' | sort | uniq); do
  [ -f "docs/$file" ] && echo "✅ $file" || echo "❌ $file MISSING"
done
```

### 5.3 Final Craftpad Review

Check craftpad completeness:
- [ ] Ground Truth Catalog 100% coverage marked
- [ ] Findings Log all verified or fixed
- [ ] Coverage Gaps addressed or documented as TODO
- [ ] No ambiguities/blockers remaining

### 5.4 Generate Coverage Report

Create summary:
```markdown
## Sync Report

**Code Features Catalogued:** [count]
**Documentation Files:** [count]
**Claims Verified:** [count]
**Inaccuracies Fixed:** [count]
**Missing Features Documented:** [count]
**Zombie Features Archived:** [count]
**Coverage:** [percentage]%

**High Priority TODOs:** [list if any remain]
```

---

## 6. Key Improvements Over v2.0

| Issue | Old Approach | New Approach |
|-------|-------------|--------------|
| Reading | Sampled 50-line chunks | Extract all factual claims, read context |
| Direction | Docs→Code only | Bidirectional (Docs→Code + Code→Docs) |
| Completeness | Assumed docs cover all | Build Ground Truth Catalog first |
| Coverage | No systematic check | Coverage matrix with gaps identified |
| Extraction | Manual reading | Automated grep patterns for facts |
| Missing Features | Not detected | Explicitly tracked and added |

---

## 7. Troubleshooting

| Issue | Protocol |
|-------|----------|
| **Too many features** | Prioritize: API endpoints > Config > Types. Use severity levels |
| **Complex code** | Extract interface/public API only, not implementation |
| **Dynamic features** | Document configuration mechanism, note dynamic nature |
| **Massive docs** | Use grep to extract sections, process iteratively |
| **Conflicting claims** | Code is always truth. Update docs to match code |

---

## 8. Completion Criteria

To declare synchronization complete:

- [ ] Phase 0: Ground Truth Catalog built with 100+ features
- [ ] Phase 1: All doc files inventoried, claims extracted
- [ ] Phase 2: Every doc claim verified (Findings Log complete)
- [ ] Phase 3: Every code feature coverage checked (Coverage Gaps identified)
- [ ] Phase 4: Mismatches fixed, gaps addressed
- [ ] Phase 5: Final validation passed, links checked
- [ ] craftpad.md documents entire audit trail
- [ ] Coverage report shows >90% documentation coverage
- [ ] High priority gaps either documented or in TODO

**True synchronization means:**
1. ✅ All doc claims are accurate
2. ✅ All code features are documented
3. ✅ No zombie features in docs
4. ✅ No undocumented features in code

**End of Process v3.0.**

```
