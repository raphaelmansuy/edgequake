# OODA Iteration 02 - ACT

## Summary

**Issue Fixed**: Critical Issue #2 - Silent WebSocket Disconnection

## Changes Made

### 1. Added `wsMaxReconnectsReached` to Ingestion Store

**File**: [use-ingestion-store.ts](../../../../edgequake_webui/src/stores/use-ingestion-store.ts#L52-L55)

Added new state field and setter:

- `wsMaxReconnectsReached: boolean` - tracks when reconnection has failed
- `setWsMaxReconnectsReached(reached: boolean)` - setter function

**WHY**: Need to track connection failure state separately from `wsConnected` to show persistent UI elements.

### 2. Toast Notifications in WebSocket Provider

**File**: [websocket-provider.tsx](../../../../edgequake_webui/src/providers/websocket-provider.tsx#L123-L165)

Added toast notifications for connection events:

| Event                       | Toast Type | Duration | Action       |
| --------------------------- | ---------- | -------- | ------------ |
| `disconnected`              | warning    | 5s       | None         |
| `max_reconnects_reached`    | error      | Infinite | Retry button |
| `connected` (after failure) | success    | 3s       | None         |

**WHY**: Users need immediate feedback when real-time updates become unavailable, and an actionable way to retry.

### 3. New ConnectionBanner Component

**File**: [connection-banner.tsx](../../../../edgequake_webui/src/components/documents/connection-banner.tsx) (NEW)

Created persistent banner component:

- Shows only when `wsMaxReconnectsReached` is true
- Displays "Connection Lost" message with description
- Includes "Retry" button to trigger reconnection
- Includes "Dismiss" button to hide
- Uses existing Alert/AlertDescription components

**WHY**: Toasts auto-dismiss; need persistent UI for prolonged disconnection.

### 4. Integrated Banner into DocumentManager

**File**: [document-manager.tsx](../../../../edgequake_webui/src/components/documents/document-manager.tsx#L1111)

- Added import for `ConnectionBanner`
- Rendered at top of content area, below header
- Shows automatically when connection fails

**WHY**: DocumentManager is the primary view where users track ingestion progress.

## Test Results

```
TypeScript compilation: ✅ No errors
ESLint: ⚠️ 269 pre-existing issues (unrelated to this change)
```

## Verification

- [x] TypeScript compiles without errors: `pnpm tsc --noEmit`
- [x] New component follows existing patterns
- [x] Uses existing UI components (Alert, Button)
- [x] Translations keys defined for i18n

## User Experience Flow

```
1. User loads documents page
   └─ WebSocket connects → Green dot in header

2. Backend goes down
   └─ WebSocket disconnects → Toast "Connection lost"
   └─ Auto-reconnect starts → Toast dismissed

3. Reconnect fails after 10 attempts
   └─ Toast "Unable to reconnect" appears (persistent)
   └─ Banner "Connection Lost" appears at top

4. User clicks "Retry" (toast or banner)
   └─ Reconnection attempt starts
   └─ If successful → Toast "Connection restored"
   └─ Banner disappears
```

## Impact

| Metric                      | Before        | After                       |
| --------------------------- | ------------- | --------------------------- |
| User notified of disconnect | ❌ No         | ✅ Toast                    |
| User notified of failure    | ❌ No         | ✅ Toast + Banner           |
| Retry available             | ❌ No         | ✅ Button in toast & banner |
| Connection state visible    | ⚠️ Subtle dot | ✅ Clear banner             |

## Next Iteration

**Issue #3**: Partial Extraction Failures Hidden (`pipeline.rs:800-850`)

- 8/10 chunks succeed → "Completed" status, but 2 chunks failed silently
- Add `partial_success` status with chunk failure visibility
