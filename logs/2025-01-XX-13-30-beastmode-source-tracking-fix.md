# Task Log: Source Tracking WebUI Integration

**Date**: 2025-01-XX 13:30  
**Mode**: BEAST MODE 🔥

## Actions

- Fixed `crypto.randomUUID` runtime error by creating `/lib/utils/uuid.ts` with cross-browser fallback
- Created `/lib/utils/source-mapper.ts` to convert `SourceReference[]` → `QueryContext`
- Updated backend `MessageContext` type to include full `MessageContextEntity` and `MessageContextRelationship` structures with source tracking fields
- Updated `sources_to_message_context()` in chat.rs to properly populate entity/relationship source tracking
- Wired source mapper into query-interface.tsx `case 'context':` block
- Added vitest testing framework to WebUI
- Created 13 unit tests for source mapper (all passing)
- Created E2E test file for source tracking validation
- Updated audit document with implementation status

## Decisions

- Used 3-tier fallback for UUID generation: `crypto.randomUUID` → `crypto.getRandomValues` → `Math.random`
- Extended `MessageContext` types rather than creating new ones for backward compatibility
- Kept TypeScript backward compatibility for string[] entity/relationship formats in `convertServerMessage`
- Used vitest.config.mjs (ES module) to avoid CommonJS/ESM conflicts

## Next Steps

- Run E2E tests against live backend to verify source citations display
- Consider adding source_chunk_ids to relationship SourceReference if needed
- Monitor for any edge cases with malformed relationship IDs

## Lessons/Insights

- The `crypto.randomUUID()` API only works in secure contexts (HTTPS) or Node.js - always use fallback for browser code
- When changing core types in Rust, re-export from mod.rs to avoid compilation errors
- Vitest 4.x requires ESM config format (.mjs) with modern Vite versions

## Files Modified

### Backend (Rust)

- `edgequake-core/src/types/conversation.rs` - Added `MessageContextEntity`, `MessageContextRelationship` structs
- `edgequake-core/src/types/mod.rs` - Re-exported new types
- `edgequake-api/src/handlers/chat.rs` - Updated `sources_to_message_context()` to populate source tracking

### Frontend (TypeScript/React)

- `lib/utils/uuid.ts` - NEW: Cross-browser UUID generation utility
- `lib/utils/source-mapper.ts` - NEW: Converts SourceReference[] to QueryContext
- `lib/utils/__tests__/source-mapper.test.ts` - NEW: 13 unit tests
- `components/query/query-interface.tsx` - Added import for mapper, fixed context case
- `stores/use-conversation-store.ts` - Fixed crypto.randomUUID usage
- `stores/use-query-store.ts` - Fixed crypto.randomUUID usage
- `vitest.config.mjs` - NEW: Vitest configuration
- `package.json` - Added vitest, test scripts
- `e2e/source-tracking.spec.ts` - NEW: E2E test for source citations

### Documentation

- `audit_lightrag_vs_edgequake/22-source-tracking-webui-audit.md` - Updated status to COMPLETE
