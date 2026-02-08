# OODA Iteration 02 - ORIENT

## Analysis of Findings

### Problem Statement

WebSocket disconnection events are logged but not shown to users, causing:

1. Confusion when real-time updates stop
2. Perceived "stuck" documents
3. No actionable feedback

### Solution Approaches

#### Option A: Toast Notifications Only

**Description**: Show toast on disconnect, reconnecting, reconnected, max_reconnects.

**Pros**:

- Low effort (~20 lines of code)
- Non-intrusive
- Uses existing infrastructure

**Cons**:

- Toasts auto-dismiss - users may miss them
- No way to manually reconnect
- Doesn't solve visibility for prolonged disconnection

**Effort**: 30 minutes

#### Option B: Persistent Banner + Toasts

**Description**:

- Toast on disconnect/reconnect events
- Persistent banner when connection lost for >10s or max_reconnects reached
- Banner includes "Retry" button

**Pros**:

- Clear visibility for prolonged issues
- Actionable (user can retry)
- Toasts for transient events

**Cons**:

- More code (~80 lines)
- Need to manage banner state

**Effort**: 2 hours

#### Option C: Automatic Polling Fallback + Notifications

**Description**:

- When WebSocket disconnects, automatically switch to polling
- Show banner "Live updates unavailable, using periodic refresh"
- Continue refreshing via REST API every 5s

**Pros**:

- Graceful degradation
- Users still get updates (just slower)
- Best UX for production environments

**Cons**:

- Most complex (~200 lines)
- Adds polling infrastructure
- Higher server load during disconnection

**Effort**: 6 hours

### Decision Matrix

| Criteria              | Weight | Option A | Option B | Option C |
| --------------------- | ------ | -------- | -------- | -------- |
| User visibility       | 40%    | 6/10     | 9/10     | 9/10     |
| Implementation effort | 30%    | 10/10    | 8/10     | 4/10     |
| Actionable feedback   | 20%    | 3/10     | 8/10     | 10/10    |
| Reliability           | 10%    | 7/10     | 7/10     | 9/10     |
| **Weighted Score**    | 100%   | **6.5**  | **8.3**  | **7.5**  |

### Recommendation

**Option B: Persistent Banner + Toasts**

**Rationale**:

1. Best balance of visibility and effort
2. Actionable - users can retry manually
3. Mission spec explicitly says "Add persistent connection status banner"
4. Can upgrade to Option C in future iteration if needed

### Implementation Strategy

1. **Add toast notifications** in `websocket-provider.tsx`:
   - On disconnect: `toast.warning("Connection lost")`
   - On max_reconnects: `toast.error("Unable to reconnect")`
   - On reconnected: `toast.success("Connection restored")`

2. **Create ConnectionBanner component**:
   - Shows when `!connected && !reconnecting` for >10s
   - Or when max_reconnects_reached
   - Includes "Retry" button that calls `connect()`
   - Includes "Dismiss" button

3. **Add to layout** (above main content, below header):
   - Conditionally render based on connection state
   - Animate in/out with transition

4. **Track max_reconnects_reached** in ingestion store:
   - New state: `wsMaxReconnectsReached: boolean`
   - Reset when manually reconnected or page refresh

### Files to Modify

| File                     | Change                                     |
| ------------------------ | ------------------------------------------ |
| `websocket-provider.tsx` | Add toast notifications                    |
| `use-ingestion-store.ts` | Add `wsMaxReconnectsReached` state         |
| `connection-banner.tsx`  | **New file** - persistent banner component |
| `document-manager.tsx`   | Import and render ConnectionBanner         |
