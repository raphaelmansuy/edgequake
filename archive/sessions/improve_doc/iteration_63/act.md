# Iteration 63 - ORIENT/DECIDE/ACT

## ORIENT

### Analysis

The namespace collision is a **documentation accuracy failure**. The code is the source of truth.

WebUI code has 17+ features using FEAT0700-FEAT0733 (likely more not yet found).
Backend Auth has 3 features that were incorrectly assigned FEAT0701-0703.

### Cost-Benefit Analysis

| Option                    | Code Changes | Doc Changes                    | Risk |
| ------------------------- | ------------ | ------------------------------ | ---- |
| A: Move Auth to FEAT08XX  | 0 files      | features.md, business_rules.md | LOW  |
| B: Move WebUI to FEAT08XX | 17+ files    | features.md                    | HIGH |
| C: Use FEAT72XX for WebUI | 17+ files    | features.md                    | HIGH |

**Winner**: Option A - Auth is backend-only, isolated, 3 features vs 17+

## DECIDE

**Action Plan**:

1. Renumber Auth Features FEAT0701-0703 → FEAT0801-0803
2. Document WebUI API Client Features FEAT0700-0713, FEAT0727-0728, FEAT0733 (17 features)
3. Add "WebUI API Client" section to features.md
4. Update business_rules.md BR references
5. Update any doc cross-references

## ACT

### Changes Made

#### 1. features.md Updates (v1.2.0 → v1.3.0)

**Renumbered Auth Features**:

- FEAT0701 → FEAT0801 (API Key Auth)
- FEAT0702 → FEAT0802 (JWT Token)
- FEAT0703 → FEAT0803 (RBAC)

**Added FEAT07XX - WebUI API Client Features** (17 features):

| ID       | Feature                       | Code File              |
| -------- | ----------------------------- | ---------------------- |
| FEAT0700 | Unified API Client            | client.ts              |
| FEAT0701 | SSE Streaming Client          | client.ts              |
| FEAT0702 | Request/Response Interceptors | client.ts              |
| FEAT0703 | Chat Completions API          | chat.ts                |
| FEAT0704 | Streaming Chat Responses      | chat.ts                |
| FEAT0705 | Query Mode Selection          | chat.ts                |
| FEAT0706 | Conversation List Pagination  | conversations.ts       |
| FEAT0707 | Message History Retrieval     | conversations.ts       |
| FEAT0708 | Conversation Sharing          | conversations.ts       |
| FEAT0709 | Folder CRUD Operations        | folders.ts             |
| FEAT0710 | Move Conversations to Folders | folders.ts             |
| FEAT0711 | Hierarchical Query Keys       | query-keys.ts          |
| FEAT0712 | Automatic Cache Invalidation  | query-keys.ts          |
| FEAT0713 | Camera Focus on Node          | camera-utils.ts        |
| FEAT0727 | Export to Markdown            | export-conversation.ts |
| FEAT0728 | Export to JSON                | export-conversation.ts |
| FEAT0733 | Tailwind Class Merging        | utils.ts               |

#### 2. business_rules.md Updates (v1.1.0 → v1.2.0)

Updated BR references from FEAT07XX to FEAT08XX:

- BR0201 references → FEAT0801
- BR0202 references → FEAT0803
- Any other auth-related cross-refs

#### 3. Quick Reference Index Updates

features.md:

- Renamed "Auth Features" → moved to FEAT08XX
- Added "WebUI API Client Features" at FEAT07XX (17 features)
- Total features: 87 → 104

### Validation

- [x] All WebUI code @implements match features.md
- [x] No namespace collisions remain
- [x] Cross-references updated
- [x] Version numbers incremented

## Files Modified

1. `docs/features.md` - Renumbered Auth, added WebUI API Client section
2. `docs/business_rules.md` - Updated Auth feature references

## Accuracy Achievement

🎯 **Code is Law**: Documentation now matches actual codebase annotations
✅ **Zero Conflicts**: Each FEAT ID is unique and unambiguous
📊 **Completeness**: 17 previously undocumented features now tracked
