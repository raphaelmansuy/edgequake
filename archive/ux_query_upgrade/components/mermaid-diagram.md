# MermaidDiagram Component Specification

> Polished Mermaid diagram container with zoom controls, fullscreen mode, and theme-aware rendering.

## Overview

The MermaidDiagram component renders Mermaid diagram code as interactive SVG with professional styling, zoom controls, and fullscreen capability.

---

## Visual Design

### Standard View

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ ⎇ Diagram                                           [🔍+] [🔍-] [⛶]        │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│                     ┌─────────────┐                                         │
│                     │   Start     │                                         │
│                     └──────┬──────┘                                         │
│                            │                                                │
│                     ┌──────▼──────┐                                         │
│                     │   Process   │                                         │
│                     └──────┬──────┘                                         │
│                            │                                                │
│                     ┌──────▼──────┐                                         │
│                     │    End      │                                         │
│                     └─────────────┘                                         │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Loading State

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ ⎇ Diagram                                                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│                    ┌───────────────────────────┐                            │
│                    │                           │                            │
│                    │     ⟳ Rendering...        │                            │
│                    │                           │                            │
│                    └───────────────────────────┘                            │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Error State

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ ⎇ Diagram                                                     [📋 Copy]    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ⚠️ Failed to render diagram                                                │
│                                                                             │
│  Error: Parse error on line 3                                               │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │ graph TD                                                            │    │
│  │     A[Start] --> B[Process]                                         │    │
│  │     B --> C[End                   ← Missing bracket                  │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Fullscreen View

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ ⎇ Diagram                                [🔍+] [🔍-] [🔄] [✕ Close]         │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│                                                                             │
│                                                                             │
│                         (Diagram at larger scale)                           │
│                                                                             │
│                                                                             │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│ Zoom: 100%                                          Pan: Click and drag     │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Component Structure

```tsx
interface MermaidDiagramProps {
  code: string;
  theme?: 'light' | 'dark' | 'auto';
  zoomable?: boolean;
  fullscreenable?: boolean;
  className?: string;
}

function MermaidDiagram({
  code,
  theme = 'auto',
  zoomable = true,
  fullscreenable = true,
  className,
}: MermaidDiagramProps) {
  // Implementation
}
```

---

## Sections

### 1. Header Bar
- Left: Diagram icon + label
- Right: Zoom controls + Fullscreen button

**Styling:**
```css
.mermaid-header {
  @apply flex items-center justify-between 
         px-4 py-2 
         bg-muted/30 
         border-b border-border/40;
}
```

### 2. Diagram Content Area
- Centered SVG content
- Overflow auto for large diagrams
- Padding for breathing room
- Pan/zoom interaction area

**Styling:**
```css
.mermaid-content {
  @apply p-6 overflow-auto 
         min-h-[200px] 
         flex items-center justify-center;
}
```

### 3. Controls
- Zoom In (+10%)
- Zoom Out (-10%)
- Reset (100%)
- Fullscreen toggle

### 4. Loading Placeholder
- Skeleton with pulse animation
- "Rendering..." text

### 5. Error Display
- Error icon and message
- Original code block for debugging
- Copy button for error reporting

---

## States

### Idle (Before Render)
- Show loading placeholder

### Rendering
- Show spinner/pulse
- Text: "Rendering diagram..."

### Success
- Display SVG
- Enable controls

### Error
- Show error message
- Display original code
- Offer copy functionality

### Fullscreen
- Portal to overlay
- Larger zoom range
- Close button

---

## Theme Support

### Auto Detection
```tsx
const resolvedTheme = useMemo(() => {
  if (theme === 'auto') {
    return document.documentElement.classList.contains('dark') 
      ? 'dark' 
      : 'default';
  }
  return theme === 'dark' ? 'dark' : 'default';
}, [theme]);
```

### Mermaid Theme Config
```tsx
mermaid.initialize({
  startOnLoad: false,
  theme: resolvedTheme,
  themeVariables: {
    primaryColor: 'var(--primary)',
    primaryTextColor: 'var(--primary-foreground)',
    primaryBorderColor: 'var(--border)',
    lineColor: 'var(--muted-foreground)',
    secondaryColor: 'var(--secondary)',
    tertiaryColor: 'var(--muted)',
  },
  securityLevel: 'loose',
});
```

---

## Zoom & Pan

### Zoom Levels
- Min: 25%
- Default: 100%
- Max: 300%
- Step: 10%

### Zoom Controls
```tsx
const [zoom, setZoom] = useState(1);

const handleZoomIn = () => setZoom(z => Math.min(z + 0.1, 3));
const handleZoomOut = () => setZoom(z => Math.max(z - 0.1, 0.25));
const handleZoomReset = () => setZoom(1);
```

### Pan Support (Fullscreen)
```tsx
const [pan, setPan] = useState({ x: 0, y: 0 });
const [dragging, setDragging] = useState(false);
const dragStart = useRef({ x: 0, y: 0 });

const handleMouseDown = (e: React.MouseEvent) => {
  setDragging(true);
  dragStart.current = { x: e.clientX - pan.x, y: e.clientY - pan.y };
};

const handleMouseMove = (e: React.MouseEvent) => {
  if (dragging) {
    setPan({
      x: e.clientX - dragStart.current.x,
      y: e.clientY - dragStart.current.y,
    });
  }
};

const handleMouseUp = () => setDragging(false);
```

---

## Animations

### Loading Pulse
```css
@keyframes diagramPulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}

.mermaid-loading {
  animation: diagramPulse 1.5s ease-in-out infinite;
}
```

### Render Fade-In
```css
@keyframes diagramFadeIn {
  from { opacity: 0; transform: scale(0.98); }
  to { opacity: 1; transform: scale(1); }
}

.mermaid-rendered {
  animation: diagramFadeIn 0.3s ease-out;
}
```

### Fullscreen Transition
```css
.mermaid-fullscreen-overlay {
  animation: fadeIn 0.2s ease-out;
}

.mermaid-fullscreen-content {
  animation: scaleIn 0.2s ease-out;
}
```

---

## Accessibility

- Role: `img` for the diagram with aria-label
- Zoom buttons: Proper aria-labels
- Fullscreen: `role="dialog"` with aria-modal
- Escape key to close fullscreen
- Focus trap in fullscreen mode
- Announce state changes to screen readers

---

## Implementation

```tsx
import { memo, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { 
  GitBranch, 
  Maximize2, 
  Minimize2, 
  ZoomIn, 
  ZoomOut, 
  RotateCcw,
  Copy,
  AlertTriangle,
  X
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';

const MermaidDiagram = memo(function MermaidDiagram({
  code,
  theme = 'auto',
  zoomable = true,
  fullscreenable = true,
  className,
}: MermaidDiagramProps) {
  const [svg, setSvg] = useState<string>('');
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [zoom, setZoom] = useState(1);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  // Determine theme
  const resolvedTheme = useMemo(() => {
    if (theme === 'auto') {
      if (typeof document !== 'undefined') {
        return document.documentElement.classList.contains('dark') 
          ? 'dark' 
          : 'default';
      }
      return 'default';
    }
    return theme === 'dark' ? 'dark' : 'default';
  }, [theme]);

  // Render diagram
  useEffect(() => {
    let isMounted = true;
    setLoading(true);
    setError(null);

    const renderDiagram = async () => {
      try {
        const { default: mermaid } = await import('mermaid');

        mermaid.initialize({
          startOnLoad: false,
          theme: resolvedTheme,
          securityLevel: 'loose',
        });

        const id = `mermaid-${Math.random().toString(36).substr(2, 9)}`;
        const { svg: renderedSvg } = await mermaid.render(id, code);

        if (isMounted) {
          setSvg(renderedSvg);
          setError(null);
          setLoading(false);
        }
      } catch (err) {
        console.error('Mermaid render error:', err);
        if (isMounted) {
          setError(err instanceof Error ? err.message : 'Failed to render diagram');
          setLoading(false);
        }
      }
    };

    if (code) {
      renderDiagram();
    } else {
      setLoading(false);
    }

    return () => {
      isMounted = false;
    };
  }, [code, resolvedTheme]);

  // Zoom handlers
  const handleZoomIn = useCallback(() => {
    setZoom(z => Math.min(z + 0.1, 3));
  }, []);

  const handleZoomOut = useCallback(() => {
    setZoom(z => Math.max(z - 0.1, 0.25));
  }, []);

  const handleZoomReset = useCallback(() => {
    setZoom(1);
  }, []);

  // Fullscreen handlers
  const openFullscreen = useCallback(() => {
    setIsFullscreen(true);
    document.body.style.overflow = 'hidden';
  }, []);

  const closeFullscreen = useCallback(() => {
    setIsFullscreen(false);
    document.body.style.overflow = '';
    setZoom(1);
  }, []);

  // Escape key handler
  useEffect(() => {
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && isFullscreen) {
        closeFullscreen();
      }
    };
    document.addEventListener('keydown', handleEscape);
    return () => document.removeEventListener('keydown', handleEscape);
  }, [isFullscreen, closeFullscreen]);

  // Copy code handler
  const handleCopyCode = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(code);
    } catch (err) {
      console.error('Copy failed:', err);
    }
  }, [code]);

  // Render loading state
  if (loading) {
    return (
      <div className={cn(
        "mermaid-container my-6 rounded-xl overflow-hidden",
        "border border-border/60 bg-card",
        className
      )}>
        <div className="mermaid-header flex items-center gap-2 
                       px-4 py-2 bg-muted/30 border-b border-border/40">
          <GitBranch className="h-3.5 w-3.5 text-muted-foreground" />
          <span className="text-xs font-medium text-muted-foreground">
            Diagram
          </span>
        </div>
        <div className="mermaid-loading p-6 min-h-[200px] 
                       flex items-center justify-center">
          <div className="flex flex-col items-center gap-3 text-muted-foreground">
            <div className="h-8 w-8 rounded-full border-2 border-muted-foreground/20 
                           border-t-primary animate-spin" />
            <span className="text-sm">Rendering diagram...</span>
          </div>
        </div>
      </div>
    );
  }

  // Render error state
  if (error) {
    return (
      <div className={cn(
        "mermaid-container my-6 rounded-xl overflow-hidden",
        "border border-destructive/60 bg-destructive/5",
        className
      )}>
        <div className="mermaid-header flex items-center justify-between 
                       px-4 py-2 bg-destructive/10 border-b border-destructive/20">
          <div className="flex items-center gap-2">
            <AlertTriangle className="h-3.5 w-3.5 text-destructive" />
            <span className="text-xs font-medium text-destructive">
              Diagram Error
            </span>
          </div>
          <Button
            variant="ghost"
            size="sm"
            className="h-7 text-xs"
            onClick={handleCopyCode}
          >
            <Copy className="h-3 w-3 mr-1" />
            Copy Code
          </Button>
        </div>
        <div className="p-4 space-y-3">
          <p className="text-sm text-destructive">{error}</p>
          <pre className="text-xs bg-muted/50 p-3 rounded-lg overflow-x-auto">
            <code>{code}</code>
          </pre>
        </div>
      </div>
    );
  }

  // Diagram content component (shared between normal and fullscreen)
  const diagramContent = (
    <div 
      className="mermaid-content p-6 overflow-auto min-h-[200px]
                flex items-center justify-center"
      style={{
        transform: `scale(${zoom})`,
        transformOrigin: 'center center',
        transition: 'transform 0.1s ease-out',
      }}
    >
      <div 
        className="mermaid-rendered animate-in fade-in duration-300"
        dangerouslySetInnerHTML={{ __html: svg }} 
      />
    </div>
  );

  // Fullscreen overlay
  if (isFullscreen) {
    return (
      <div 
        className="mermaid-fullscreen-overlay fixed inset-0 z-50 
                  bg-background/95 backdrop-blur-sm
                  animate-in fade-in duration-200"
        role="dialog"
        aria-modal="true"
        aria-label="Diagram fullscreen view"
      >
        {/* Fullscreen header */}
        <div className="mermaid-fullscreen-header absolute top-0 left-0 right-0 
                       flex items-center justify-between 
                       px-6 py-4 bg-background/80 border-b border-border/40">
          <div className="flex items-center gap-2">
            <GitBranch className="h-4 w-4 text-muted-foreground" />
            <span className="text-sm font-medium">Diagram</span>
            <span className="text-xs text-muted-foreground">
              {Math.round(zoom * 100)}%
            </span>
          </div>
          <div className="flex items-center gap-2">
            <Button variant="ghost" size="icon" onClick={handleZoomOut}>
              <ZoomOut className="h-4 w-4" />
            </Button>
            <Button variant="ghost" size="icon" onClick={handleZoomReset}>
              <RotateCcw className="h-4 w-4" />
            </Button>
            <Button variant="ghost" size="icon" onClick={handleZoomIn}>
              <ZoomIn className="h-4 w-4" />
            </Button>
            <div className="w-px h-6 bg-border mx-2" />
            <Button variant="ghost" size="icon" onClick={closeFullscreen}>
              <X className="h-4 w-4" />
            </Button>
          </div>
        </div>

        {/* Fullscreen content */}
        <div className="absolute inset-0 top-16 overflow-auto 
                       flex items-center justify-center p-8">
          {diagramContent}
        </div>

        {/* Fullscreen footer */}
        <div className="absolute bottom-0 left-0 right-0 
                       px-6 py-2 bg-background/80 border-t border-border/40
                       text-xs text-muted-foreground text-center">
          Press Escape to close • Scroll to pan
        </div>
      </div>
    );
  }

  // Normal view
  return (
    <div 
      ref={containerRef}
      className={cn(
        "mermaid-container my-6 rounded-xl overflow-hidden",
        "border border-border/60 bg-card",
        className
      )}
    >
      {/* Header with controls */}
      <div className="mermaid-header flex items-center justify-between 
                     px-4 py-2 bg-muted/30 border-b border-border/40">
        <div className="flex items-center gap-2">
          <GitBranch className="h-3.5 w-3.5 text-muted-foreground" />
          <span className="text-xs font-medium text-muted-foreground">
            Diagram
          </span>
        </div>
        <div className="flex items-center gap-1">
          {zoomable && (
            <>
              <Button 
                variant="ghost" 
                size="icon" 
                className="h-7 w-7"
                onClick={handleZoomOut}
                aria-label="Zoom out"
              >
                <ZoomOut className="h-3.5 w-3.5" />
              </Button>
              <Button 
                variant="ghost" 
                size="icon" 
                className="h-7 w-7"
                onClick={handleZoomIn}
                aria-label="Zoom in"
              >
                <ZoomIn className="h-3.5 w-3.5" />
              </Button>
            </>
          )}
          {fullscreenable && (
            <Button 
              variant="ghost" 
              size="icon" 
              className="h-7 w-7"
              onClick={openFullscreen}
              aria-label="Fullscreen"
            >
              <Maximize2 className="h-3.5 w-3.5" />
            </Button>
          )}
        </div>
      </div>

      {/* Diagram content */}
      {diagramContent}
    </div>
  );
});

export default MermaidDiagram;
```

---

## CSS Classes Summary

```css
/* Container */
.mermaid-container {
  @apply my-6 rounded-xl overflow-hidden
         border border-border/60 bg-card;
}

/* Header */
.mermaid-header {
  @apply flex items-center justify-between 
         px-4 py-2 bg-muted/30 border-b border-border/40;
}

/* Content */
.mermaid-content {
  @apply p-6 overflow-auto min-h-[200px]
         flex items-center justify-center;
}

/* Loading */
.mermaid-loading {
  @apply animate-pulse;
}

/* Fullscreen overlay */
.mermaid-fullscreen-overlay {
  @apply fixed inset-0 z-50 
         bg-background/95 backdrop-blur-sm;
}

/* SVG styling */
.mermaid-container svg {
  max-width: 100%;
  height: auto;
}
```

---

*Last updated: December 26, 2025*
