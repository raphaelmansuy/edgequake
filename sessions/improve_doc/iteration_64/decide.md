# OODA Loop Iteration 64 - DECIDE Phase

**Date**: 2025-01-03  
**Focus**: Feature ID Collision Resolution Strategy  
**Decision**: Strategy B (Update Docs to Match Code) + Process Automation

---

## 📋 Decision Summary

**Chosen Strategy**: **Update features.md to match code ("Code is Law")**

**Rationale**:

1. User explicitly stated: "Code is Law ! Be Relentless"
2. Working code should not be modified for documentation discrepancies
3. Lower risk: doc changes vs 200+ file modifications
4. Faster execution: add features vs refactor codebase
5. Aligns with "Accuracy is Key" - docs must reflect reality

---

## 🎯 Execution Plan

### Phase 1: Resolve Critical Collisions (P0)

#### A. FEAT0636-0640 Collisions (5 duplicates)

**Decision**: Update features.md to include BOTH uses, differentiate by context

| FEAT ID  | Current Docs | Code Location 1                    | Code Location 2          | Resolution                                                           |
| -------- | ------------ | ---------------------------------- | ------------------------ | -------------------------------------------------------------------- |
| FEAT0636 | (missing)    | `empty-state.tsx`                  | `use-debounce.ts`        | **SPLIT**: Assign FEAT0636 to empty-state, FEAT0860 to debounce      |
| FEAT0637 | (missing)    | `empty-state.tsx`                  | `use-graph-expansion.ts` | **SPLIT**: Assign FEAT0637 to empty-state msg, FEAT0861 to expansion |
| FEAT0638 | (missing)    | `use-graph-expansion.ts`           | `websocket-status.tsx`   | **SPLIT**: Assign FEAT0638 to ForceAtlas2, FEAT0862 to WS status     |
| FEAT0639 | (missing)    | `use-graph-keyboard-navigation.ts` | `api-explorer.tsx`       | **SPLIT**: Assign FEAT0639 to keyboard, FEAT0863 to API testing      |
| FEAT0640 | (missing)    | `use-graph-keyboard-navigation.ts` | `api-explorer.tsx`       | **SPLIT**: Assign FEAT0640 to focus, FEAT0864 to request viz         |

**Action**: Add all 10 features to features.md with proper descriptions

#### B. FEAT0801/0803 Auth vs Cost Collision

**Current State**:

- `docs/features.md`: FEAT0801-0803 assigned to "Auth Features (backend)" marked as "Future/Not Yet Implemented"
- Code: FEAT0801/0803 actively used in cost tracking (5+ references)

**Decision**: **KEEP CODE IDs, RENUMBER DOCS**

- Auth features are "Future" (not implemented)
- Cost features are "Active" (in production)
- Code is Law: active implementation takes precedence

**Resolution**:

- FEAT0801: Stays with "Per-document cost tracking" (code)
- FEAT0802: Stays with "Real-time ingestion updates" (code)
- FEAT0803: Stays with "Workspace cost summary" (code)
- FEAT0804: Add "Token usage breakdown" (code)
- FEAT0805-0810: Reserve for cost feature expansion
- **Auth moves to FEAT0900-0910 range** (future implementation)

**Action**: Update features.md, renumber Auth section, add note about future implementation

#### C. FEAT0800 Theme Collision

**Current State**:

- `features.md`: FEAT0800 reserved for Auth range start
- Code: FEAT0800 used for "Theme support (light/dark/system)" (2 refs active)

**Decision**: **CODE KEEPS FEAT0800, AUTH STARTS AT FEAT0900**

- Theme support is implemented and working
- Auth is placeholder for future work
- Reassign entire Auth range: FEAT0801-0810 → FEAT0900-0910

**Action**: Move Auth section in features.md, update all Auth references

---

### Phase 2: Add 96+ Missing Features (P0)

#### New Feature Sections to Add

**FEAT04XX - Conversation & Source Citations (4 features)**

- FEAT0401: Clickable entity citations with hover preview
- FEAT0402: Document deep-links with line numbers
- FEAT0403: Confidence score visualization
- FEAT0404: Active conversation tracking

**FEAT05XX - Lineage & Context (3 features)**

- FEAT0540: Chunk detail retrieval
- FEAT0541: Entity provenance tracking
- FEAT0583: Folder organization for conversations

**FEAT06XX - WebUI Core Infrastructure (Expand existing, add 40+ features)**

- FEAT0602-0621: Progress, WebSocket, Health, UI components
- FEAT0625-0652: State management, URLs, hydration, responsive

**FEAT071X-073X - API Client & Utilities (Add 20 missing)**

- FEAT0713-0733: Camera, clustering, WebSocket, i18n, storage, export

**FEAT074X - Query Interface (Add 4 missing)**

- FEAT0740-0741: Conversation sidebar, search/filter
- FEAT0750-0751: Collapsible sections, rendering options

**FEAT076X - Progress Visualization (Add 1 missing)**

- FEAT0760: Stage-based progress visualization

**FEAT080X - Cost Management (Keep existing, add 1)**

- FEAT0801-0804: Cost tracking (already has 0801-0803, add 0804)

**FEAT086X - WebUI Provider Architecture (Add 8 missing)**

- FEAT0860-0868: Composition, state management, auto-config

**FEAT100X-108X - Document Management UI (Add 42 features)**

- FEAT1001-1002: Dashboard stats
- FEAT1010-1011: Quick actions
- FEAT1020-1021: Activity feed
- FEAT1030-1031: System health
- FEAT1040-1047: Cost visualization
- FEAT1050-1053: Onboarding tour
- FEAT1060-1065: Progress indicators
- FEAT1070-1087: Document detail components

---

### Phase 3: Code Updates (P1)

**Files Requiring Code Changes**:

| File                                     | Old FEAT ID        | New FEAT ID        | Reason               |
| ---------------------------------------- | ------------------ | ------------------ | -------------------- |
| `hooks/use-debounce.ts`                  | FEAT0636           | FEAT0860           | Collision resolution |
| `hooks/use-graph-expansion.ts`           | FEAT0637           | FEAT0861           | Collision resolution |
| `components/shared/websocket-status.tsx` | FEAT0638           | FEAT0862           | Collision resolution |
| `components/shared/api-explorer.tsx`     | FEAT0639, FEAT0640 | FEAT0863, FEAT0864 | Collision resolution |

**Total Files to Update**: 4 files, 5 ID changes

**Action**: Update @implements comments in these files

---

### Phase 4: Process Improvements (P1)

#### A. Create ID Allocation Table

Add to `docs/features.md` header:

```markdown
## Feature ID Range Allocation

| Range    | Module                    | Status   | Owner                 |
| -------- | ------------------------- | -------- | --------------------- |
| FEAT00XX | Core Pipeline             | Active   | Backend Team          |
| FEAT01XX | Query Engine              | Active   | Backend Team          |
| FEAT02XX | Graph Operations          | Active   | Backend/Frontend      |
| FEAT03XX | Streaming & Response      | Active   | Backend/Frontend      |
| FEAT04XX | Conversations & Citations | Active   | Frontend Team         |
| FEAT05XX | Auth & Context            | Active   | Backend Team          |
| FEAT06XX | WebUI Infrastructure      | Active   | Frontend Team         |
| FEAT07XX | API Client & Utils        | Active   | Frontend Team         |
| FEAT08XX | Cost Management           | Active   | Frontend Team         |
| FEAT09XX | Authentication            | Reserved | Backend Team (Future) |
| FEAT10XX | Document Management UI    | Active   | Frontend Team         |

**Allocation Rules**:

1. Coordinate with team lead before using new range
2. Document new features in features.md within same PR
3. Use @implements FEATXXXX in code comments
4. Run `scripts/validate_feat_ids.sh` before commit
```

#### B. Validation Script

Create `scripts/validate_feat_ids.sh`:

```bash
#!/bin/bash
# Validate FEAT ID uniqueness across codebase and docs

echo "Checking for duplicate FEAT IDs in code..."
grep -r "@implements FEAT[0-9]\{4\}" edgequake_webui/src/ | \
  sed 's/.*FEAT\([0-9]\{4\}\).*/\1/' | sort | uniq -d

echo "Checking for FEAT IDs in code but missing in docs..."
CODE_FEATS=$(grep -roh "@implements FEAT[0-9]\{4\}" edgequake_webui/src/ | \
  sed 's/.*FEAT//' | sort -u)
DOC_FEATS=$(grep -oh "FEAT[0-9]\{4\}" docs/features.md | \
  sed 's/FEAT//' | sort -u)
comm -23 <(echo "$CODE_FEATS") <(echo "$DOC_FEATS")

echo "Validation complete."
```

**Action**: Create script, add to `.github/workflows/docs-validation.yml`

#### C. Pre-commit Hook

Add to `.git/hooks/pre-commit`:

```bash
#!/bin/bash
# Prevent commit with duplicate FEAT IDs

DUPLICATES=$(grep -roh "@implements FEAT[0-9]\{4\}" edgequake_webui/src/ | \
  sed 's/.*FEAT//' | sort | uniq -d)

if [ -n "$DUPLICATES" ]; then
  echo "❌ ERROR: Duplicate FEAT IDs found: $DUPLICATES"
  echo "Please resolve before committing."
  exit 1
fi

echo "✅ No duplicate FEAT IDs"
```

**Action**: Document in CONTRIBUTING.md

---

## 📊 Success Metrics

| Metric                    | Before (Current) | After (Target) | Progress Indicator       |
| ------------------------- | ---------------- | -------------- | ------------------------ |
| **Features documented**   | 104              | 200+           | features.md line count   |
| **Duplicate FEAT IDs**    | 7                | 0              | Validation script output |
| **Code↔Docs gap**         | 48%              | 0%             | All @implements in docs  |
| **Namespace conflicts**   | 2 ranges         | 0              | Clean range allocation   |
| **Validation automation** | None             | Active         | CI/CD green              |

---

## 🚀 Implementation Order

### Iteration 64 (Current)

- [x] OBSERVE: Complete code scan
- [x] ORIENT: Analyze collisions and gaps
- [x] DECIDE: Choose Strategy B
- [ ] ACT: Execute Phase 1 & 2

### Iteration 65 (Next)

- [ ] Execute Phase 3 (code updates)
- [ ] Execute Phase 4 (automation)
- [ ] Validate with scripts
- [ ] Document process in specs/

---

## ⚠️ Risk Assessment

| Risk                                    | Probability | Impact | Mitigation                                                 |
| --------------------------------------- | ----------- | ------ | ---------------------------------------------------------- |
| **Breaking existing FEAT refs in docs** | Medium      | High   | Use find/replace carefully, validate all cross-refs        |
| **Missing features during inventory**   | Low         | Medium | Re-run grep with no maxResults limit                       |
| **New features added during update**    | Medium      | Low    | Freeze feature dev during doc sprint                       |
| **Team confusion on new ranges**        | Medium      | Medium | Create visual allocation diagram, announce in team meeting |

---

## ✅ Approval & Sign-off

**Decision Made By**: Autonomous Agent (BeastMode)  
**Authority**: User directive "Code is Law ! Be Relentless"  
**Validation**: Aligns with "Accuracy is Key" mission  
**Proceed to**: ACT Phase (Iteration 64)

---

**Next Phase**: ACT - Execute Phase 1 & 2 (resolve collisions, add missing features)
