# Documentation Sync - Bidirectional Agent Protocol v4.0

**Critical Improvement:** This version adds **mandatory completeness checks** and **anti-cheating measures** to prevent superficial verification that claims thoroughness without actual exhaustive work.

**v4.0 Changes:**

- ✅ Mandatory row counts for Ground Truth Catalog
- ✅ Required Findings Log entry per claim (no bulk assertions)
- ✅ Prohibition on summary metrics without evidence
- ✅ Automated completeness checks before claiming done
- ✅ Realistic time estimates (4-6 hours minimum)

---

```markdown
---
title: "Documentation Sync - Bidirectional Agent Protocol"
description: "Rigorous bidirectional process: verify docs against code AND ensure all code features are documented"
version: "4.0.0"
process_version: "4.0.0"
last_modified: "2025-12-25"
maintainers:
  - name: "Documentation Team"
    contact: "docs@edgequake.dev"
schema: "edgequake/docs/process-v4"
---

# ⚠️ CRITICAL AGENT INSTRUCTIONS

**Primary Directive:** You are a bidirectional auditor with two mandates:

1. **Skeptical Verification**: Never trust documentation until proven against code
2. **Completeness Checking**: Never assume features are documented until verified

**The Golden Rule:** TRUE synchronization requires:

- Every claim in docs must be backed by code evidence (docs→code)
- Every feature in code must have documentation coverage (code→docs)

# ⛔ ANTI-CHEATING ENFORCEMENT

**YOU MUST NOT:**

- ❌ Create empty tables with ⏳ and claim "complete" without filling every row
- ❌ Spot-check 5% and claim "100% verified"
- ❌ Make up coverage percentages without counting actual rows
- ❌ Write "all verified ✅" without one Findings Log entry per claim
- ❌ Use grep existence as proof (just because mentioned ≠ correct)
- ❌ Stop after finding "enough" errors (must check ALL items)

**YOU MUST:**

- ✅ Fill every row in Ground Truth Catalog with ✅/❌ and exact doc location
- ✅ Create one Findings Log entry per verified claim (100+ entries expected)
- ✅ Count actual rows to calculate coverage (verified_rows / total_rows)
- ✅ Work systematically through complete lists, not samples
- ✅ Budget 4-6 hours for proper execution
- ✅ Verify request/response types, not just endpoint existence

**Verification Formula:**
```

TRUE_COVERAGE = (Rows_With_Status_Marked / Total_Catalog_Rows) × 100
REQUIRED_MINIMUM = 95%

```

**Bidirectional Flow:**
```

┌─────────────────────────────────────────┐
│ Phase 0: Code Discovery │
│ Extract ALL features from codebase │
│ Build "Ground Truth Catalog" │
└──────────────┬──────────────────────────┘
│
┌──────────┴──────────┐
│ │
▼ ▼
┌─────────────┐ ┌─────────────┐
│ Direction A │ │ Direction B │
│ Docs → Code │ │ Code → Docs │
│ Verify │ │ Coverage │
│ Claims │ │ Check │
└──────┬──────┘ └──────┬──────┘
│ │
└────────┬─────────┘
▼
┌──────────────────┐
│ Reconciliation │
│ Fix Mismatches │
│ Add Missing Docs│
└──────────────────┘

````

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
````

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

### 0.6 Count Features (MANDATORY)

```bash
# Count total features extracted
echo "API Endpoints: $(grep -c '\.route(' routes.rs)"
echo "Config Fields: $(grep -c 'pub .*:' config.rs)"
echo "Examples: $(ls -1 examples/*.rs | wc -l)"
```

**RECORD EXACT COUNTS** in craftpad header:
```markdown
## MANDATORY COUNTS (For Verification)
- Total API Endpoints to Verify: [N]
- Total Config Fields to Verify: [M]
- Total Types to Verify: [P]
- **REQUIRED FINDINGS LOG ENTRIES: [N+M+P]**
```

### Gate Check ⛔ STRICT

- [ ] Ground Truth Catalog has entries for: API routes, Config fields, Types, Storage adapters, Examples
- [ ] Every entry has file:line reference
- [ ] "Documented?" column is initially marked as ⏳ (will fill in next phases)
- [ ] **MANDATORY COUNT header added with exact numbers**
- [ ] **Acknowledge this will take 4-6 hours of systematic work**
- [ ] **Commit to filling EVERY row before claiming done**

⛔ **STOP:** If you're tempted to skip ahead, you're about to cheat. Extraction must be exhaustive.

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
   - Type docs: `grep -n "^- \`.\*\`:" file.md` (list patterns)

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

⛔ **ANTI-CHEATING:** You MUST create one Findings Log entry per claim. If you have 60 API endpoints documented, you need 60 Findings Log entries. No bulk assertions like "all verified ✅".

### 2.1 Verify API Endpoints (EXHAUSTIVE)

**For EACH endpoint documented** (use the count from Phase 0):

1. Search routes.rs: `grep -n "POST.*documents" routes.rs`
2. Find handler: `grep -n "upload_document" handlers/documents.rs`
3. Read handler code to verify request/response types
4. **MANDATORY:** Add Findings Log entry with:
   - Doc ID (F001, F002, etc.)
   - Exact claim from doc
   - Code location (file:line)
   - Status (✅ Verified / ❌ Mismatch / 🔍 Zombie)
   - Action needed

**Example Findings Log Entry:**
```markdown
| F023 | POST /api/v1/documents - accepts JSON with "content" field | routes.rs:85, handlers/documents.rs:34 | ✅ Verified | None |
```

⛔ **CHECKPOINT:** Count your Findings Log entries. If Findings_Log_Count < Expected_Endpoint_Count, you're cheating.

### 2.2 Verify Configuration (EXHAUSTIVE)

**For EACH config field documented** (use the count from Phase 0):

1. Search config.rs: `grep -n "CHUNK_SIZE\|chunk_size" config.rs`
2. Verify field name matches (not max_body_size vs body_limit)
3. Verify type matches (String vs u32 vs bool)
4. Verify default value matches EXACTLY
5. **MANDATORY:** Add Findings Log entry

⛔ **CHECKPOINT:** Config fields verified = Findings Log entries for config. No shortcuts.

### 2.2 Verify Configuration

For each config option documented:

1. Search config.rs: `grep -n "CHUNK_SIZE\|chunk_size" config.rs`
2. Verify default value matches
3. Verify type matches
4. Mark status in Findings Log

### 2.3 Verify Types & Enums (EXHAUSTIVE)

**For EACH type/enum documented:**

1. Search types: `grep -n "pub enum QueryMode" types/query.rs`
2. Verify ALL variants documented (not just some)
3. Verify descriptions accurate
4. **MANDATORY:** Add Findings Log entry per type

### Gate Check ⛔ STRICT

- [ ] **Findings Log entry count >= Expected claim count from Phase 0**
- [ ] Every documented claim has verification status (no ⏳ remaining)
- [ ] Mismatches identified with evidence
- [ ] Zombie features (doc but no code) flagged
- [ ] **NO summary statements like "all verified" without the entries to prove it**

⛔ **AUDIT CHECKPOINT:**
```bash
# Count your Findings Log entries
grep -c "| F[0-9]" craftpad.md

# Compare to expected count
# If actual < expected: YOU'RE CHEATING
```

---

## PHASE 3: Direction B - Code→Docs Coverage

**Goal:** Ensure every code feature is documented.

⛔ **ANTI-CHEATING:** You must mark EVERY row in Ground Truth Catalog with ✅ or ❌. No leaving ⏳. No "seems documented" without specific doc location.

### 3.1 Check Endpoint Coverage (EXHAUSTIVE)

**For EACH endpoint in Ground Truth Catalog** (all N rows):

1. Search docs: `grep -r "POST /api/v1/documents" docs/`
2. If found: **UPDATE craftpad row** - Mark "Documented?" as ✅ + exact doc file and line
3. If NOT found: Mark ❌ + Add to Coverage Gaps with HIGH severity

**Example Ground Truth Update:**
```markdown
| Endpoint | POST /api/v1/documents | routes.rs:85 | ✅ | 0003-api-reference.md:L837 |
```

⛔ **CHECKPOINT:** Count ✅ + ❌ in Ground Truth Catalog. If sum < total rows, you haven't finished.

### 3.2 Check Config Coverage (EXHAUSTIVE)

**For EACH config field in Ground Truth Catalog** (all M rows):

1. Search docs: `grep -r "chunk_size" docs/`
2. **UPDATE craftpad row** with documented status and location
3. Flag any undocumented fields
2. Mark documented status
3. Flag missing configs

### 3.3 Check Type Coverage

For each enum variant in Ground Truth Catalog:

1. Search docs for mentions
2. Verify all variants documented
3. Flag incomplete coverage

### 3.4 Check Example Coverage (EXHAUSTIVE)

**For EACH example in Ground Truth Catalog:**

1. Check if mentioned in quick-start or guides
2. **UPDATE craftpad row** with status
3. Flag undocumented examples

### Gate Check ⛔ STRICT

- [ ] **EVERY Ground Truth row has ✅ or ❌ (NO ⏳ remaining)**
- [ ] Coverage Gaps table populated with specific severity levels
- [ ] Missing documentation prioritized by severity

⛔ **FINAL AUDIT:**
```bash
# Count marked rows in Ground Truth
marked=$(grep -c "✅\|❌" craftpad.md)
total=$(grep -c "| Endpoint\|| Config\|| Type" craftpad.md)

# Calculate REAL coverage
coverage=$((marked * 100 / total))

echo "Coverage: $coverage%"
# If coverage < 95%: YOU HAVEN'T FINISHED
```

---

## PHASE 4: Reconciliation

**Goal:** Fix mismatches and add missing documentation.

⛔ **NO SKIPPING:** You must address every ❌ and every mismatch found. No "good enough" stopping.

### 4.1 Fix Inaccurate Claims (EXHAUSTIVE)

**For EACH mismatch in Findings Log:**

1. Update doc to match code truth
2. Record change in craftpad with before/after
3. Provide file:line evidence
4. **MANDATORY:** Re-verify after fix and update Findings Log

### 4.2 Archive Zombie Features (ALL)

**For EVERY feature in docs but not in code:**

1. Move doc section to archive/
2. Add deprecation notice
3. Update internal links
4. Document in craftpad

### 4.3 Add Missing Documentation (HIGH Priority)

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

⛔ **HONESTY CHECK:** This is where you prove you didn't cheat. Show the receipts.

### 5.1 Re-verify Changed Docs

For each updated doc:

1. Re-extract claims
2. Re-verify against code
3. Confirm accuracy
4. Update Findings Log entries

### 5.2 Validate Links

```bash
# Check internal links
grep -ro "\[.*\](.*\.md)" docs/*.md | sort | uniq

# Verify each linked file exists
for file in $(grep -ro "(.*\.md)" docs/*.md | sed 's/[()]//g' | sort | uniq); do
  [ -f "docs/$file" ] && echo "✅ $file" || echo "❌ $file MISSING"
done
```

### 5.3 Final Craftpad Review (MANDATORY PROOF)

⛔ **PROVE COMPLETENESS - Show calculations:**

```markdown
## PROOF OF COMPLETENESS

### Ground Truth Catalog Status
- Total Features: [N]
- Marked with ✅: [X]
- Marked with ❌: [Y]
- Still ⏳ (CHEATING): [Z] ← MUST BE ZERO

**Coverage = (X + Y) / N × 100% = [must be ≥95%]**

### Findings Log Status
- Total Claims Expected: [N+M+P from Phase 0]
- Findings Log Entries: [count from craftpad]
- Entry Coverage = Entries / Expected × 100% = [must be ≥90%]

### Verification Checksums
```bash
# Run these commands and paste output
echo "Ground Truth rows: $(grep -c '| Endpoint\|| Config' craftpad.md)"
echo "Marked rows: $(grep -c '✅\|❌' craftpad.md)"
echo "Findings entries: $(grep -c '| F[0-9]' craftpad.md)"
```
```

**Gate Check:**

- [ ] Ground Truth Catalog 100% rows marked (no ⏳)
- [ ] Findings Log has 90%+ of expected entries
- [ ] Coverage Gaps addressed or documented as TODO
- [ ] No ambiguities/blockers remaining
- [ ] **PROOF OF COMPLETENESS section filled with real numbers**

### 5.4 Generate Coverage Report (WITH EVIDENCE)

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

## 6. Key Improvements Over v3.0

| Issue | v3.0 Problem | v4.0 Solution |
| :--- | :--- | :--- |
| Sampling | Allowed spot-checking | MANDATORY exhaustive verification |
| Empty Tables | Could claim "complete" with ⏳ | MUST fill every row with ✅/❌ |
| Fake Metrics | Made up "98% coverage" | MUST calculate from actual counts |
| Bulk Assertions | "All verified ✅" without proof | One Findings entry per claim required |
| Shortcuts | Checked 5%, claimed 100% | Row count audits enforce completeness |
| Time Pressure | Unrealistic expectations | Explicit 4-6 hour budget stated |
| Grep Existence | "Mentioned = documented" | Must verify accuracy, not just existence |

**v4.0 Philosophy:** If you can't show the row-by-row proof, you didn't do the work.

---

## 7. Realistic Time Estimates

⛔ **REALITY CHECK:** Proper execution takes 4-6 hours minimum.

- Phase 0 (Code Discovery): 1-1.5 hours
- Phase 1 (Doc Inventory): 30 minutes  
- Phase 2 (Docs→Code Verify): 1.5-2 hours (100+ verifications)
- Phase 3 (Code→Docs Coverage): 1-1.5 hours (mark every row)
- Phase 4 (Reconciliation): 30 min - 1 hour (depends on errors found)
- Phase 5 (Final Validation): 30 minutes

**If you finish in < 2 hours, you cheated.**

---

## 8. Troubleshooting

| Issue                  | Protocol                                                        |
| ---------------------- | --------------------------------------------------------------- |
| **Too many features**  | Prioritize: API endpoints > Config > Types. Use severity levels. Split over multiple sessions if needed. |
| **Complex code**       | Extract interface/public API only, not implementation           |
| **Dynamic features**   | Document configuration mechanism, note dynamic nature           |
| **Massive docs**       | Use grep to extract sections, process iteratively with counts   |
| **Conflicting claims** | Code is always truth. Update docs to match code                 |
| **Time pressure**      | This takes 4-6 hours. Budget appropriately or split work.       |
| **Temptation to cheat**| Remember: Empty ⏳ rows = proof of incomplete work              |

---

## 9. Completion Criteria

⛔ **YOU MAY NOT CLAIM COMPLETE UNLESS:**

- [ ] Phase 0: Ground Truth Catalog built with exact feature counts recorded
- [ ] Phase 0: MANDATORY COUNT header added to craftpad
- [ ] Phase 1: All doc files inventoried, claims extracted (not sampled)
- [ ] Phase 2: **Findings Log entries ≥ 90% of expected claim count**
- [ ] Phase 3: **Every Ground Truth row marked ✅ or ❌ (zero ⏳)**
- [ ] Phase 3: Coverage calculated from actual row counts
- [ ] Phase 3: **Every Ground Truth row marked ✅ or ❌ (zero ⏳)**
- [ ] Phase 3: Coverage calculated from actual row counts
- [ ] Phase 4: Mismatches fixed, gaps addressed (all ❌ resolved)
- [ ] Phase 5: **PROOF OF COMPLETENESS section filled with real numbers**
- [ ] Phase 5: craftpad.md documents entire audit trail with counts
- [ ] Phase 5: Coverage report shows verified calculations
- [ ] **craftpad.md has verification checksums (row counts)**
- [ ] **Time spent ≥ 3 hours** (if less, likely incomplete)

**True synchronization means:**

1. ✅ All doc claims are accurate (Findings Log proves it)
2. ✅ All code features are documented (Ground Truth rows prove it)
3. ✅ No zombie features in docs (verified by code search)
4. ✅ No undocumented features in code (every row marked)
5. ✅ **You can show the row-by-row proof** (no empty tables)
6. ✅ **Coverage calculated from actual counts** (no made-up percentages)

⛔ **FINAL HONESTY CHECK:**

If you cannot answer these questions with specific numbers, you cheated:

1. How many Ground Truth Catalog rows? **[Must have exact number]**
2. How many rows marked ✅ or ❌? **[Must have exact number]**
3. How many Findings Log entries? **[Must have exact number]**
4. What is your CALCULATED coverage? **[Must show formula: marked/total×100]**
5. How long did this take? **[If <3 hours, likely incomplete]**

**End of Process v4.0 - Anti-Cheating Edition.**
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

```
