# OODA Iteration 02 - DECIDE

## Planned Changes

### Change 1: Add `wsMaxReconnectsReached` to Ingestion Store

**File**: `edgequake_webui/src/stores/use-ingestion-store.ts`

**Action**: Add new state field and setter.

```typescript
// Add to state interface
wsMaxReconnectsReached: boolean;

// Add setter
setWsMaxReconnectsReached: (reached: boolean) => void;

// Initialize in create()
wsMaxReconnectsReached: false,

// Implement setter
setWsMaxReconnectsReached: (reached) => set({ wsMaxReconnectsReached: reached }),
```

### Change 2: Add Toast Notifications to WebSocket Provider

**File**: `edgequake_webui/src/providers/websocket-provider.tsx`

**Action**: Add toast calls in event handlers.

```typescript
// In 'disconnected' handler:
const unsubDisconnected = client.on("disconnected", () => {
  connectedRef.current = false;
  setWsConnected(false);
  // NEW: Notify user
  toast.warning("Connection lost", {
    description: "Attempting to reconnect...",
    duration: 5000,
  });
});

// In 'max_reconnects_reached' handler:
const unsubMaxReconnects = client.on("max_reconnects_reached", () => {
  reconnectingRef.current = false;
  setWsReconnecting(false);
  setWsMaxReconnectsReached(true); // NEW
  // NEW: Notify user
  toast.error("Unable to reconnect", {
    description: "Real-time updates unavailable. Click to retry.",
    duration: Infinity, // Persistent until dismissed
    action: {
      label: "Retry",
      onClick: () => {
        setWsMaxReconnectsReached(false);
        clientRef.current?.connect();
      },
    },
  });
});

// In 'connected' handler:
const unsubConnected = client.on("connected", () => {
  connectedRef.current = true;
  reconnectingRef.current = false;
  setWsConnected(true);
  // NEW: Only show if we were disconnected
  if (wsMaxReconnectsReached) {
    setWsMaxReconnectsReached(false);
    toast.success("Connection restored", {
      description: "Real-time updates are back online.",
      duration: 3000,
    });
  }
});
```

### Change 3: Create ConnectionBanner Component

**File**: `edgequake_webui/src/components/documents/connection-banner.tsx` (NEW)

**Purpose**: Persistent banner shown when connection is lost.

```tsx
"use client";

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { useIngestionStore } from "@/stores/use-ingestion-store";
import { useWebSocket } from "@/hooks/use-websocket";
import { AlertCircle, RefreshCw, X } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

export function ConnectionBanner() {
  const { t } = useTranslation();
  const { connect } = useWebSocket();
  const {
    wsConnected,
    wsReconnecting,
    wsMaxReconnectsReached,
    setWsMaxReconnectsReached,
  } = useIngestionStore();
  const [dismissed, setDismissed] = useState(false);

  // Only show if max reconnects reached and not dismissed
  if (!wsMaxReconnectsReached || dismissed) {
    return null;
  }

  const handleRetry = () => {
    setWsMaxReconnectsReached(false);
    connect();
  };

  const handleDismiss = () => {
    setDismissed(true);
  };

  return (
    <Alert variant="destructive" className="mb-4">
      <AlertCircle className="h-4 w-4" />
      <AlertTitle>{t("connection.banner.title", "Connection Lost")}</AlertTitle>
      <AlertDescription className="flex items-center justify-between">
        <span>
          {t(
            "connection.banner.description",
            "Real-time updates are unavailable. Document progress may be delayed.",
          )}
        </span>
        <div className="flex items-center gap-2 ml-4">
          <Button variant="outline" size="sm" onClick={handleRetry}>
            <RefreshCw className="h-4 w-4 mr-1" />
            {t("connection.banner.retry", "Retry")}
          </Button>
          <Button
            variant="ghost"
            size="icon"
            className="h-6 w-6"
            onClick={handleDismiss}
          >
            <X className="h-4 w-4" />
          </Button>
        </div>
      </AlertDescription>
    </Alert>
  );
}
```

### Change 4: Add ConnectionBanner to DocumentManager

**File**: `edgequake_webui/src/components/documents/document-manager.tsx`

**Action**: Import and render banner at top of content.

```tsx
// Add import
import { ConnectionBanner } from './connection-banner';

// In render, after "Fixed Header Zone" comment:
<div className="shrink-0 px-4 pt-4 space-y-3 bg-background">
  {/* Connection status banner */}
  <ConnectionBanner />

  {/* Header - Compact */}
  <header className="flex items-center justify-between gap-3 flex-wrap">
```

### Verification Plan

1. **Unit Test**: Mock WebSocket events and verify store state changes
2. **Manual Test**:
   - Start backend, open UI, verify connected
   - Stop backend, observe toast "Connection lost"
   - Wait for max reconnects, observe persistent toast + banner
   - Click "Retry", observe reconnection attempt
   - Start backend, observe "Connection restored" toast
   - Verify banner disappears

### Rollback Plan

If issues found:

1. Revert toast notifications in websocket-provider.tsx
2. Keep ConnectionBanner component but don't import in DocumentManager
3. Feature flag: `NEXT_PUBLIC_SHOW_CONNECTION_BANNER=false`
