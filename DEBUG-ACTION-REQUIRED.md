# URGENT: Dashboard Stats Debug - Action Required

**Status:** 🔴 **DEBUGGING IN PROGRESS - Need Your Help**

## What I've Found So Far

1. **Backend is Working Correctly** ✅
   - API returns correct stats for WorkspaceA: **8 entities, 6 relationships**
   - API returns correct stats for another workspace: **13 entities, 9 relationships**

2. **Dashboard Showing 0/0** ❌
   - Your screenshot shows: **0 entities, 0 relationships**
   - But backend is returning 8/6 for WorkspaceA

3. **This Means:** The problem is NOT with cache or API calls - **the data is being received but not displayed**

## What I've Added (Debug Code)

I've added extensive debugging to the Dashboard page that will show:
- When API is called
- What data is received
- What values are passed to StatsCard components

## 🚨 **WHAT YOU NEED TO DO NOW**

### Step 1: Open Browser Console

1. Open http://localhost:3000 (Dashboard page)
2. Press **F12** to open DevTools
3. Go to **Console** tab
4. **Reload the page (F5)**

### Step 2: Copy ALL Console Logs

Look for logs starting with `[Dashboard]`. Copy ALL of them and send to me.

Expected logs:
```
[Dashboard] Render: {...}
[Dashboard] Cache validation complete
[Dashboard] ========================================
[Dashboard] Fetching stats for workspace: ...
[Dashboard] RAW API Response: {...}
[Dashboard] entity_count: 8
[Dashboard] relationship_count: 6
[Dashboard] ========================================
[Dashboard] Stats state changed: {...}
[Dashboard] 🚨 STATS RECEIVED:
[Dashboard] - entity_count: 8
[Dashboard] - relationship_count: 6
[Dashboard] VALUES BEING PASSED TO STATSCARDS: {...}
```

### Step 3: Check window.__EDGEQUAKE_STATS__

In the browser console, type:
```javascript
window.__EDGEQUAKE_STATS__
```

This will show the exact stats object. Copy the output.

### Step 4: Check StatsCard Data Attributes

In the browser DevTools:
1. Go to **Elements** tab
2. Find the stats cards (they have `data-testid="stats-card"`)
3. Look for `data-value` attribute on each card
4. Tell me what values you see

### Step 5: Take a Network Tab Screenshot

1. Open **Network** tab in DevTools  
2. Reload page (F5)
3. Filter by "stats"
4. Click on the /stats API call
5. Go to **Response** tab
6. Take screenshot and send to me

## Why This is Critical

The bug is happening AFTER the API returns data but BEFORE it displays. Possible causes:
1. React Query is caching old data
2. Data transformation is zeroing out values
3. Component re-render race condition
4. Type mismatch (backend sends `entity_count`, frontend expects something else)

I need to see the console logs to identify which one.

## Quick Test Commands

You can also run these in terminal:

```bash
# 1. Check what stats the API returns for WorkspaceA
curl -s "http://localhost:8080/api/v1/workspaces/23d89fe3-e822-4c06-8f8c-82752436f7f3/stats" \
  -H "X-Tenant-ID: 00000000-0000-0000-0000-000000000002" | jq '.'

# 2. Run debug script to see all workspaces
./debug_workspace_stats.sh

# 3. Open debug HTML page
open debug-dashboard.html
```

## What I'm Waiting For

**CONSOLE LOGS FROM BROWSER** - This is the only way I can see what's happening in the React components.

Please:
1. Open Dashboard in browser
2. Open Console (F12)
3. Reload page
4. Copy **ALL** console output starting with `[Dashboard]`
5. Send it to me

---

**Current Status:** I've added debug code, now waiting for browser console logs to identify the exact point where data becomes 0.
