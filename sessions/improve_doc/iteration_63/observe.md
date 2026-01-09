# Iteration 63 - OBSERVE Phase

## Critical Accuracy Issue Discovered

### 1. FEAT Namespace Collision (FEAT07XX)

**Problem**: WebUI code uses FEAT0700-FEAT0733 but features.md has FEAT07XX = "Auth Features"

#### Evidence from Code

WebUI files with @implements annotations:

| File                         | FEAT Reference | Description                               |
| ---------------------------- | -------------- | ----------------------------------------- |
| `lib/api/client.ts`          | FEAT0700       | Unified API client                        |
| `lib/api/client.ts`          | FEAT0701       | SSE streaming client ⚠️ CONFLICT          |
| `lib/api/client.ts`          | FEAT0702       | Request/response interceptors ⚠️ CONFLICT |
| `lib/api/chat.ts`            | FEAT0703       | Chat completions API ⚠️ CONFLICT          |
| `lib/api/chat.ts`            | FEAT0704       | Streaming chat responses                  |
| `lib/api/chat.ts`            | FEAT0705       | Query mode selection                      |
| `lib/api/conversations.ts`   | FEAT0706       | Conversation list                         |
| `lib/api/conversations.ts`   | FEAT0707       | Message history                           |
| `lib/api/conversations.ts`   | FEAT0708       | Conversation sharing                      |
| `lib/api/folders.ts`         | FEAT0709       | Folder CRUD                               |
| `lib/api/folders.ts`         | FEAT0710       | Move to folders                           |
| `lib/api/query-keys.ts`      | FEAT0711       | Hierarchical query keys                   |
| `lib/api/query-keys.ts`      | FEAT0712       | Cache invalidation                        |
| `lib/graph/camera-utils.ts`  | FEAT0713       | Camera focus                              |
| `lib/export-conversation.ts` | FEAT0727       | Export to Markdown                        |
| `lib/export-conversation.ts` | FEAT0728       | Export to JSON                            |
| `lib/utils.ts`               | FEAT0733       | Tailwind class merging                    |

#### Evidence from features.md

Current FEAT07XX in features.md (Auth Features):

| ID       | Name                      | Module                    |
| -------- | ------------------------- | ------------------------- |
| FEAT0701 | API Key Authentication    | edgequake-auth ⚠️ BACKEND |
| FEAT0702 | JWT Token Support         | edgequake-auth ⚠️ BACKEND |
| FEAT0703 | Role-Based Access Control | edgequake-auth ⚠️ BACKEND |

### 2. Impact Assessment

**Severity**: 🔴 CRITICAL - Documentation does not match code

**Conflicts**:

- FEAT0701: Backend Auth vs WebUI SSE client
- FEAT0702: Backend JWT vs WebUI interceptors
- FEAT0703: Backend RBAC vs WebUI chat API

**Affected**: 17 WebUI features incorrectly documented or missing

### 3. Root Cause

- WebUI development used FEAT07XX for API client library
- Backend used FEAT07XX for authentication
- No namespace coordination between teams
- features.md reflects backend view only

## Decision Required

Need to resolve namespace collision. Options:

**Option A**: Move Auth to FEAT08XX, keep WebUI at FEAT07XX (matches code)
**Option B**: Move WebUI API features to FEAT08XX, update code annotations
**Option C**: Split into FEAT07XX (Backend Auth) and FEAT72XX (WebUI API Client)

Recommendation: **Option A** - Minimize code changes, Auth is only 3 features vs 17+ WebUI features
