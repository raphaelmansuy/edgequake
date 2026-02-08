# OODA Iteration 05 - DECIDE

## Action Plan

1. Create `use-document-websocket.ts` hook file
2. Move WebSocket subscription logic from document-manager.tsx
3. Move progress event listener logic  
4. Export and integrate in DocumentManager
5. Verify TypeScript compiles
6. Commit changes

## Hook Implementation

```typescript
// edgequake_webui/src/hooks/use-document-websocket.ts
import { useEffect } from 'react';
import { QueryClient } from '@tanstack/react-query';
import { useWebSocket } from '@/providers/websocket-provider';
import { getWebSocketClient } from '@/lib/progress-websocket';
import type { Document } from '@/types/document';

interface UseDocumentWebSocketOptions {
  queryKey?: unknown[];
  enabled?: boolean;
}

export function useDocumentWebSocket(
  documents: Document[] | undefined,
  queryClient: QueryClient,
  options?: UseDocumentWebSocketOptions
): void {
  // ... implementation
}
```

## Expected Outcome

- 50 lines removed from document-manager.tsx
- New hook ~55 lines (with comments)
- DocumentManager calls single hook
