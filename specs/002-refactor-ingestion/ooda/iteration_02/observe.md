# OODA Iteration 02 - OBSERVE

## Issue

**Critical Issue #2**: Silent WebSocket Disconnection (`websocket-provider.tsx:130-140`)

From mission spec:
> Disconnects logged but not surfaced to user. Add persistent connection status banner.

## Data Gathered

### Current WebSocket Architecture

```
┌─────────────────────┐      ┌─────────────────────────┐      ┌───────────────────┐
│ websocket-provider  │──────│ progress-websocket.ts   │──────│ Backend WebSocket │
│ (React Context)     │      │ (ProgressWebSocket)     │      │ /ws/progress      │
└─────────────────────┘      └─────────────────────────┘      └───────────────────┘
        │                              │
        │                              └─── max_reconnects_reached event
        │                                   (console.warn → NO USER NOTIFICATION)
        │
        └─── Sets wsConnected, wsReconnecting in ingestion-store
             (used by ConnectionStatus component)
```

### File Analysis

| File | Lines | Purpose |
|------|-------|---------|
| [websocket-provider.tsx](../../../../edgequake_webui/src/providers/websocket-provider.tsx) | 248 | React context for WebSocket |
| [progress-websocket.ts](../../../../edgequake_webui/src/lib/websocket/progress-websocket.ts) | 346 | WebSocket client class |
| [connection-status.tsx](../../../../edgequake_webui/src/components/documents/connection-status.tsx) | 205 | Status badge component |
| [document-manager.tsx](../../../../edgequake_webui/src/components/documents/document-manager.tsx#L1121) | 1822 | Uses ConnectionStatus (compact) |

### Current Disconnection Handling

**websocket-provider.tsx:140-145**:
```tsx
const unsubMaxReconnects = client.on('max_reconnects_reached', () => {
  reconnectingRef.current = false;
  setWsReconnecting(false);
  console.warn('[WebSocketProvider] Max reconnection attempts reached');
});
```

**Problems**:
1. Only `console.warn` - users never see this
2. `connected` becomes false but no notification
3. `ConnectionStatus` shows grey dot - very subtle
4. No toast/banner explaining impact

### progress-websocket.ts Configuration

```typescript
this.options = {
  reconnectInterval: 3000,     // 3 seconds base
  maxReconnectAttempts: 10,    // After 10 attempts = ~30s total
  heartbeatInterval: 30000,    // 30 seconds heartbeat
  ...options,
};
```

**Exponential Backoff**: `delay = 3000 * 2^(attempt-1)`
- Attempt 1: 3s
- Attempt 2: 6s
- Attempt 3: 12s
- ...
- Attempt 10: 1536s (~25 min)

Wait - that's excessive. Let me check the actual calculation:
```typescript
const delay = this.options.reconnectInterval * Math.pow(2, this.reconnectAttempts - 1);
```

After 10 attempts:
- Total wait: 3 + 6 + 12 + 24 + 48 + 96 + 192 + 384 + 768 + 1536 = 3069s = **51 minutes!**

This seems intentional to avoid hammering the server, but users are left in the dark for that entire time.

### ConnectionStatus Component Usage

**document-manager.tsx:1121**:
```tsx
<ConnectionStatus compact={true} />
```

The `compact` mode only shows a small dot:
- Green pulsing dot = connected
- Grey dot = disconnected  ← Very subtle!
- Amber spinning = reconnecting

Users likely don't notice the color change.

### User Experience Gap

| Event | Current Behavior | Expected Behavior |
|-------|------------------|-------------------|
| Disconnect | Grey dot, console.warn | Toast notification |
| Reconnecting | Amber dot | Optional: toast "Reconnecting..." |
| Max reconnects | Grey dot, console.warn | **Persistent banner + toast** |
| Reconnected | Green dot | Toast "Back online" |

### Existing Infrastructure

1. **Toast System**: Uses `sonner` library (already imported in websocket-provider.tsx)
2. **ConnectionStatus component**: Has full mode with text + tooltip
3. **useWebSocketStatus hook**: Exposes `connected` and `reconnecting`
4. **Translation keys**: Already defined for `connection.status.*`

## Key Observations

1. **No user notification** when WebSocket disconnects or fails to reconnect
2. **Subtle visual indicator** - small colored dot easy to miss
3. **Infrastructure exists** - sonner toasts, ConnectionStatus component
4. **Long reconnect cycle** - users wait up to 51 minutes with no feedback
5. **No "retry now" button** when max reconnects reached

## Impact Assessment

| Impact | Severity | Description |
|--------|----------|-------------|
| User confusion | High | Users think uploads stuck when actually backend disconnected |
| Lost updates | High | Real-time progress stops but users don't know |
| Wasted time | Medium | Users wait for updates that won't come |
| Support burden | Medium | Users report "stuck" documents that are actually fine |

## Root Cause

The WebSocket provider only logs disconnection events and doesn't surface them to the user via toasts or persistent UI elements.
