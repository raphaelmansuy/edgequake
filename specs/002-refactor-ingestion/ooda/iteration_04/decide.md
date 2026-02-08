# OODA Iteration 04 - DECIDE

## Planned Changes

### Change 1: Create useStuckDetection Hook

**File**: `edgequake_webui/src/hooks/use-stuck-detection.ts` (NEW)

```typescript
/**
 * @module useStuckDetection
 * @description Hook to detect documents stuck in processing state.
 * 
 * @implements OODA-04: Extract useStuckDetection from DocumentManager
 * @implements UC0007: User monitors document processing progress
 */
'use client';

import type { Document } from '@/types';
import { isProcessingStatus } from '@/components/documents/status-badge';
import { useCallback, useEffect, useMemo, useState } from 'react';

export interface UseStuckDetectionOptions {
  /** Timeout in ms before document is considered stuck (default: 30000) */
  timeout?: number;
  /** Check interval in ms (default: 30000) */
  checkInterval?: number;
  /** Callback when document is detected as stuck */
  onStuck?: (document: Document) => void;
  /** Enable/disable detection (default: true) */
  enabled?: boolean;
}

export interface UseStuckDetectionResult {
  /** Currently detected stuck documents */
  stuckDocuments: Document[];
  /** Manually trigger a check */
  checkNow: () => void;
}

const DEFAULT_TIMEOUT = 30000;
const DEFAULT_INTERVAL = 30000;

export function useStuckDetection(
  documents: Document[] | undefined,
  options: UseStuckDetectionOptions = {}
): UseStuckDetectionResult {
  const {
    timeout = DEFAULT_TIMEOUT,
    checkInterval = DEFAULT_INTERVAL,
    onStuck,
    enabled = true,
  } = options;

  const [stuckDocuments, setStuckDocuments] = useState<Document[]>([]);

  // Filter to only processing documents
  const processingDocs = useMemo(() => {
    if (!documents) return [];
    return documents.filter(
      (doc) => doc.track_id && isProcessingStatus(doc.status as any)
    );
  }, [documents]);

  // Check function
  const checkNow = useCallback(() => {
    const now = Date.now();
    const stuck: Document[] = [];

    processingDocs.forEach((doc) => {
      const updatedAt = doc.updated_at ? new Date(doc.updated_at).getTime() : 0;
      const timeSinceUpdate = now - updatedAt;

      if (timeSinceUpdate > timeout) {
        stuck.push(doc);
        console.warn('[useStuckDetection] Document may be stuck:', {
          id: doc.id,
          title: doc.title,
          status: doc.status,
          current_stage: doc.current_stage,
          seconds_since_update: Math.floor(timeSinceUpdate / 1000),
        });
        onStuck?.(doc);
      }
    });

    setStuckDocuments(stuck);
  }, [processingDocs, timeout, onStuck]);

  // Run detection on interval
  useEffect(() => {
    if (!enabled || processingDocs.length === 0) {
      setStuckDocuments([]);
      return;
    }

    // Check immediately
    checkNow();

    // Check on interval
    const interval = setInterval(checkNow, checkInterval);

    return () => clearInterval(interval);
  }, [enabled, processingDocs.length, checkNow, checkInterval]);

  return { stuckDocuments, checkNow };
}

export default useStuckDetection;
```

### Change 2: Update DocumentManager to Use Hook

**File**: `edgequake_webui/src/components/documents/document-manager.tsx`

**Add import**:
```typescript
import { useStuckDetection } from '@/hooks/use-stuck-detection';
```

**Replace useEffect (lines 329-365)** with:
```typescript
// OODA-04: Detect stuck documents using extracted hook
useStuckDetection(data?.items, {
  timeout: 30000,
  checkInterval: 30000,
});
```

### Verification Plan

1. **TypeScript**: `pnpm tsc --noEmit`
2. **Unit test**: Create test for useStuckDetection hook
3. **Manual test**: Upload document, verify stuck detection still works

### Rollback Plan

If issues found:
1. Revert document-manager.tsx to inline useEffect
2. Remove hooks/use-stuck-detection.ts
3. No runtime changes needed
