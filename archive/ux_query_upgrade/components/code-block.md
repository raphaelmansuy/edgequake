# CodeBlock Component Specification

> Polished code block with line numbers, syntax highlighting, copy functionality, and collapsible long content.

## Overview

The CodeBlock component displays code snippets with professional styling, syntax highlighting, line numbers, and interactive features like copy-to-clipboard and collapse/expand for long code.

---

## Visual Design

### Standard Code Block

```
┌─────────────────────────────────────────────────────────────────┐
│ javascript                                         📋 Copy      │
├────────────────────────────────────────────────────────────────-┤
│  1 │ function greet(name) {                                     │
│  2 │   const message = `Hello, ${name}!`;                       │
│  3 │   console.log(message);                                    │
│  4 │   return message;                                          │
│  5 │ }                                                          │
└─────────────────────────────────────────────────────────────────┘
```

### Long Code Block (Collapsed)

```
┌─────────────────────────────────────────────────────────────────┐
│ python                                    25 lines   📋 Copy    │
├─────────────────────────────────────────────────────────────────┤
│  1 │ class KnowledgeGraph:                                      │
│  2 │     def __init__(self):                                    │
│  3 │         self.nodes = {}                                    │
│  4 │         self.edges = []                                    │
│  5 │                                                            │
│  6 │     def add_node(self, id, data):                          │
│  7 │         self.nodes[id] = data                              │
│  8 │                                                            │
│  9 │     def add_edge(self, source, target, type):              │
│ 10 │         self.edges.append({                                │
│    │ ────────────────────────────────────────────────────────── │
│    │  ▼ Show all 25 lines                                       │
└─────────────────────────────────────────────────────────────────┘
```

### Long Code Block (Expanded)

```
┌─────────────────────────────────────────────────────────────────┐
│ python                                    25 lines   📋 Copy    │
├─────────────────────────────────────────────────────────────────┤
│  1 │ class KnowledgeGraph:                                      │
│  2 │     def __init__(self):                                    │
│ ... (all lines visible) ...                                     │
│ 25 │ graph.visualize()                                          │
├─────────────────────────────────────────────────────────────────┤
│  ▲ Collapse                                                     │
└─────────────────────────────────────────────────────────────────┘
```

---

## Component Structure

```tsx
interface CodeBlockProps {
  code: string;
  language: string;
  showLineNumbers?: boolean;
  collapsible?: boolean;
  maxLines?: number;
  filename?: string;
  highlightLines?: number[];
  className?: string;
}

function CodeBlock({
  code,
  language,
  showLineNumbers = true,
  collapsible = true,
  maxLines = 20,
  filename,
  highlightLines = [],
  className,
}: CodeBlockProps) {
  const [expanded, setExpanded] = useState(false);
  const [copied, setCopied] = useState(false);
  
  // Implementation
}
```

---

## Sections

### 1. Header Bar
- Left: Language badge (uppercase, monospace)
- Center: Filename if provided
- Right: Line count + Copy button

**Styling:**
```css
.code-header {
  @apply flex items-center justify-between 
         px-4 py-2 
         bg-muted/50 
         border-b border-border/40;
}

.code-language {
  @apply text-xs font-medium text-muted-foreground 
         uppercase tracking-wide;
}

.code-line-count {
  @apply text-xs text-muted-foreground mr-3;
}
```

### 2. Line Numbers Gutter
- Fixed width: `3rem` (48px)
- Right-aligned numbers
- Non-selectable
- Subtle color

**Styling:**
```css
.line-numbers {
  @apply absolute left-0 top-0 w-12 
         py-4 pr-4 
         text-right select-none
         text-muted-foreground/40 
         text-xs leading-6 
         font-mono;
}
```

### 3. Code Content
- Horizontal scroll for long lines
- Proper indentation preserved
- Syntax highlighting via rehype-highlight

**Styling:**
```css
.code-content {
  @apply overflow-x-auto 
         py-4 pl-14 pr-4 
         text-sm leading-6 
         font-mono;
}
```

### 4. Collapse Toggle (Optional)
- Appears for code > maxLines
- Gradient fade effect when collapsed
- Toggle button at bottom

**Styling:**
```css
.code-collapse-button {
  @apply w-full py-2 
         bg-muted/50 
         border-t border-border/40 
         text-xs text-muted-foreground
         hover:bg-muted/70 
         transition-colors
         flex items-center justify-center gap-2;
}
```

---

## States

### Default
- Full code visible (or collapsed if long)
- Copy button idle

### Hover
- Copy button more prominent
- Language badge slightly highlighted

### Copied
- Checkmark icon instead of copy
- Green color for feedback
- Reset after 2 seconds

### Collapsed
- Show first N lines
- Gradient fade at bottom
- "Show all X lines" button

### Expanded
- All lines visible
- "Collapse" button at bottom

### Line Highlight
- Specific lines with background tint
- Used for emphasizing important code

---

## Syntax Highlighting Theme

### Light Mode
```css
/* Keywords: purple */
.hljs-keyword { color: oklch(0.55 0.2 270); }

/* Strings: green */
.hljs-string { color: oklch(0.50 0.15 145); }

/* Numbers: orange */
.hljs-number { color: oklch(0.55 0.2 45); }

/* Comments: gray */
.hljs-comment { color: oklch(0.55 0 0); font-style: italic; }

/* Functions: blue */
.hljs-function,
.hljs-title.function_ { color: oklch(0.50 0.2 250); }

/* Types/Classes: teal */
.hljs-type,
.hljs-title.class_ { color: oklch(0.50 0.15 195); }

/* Variables: default text */
.hljs-variable { color: inherit; }

/* Operators: magenta */
.hljs-operator { color: oklch(0.55 0.2 320); }
```

### Dark Mode
```css
.dark .hljs-keyword { color: oklch(0.75 0.2 270); }
.dark .hljs-string { color: oklch(0.75 0.15 145); }
.dark .hljs-number { color: oklch(0.75 0.2 45); }
.dark .hljs-comment { color: oklch(0.55 0 0); }
.dark .hljs-function,
.dark .hljs-title.function_ { color: oklch(0.75 0.2 250); }
.dark .hljs-type,
.dark .hljs-title.class_ { color: oklch(0.75 0.15 195); }
.dark .hljs-operator { color: oklch(0.75 0.2 320); }
```

---

## Animations

### Copy Feedback
```css
@keyframes copyPulse {
  0% { transform: scale(1); }
  50% { transform: scale(1.2); }
  100% { transform: scale(1); }
}

.copy-success {
  animation: copyPulse 0.3s ease-out;
}
```

### Expand/Collapse
```css
.code-content {
  transition: max-height 0.3s ease-out;
}

.code-content.collapsed {
  max-height: calc(var(--max-lines) * 1.5rem + 2rem);
  overflow: hidden;
}

.code-content.collapsed::after {
  content: '';
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  height: 4rem;
  background: linear-gradient(transparent, var(--card));
  pointer-events: none;
}
```

---

## Interactions

### Copy Button
1. Click → Copy code to clipboard
2. Icon changes to checkmark
3. Button color changes to green
4. Reset after 2 seconds

```tsx
const handleCopy = useCallback(async () => {
  try {
    await navigator.clipboard.writeText(code);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  } catch (err) {
    console.error('Copy failed:', err);
    toast.error('Failed to copy code');
  }
}, [code]);
```

### Line Number Click (Optional)
1. Click → Select that line
2. Shift+Click → Select range
3. Copy selected lines

### Collapse/Expand
1. Click toggle → Animate height change
2. Scroll into view if needed
3. Toggle button text changes

---

## Accessibility

- Role: `region` with aria-label
- Code block: Use `<pre>` and `<code>` properly
- Copy button: aria-label="Copy code"
- Keyboard: Tab to copy button, Enter to copy
- Line numbers: aria-hidden="true" (decorative)

---

## Responsive Behavior

### Desktop (≥1024px)
- Full width with line numbers
- Horizontal scroll for long lines

### Tablet (768px - 1023px)
- Slightly smaller font
- Line numbers visible

### Mobile (<768px)
- Font size: 12px
- Line numbers hidden by default (toggleable)
- Touch-friendly copy button (48px)
- Horizontal scroll with snap

---

## Implementation

```tsx
import { Check, Copy, ChevronDown, ChevronUp } from 'lucide-react';
import { memo, useCallback, useMemo, useState } from 'react';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';

const CodeBlock = memo(function CodeBlock({
  code,
  language,
  showLineNumbers = true,
  collapsible = true,
  maxLines = 20,
  filename,
  highlightLines = [],
  className,
}: CodeBlockProps) {
  const [expanded, setExpanded] = useState(false);
  const [copied, setCopied] = useState(false);
  
  const lines = useMemo(() => code.split('\n'), [code]);
  const isLong = lines.length > maxLines;
  const shouldCollapse = collapsible && isLong && !expanded;
  
  const visibleLines = shouldCollapse 
    ? lines.slice(0, maxLines) 
    : lines;

  const handleCopy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(code);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error('Copy failed:', err);
    }
  }, [code]);

  return (
    <div className={cn(
      "code-block relative my-4 rounded-xl overflow-hidden",
      "border border-border/60 bg-muted/30",
      className
    )}>
      {/* Header */}
      <div className="code-header flex items-center justify-between 
                     px-4 py-2 bg-muted/50 border-b border-border/40">
        <div className="flex items-center gap-3">
          <span className="text-xs font-medium text-muted-foreground 
                          uppercase tracking-wide">
            {language}
          </span>
          {filename && (
            <span className="text-xs text-muted-foreground/70">
              {filename}
            </span>
          )}
        </div>
        <div className="flex items-center gap-2">
          {isLong && (
            <span className="text-xs text-muted-foreground">
              {lines.length} lines
            </span>
          )}
          <Button
            variant="ghost"
            size="sm"
            className="h-7 px-2"
            onClick={handleCopy}
            aria-label="Copy code"
          >
            {copied ? (
              <Check className="h-3.5 w-3.5 text-green-500" />
            ) : (
              <Copy className="h-3.5 w-3.5" />
            )}
          </Button>
        </div>
      </div>

      {/* Code Content */}
      <div className={cn(
        "code-content relative",
        shouldCollapse && "max-h-[calc(20*1.5rem+2rem)] overflow-hidden"
      )}>
        {showLineNumbers && (
          <div className="line-numbers absolute left-0 top-0 w-12 
                         py-4 pr-4 text-right select-none
                         text-muted-foreground/40 text-xs leading-6 font-mono"
               aria-hidden="true">
            {visibleLines.map((_, i) => (
              <div key={i} className={cn(
                highlightLines.includes(i + 1) && "text-primary"
              )}>
                {i + 1}
              </div>
            ))}
          </div>
        )}
        
        <pre className={cn(
          "overflow-x-auto py-4 pr-4 text-sm leading-6",
          showLineNumbers ? "pl-14" : "pl-4"
        )}>
          <code className={`language-${language}`}>
            {visibleLines.join('\n')}
          </code>
        </pre>

        {/* Gradient fade for collapsed state */}
        {shouldCollapse && (
          <div className="absolute bottom-0 left-0 right-0 h-16 
                         bg-gradient-to-t from-muted/30 to-transparent
                         pointer-events-none" />
        )}
      </div>

      {/* Collapse Toggle */}
      {isLong && collapsible && (
        <button
          className="w-full py-2 bg-muted/50 border-t border-border/40 
                    text-xs text-muted-foreground
                    hover:bg-muted/70 transition-colors
                    flex items-center justify-center gap-2"
          onClick={() => setExpanded(!expanded)}
        >
          {expanded ? (
            <>
              <ChevronUp className="h-3.5 w-3.5" />
              Collapse
            </>
          ) : (
            <>
              <ChevronDown className="h-3.5 w-3.5" />
              Show all {lines.length} lines
            </>
          )}
        </button>
      )}
    </div>
  );
});

export default CodeBlock;
```

---

## CSS Classes Summary

```css
/* Container */
.code-block {
  @apply relative my-4 rounded-xl overflow-hidden
         border border-border/60 bg-muted/30;
}

/* Header */
.code-header {
  @apply flex items-center justify-between 
         px-4 py-2 bg-muted/50 border-b border-border/40;
}

/* Line numbers */
.line-numbers {
  @apply absolute left-0 top-0 w-12 py-4 pr-4 
         text-right select-none
         text-muted-foreground/40 text-xs leading-6 font-mono;
}

/* Code content */
.code-content {
  @apply overflow-x-auto py-4 pl-14 pr-4 text-sm leading-6;
}

/* Collapse button */
.code-collapse {
  @apply w-full py-2 bg-muted/50 border-t border-border/40 
         text-xs text-muted-foreground
         hover:bg-muted/70 transition-colors
         flex items-center justify-center gap-2;
}
```

---

*Last updated: December 26, 2025*
