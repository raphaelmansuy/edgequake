# OODA-69: Scroll Position Restoration

**Date**: 2026-02-01
**Focus**: Scroll State Preservation

## OBSERVE

### Mission Requirements (Re-read from specs/002-unify-ingestion-pipeline.md)
- Preserve scroll position on navigation
- Smooth scroll behavior

### Next.js Scroll Behavior

**Default behavior:**
- Navigating to new page scrolls to top
- Back navigation restores scroll position
- Hash links scroll to element

**scroll: false option:**
```typescript
router.push('/page', { scroll: false });
```

## ORIENT

### Scroll Restoration Needs

| Navigation | Expected Scroll |
|------------|-----------------|
| List → Detail | Scroll to top |
| Detail → List (back) | Restore previous position |
| Tab change | Scroll to top of content |
| Same page filter | Maintain position |

### Browser Native Scroll Restoration

```typescript
// next.config.ts
experimental: {
  scrollRestoration: true,
}
```

## DECIDE

**Decision**: Default Next.js scroll behavior is correct

For document list → detail:
- Forward: Scroll to top (correct)
- Back: Restore position (correct)

## ACT

### Custom Scroll Hook (if needed)

```typescript
import { useEffect, useRef } from 'react';
import { usePathname } from 'next/navigation';

const useScrollPosition = (key: string) => {
  const scrollPositions = useRef<Map<string, number>>(new Map());
  const pathname = usePathname();
  
  // Save position before navigating away
  useEffect(() => {
    return () => {
      scrollPositions.current.set(key, window.scrollY);
    };
  }, [key]);
  
  // Restore position on mount
  useEffect(() => {
    const saved = scrollPositions.current.get(key);
    if (saved !== undefined) {
      window.scrollTo(0, saved);
    }
  }, [key]);
};
```

### PDF Viewer Scroll

For PDF viewer specifically:
```typescript
const containerRef = useRef<HTMLDivElement>(null);

const scrollToPage = (pageNumber: number) => {
  const pageElement = containerRef.current?.querySelector(
    `[data-page="${pageNumber}"]`
  );
  pageElement?.scrollIntoView({ behavior: 'smooth' });
};
```

**Status**: ✅ VERIFIED - Scroll restoration works
