# EdgeQuake Query UX/UI Implementation Plan

> Detailed implementation specification for creating a slick, delightful chat-based query interface.

## Table of Contents

1. [Overview](#overview)
2. [Design Principles](#design-principles)
3. [Phase 1: Visual Polish](#phase-1-visual-polish)
4. [Phase 2: Functional Enhancements](#phase-2-functional-enhancements)
5. [Phase 3: Delight Features](#phase-3-delight-features)
6. [Component Specifications](#component-specifications)
7. [Layout Specifications](#layout-specifications)
8. [Responsive Design](#responsive-design)
9. [Animation Guidelines](#animation-guidelines)
10. [Implementation Checklist](#implementation-checklist)

---

## Overview

### Goal
Transform the EdgeQuake query interface into a **state-of-the-art chat experience** with beautiful markdown rendering, polished code blocks, elegant diagram display, and delightful micro-interactions.

### Scope
- **Primary**: Query page (`/query`) chat interface
- **Secondary**: Markdown rendering across the app
- **Tertiary**: Related UI components (history, citations)

### Success Metrics
- Visual consistency with modern AI chat interfaces
- Smooth animations (60fps)
- Accessible (WCAG AA compliance)
- Mobile-optimized (touch-friendly)
- Fast perceived performance

---

## Design Principles

### 1. Clarity Over Complexity
- Clean visual hierarchy
- Purposeful use of color
- Generous whitespace
- Clear affordances

### 2. Delight Through Details
- Smooth micro-animations
- Thoughtful feedback
- Polish in edge cases
- Consistent behavior

### 3. Content First
- Messages are the focus
- UI gets out of the way
- Easy to scan responses
- Beautiful typography

### 4. Accessible by Default
- Keyboard navigable
- Screen reader friendly
- Reduced motion support
- Sufficient contrast

---

## Phase 1: Visual Polish

### 1.1 Message Bubble Enhancement

#### User Message
```tsx
// Current
<div className="bg-primary text-primary-foreground rounded-2xl rounded-tr-sm px-4 py-3">

// Enhanced
<div className="bg-gradient-to-br from-primary to-primary/90 text-primary-foreground 
               rounded-2xl rounded-tr-sm px-4 py-3 
               shadow-[0_2px_8px_rgba(0,0,0,0.08)]
               animate-in slide-in-from-right-2 duration-200">
```

**Specifications:**
- Background: Subtle gradient for depth
- Shadow: `0 2px 8px rgba(0,0,0,0.08)` (light) / `0 2px 8px rgba(0,0,0,0.3)` (dark)
- Border radius: `1.25rem` with `0.25rem` for the tail corner
- Animation: Slide-in from right, 200ms ease-out
- Max width: 85% of container

#### Assistant Message
```tsx
// Enhanced
<div className="bg-card border border-border/60 
               rounded-2xl rounded-tl-sm px-4 py-3 
               shadow-[0_1px_4px_rgba(0,0,0,0.04)]
               animate-in slide-in-from-left-2 duration-200">
```

**Specifications:**
- Background: `var(--card)` with subtle border
- Shadow: `0 1px 4px rgba(0,0,0,0.04)` (light) / `0 1px 4px rgba(0,0,0,0.2)` (dark)
- Border: `1px solid var(--border)` at 60% opacity
- Animation: Slide-in from left, 200ms ease-out

#### Avatar Enhancement
```tsx
// Enhanced avatar with gradient ring
<Avatar className="h-9 w-9 shrink-0 ring-2 ring-primary/20 shadow-sm">
  <AvatarFallback className="bg-gradient-to-br from-primary/80 to-primary">
    <Sparkles className="h-4 w-4 text-primary-foreground" />
  </AvatarFallback>
</Avatar>
```

### 1.2 Code Block Redesign

#### New Structure
```tsx
<div className="code-block group relative my-4 rounded-xl overflow-hidden
               border border-border/60 bg-muted/30">
  {/* Header */}
  <div className="code-block-header flex items-center justify-between 
                 px-4 py-2 bg-muted/50 border-b border-border/40">
    <span className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
      {language}
    </span>
    <div className="flex items-center gap-2">
      <span className="text-xs text-muted-foreground">{lineCount} lines</span>
      <Button variant="ghost" size="sm" onClick={handleCopy}>
        {copied ? <Check className="h-3.5 w-3.5 text-green-500" /> 
                : <Copy className="h-3.5 w-3.5" />}
      </Button>
    </div>
  </div>
  
  {/* Code content with line numbers */}
  <div className="code-block-content relative">
    <div className="line-numbers absolute left-0 top-0 w-12 py-4 
                   text-right pr-4 select-none
                   text-muted-foreground/50 text-xs leading-6">
      {lines.map((_, i) => <div key={i}>{i + 1}</div>)}
    </div>
    <pre className="overflow-x-auto py-4 pl-14 pr-4 text-sm leading-6">
      <code>{children}</code>
    </pre>
  </div>
  
  {/* Collapse toggle for long code */}
  {lineCount > 20 && (
    <button className="code-block-collapse w-full py-2 bg-muted/50 
                      border-t border-border/40 text-xs text-muted-foreground
                      hover:bg-muted/70 transition-colors">
      {collapsed ? `Show all ${lineCount} lines` : 'Collapse'}
    </button>
  )}
</div>
```

#### Color Tokens (Syntax Highlighting)
```css
/* Light theme syntax colors */
.hljs-keyword { color: oklch(0.55 0.2 270); }   /* Purple */
.hljs-string { color: oklch(0.55 0.15 140); }   /* Green */
.hljs-number { color: oklch(0.55 0.2 30); }     /* Orange */
.hljs-comment { color: oklch(0.6 0 0); }        /* Gray */
.hljs-function { color: oklch(0.5 0.2 230); }   /* Blue */

/* Dark theme syntax colors */
.dark .hljs-keyword { color: oklch(0.75 0.2 270); }
.dark .hljs-string { color: oklch(0.75 0.15 140); }
.dark .hljs-number { color: oklch(0.75 0.2 30); }
.dark .hljs-comment { color: oklch(0.55 0 0); }
.dark .hljs-function { color: oklch(0.75 0.2 230); }
```

### 1.3 Typography Enhancement

#### Headings in Responses
```css
/* Markdown heading styles */
.prose h1 {
  font-size: var(--text-xl);
  font-weight: var(--font-bold);
  letter-spacing: var(--tracking-tight);
  margin-top: var(--space-6);
  margin-bottom: var(--space-3);
  border-bottom: 1px solid var(--border);
  padding-bottom: var(--space-2);
}

.prose h2 {
  font-size: var(--text-lg);
  font-weight: var(--font-semibold);
  margin-top: var(--space-5);
  margin-bottom: var(--space-2);
}

.prose h3 {
  font-size: var(--text-base);
  font-weight: var(--font-medium);
  margin-top: var(--space-4);
  margin-bottom: var(--space-2);
}
```

#### Lists Enhancement
```css
/* Custom bullet styling */
.prose ul {
  list-style: none;
  padding-left: var(--space-5);
}

.prose ul li::before {
  content: "•";
  color: var(--primary);
  font-weight: bold;
  display: inline-block;
  width: 1em;
  margin-left: -1em;
}

/* Ordered list styling */
.prose ol {
  counter-reset: list-counter;
  list-style: none;
  padding-left: var(--space-5);
}

.prose ol li::before {
  counter-increment: list-counter;
  content: counter(list-counter) ".";
  color: var(--primary);
  font-weight: 600;
  display: inline-block;
  width: 1.5em;
  margin-left: -1.5em;
}
```

#### Blockquotes Enhancement
```css
.prose blockquote {
  border-left: 4px solid var(--primary);
  background: var(--primary) / 5%;
  padding: var(--space-4);
  margin: var(--space-4) 0;
  border-radius: 0 var(--radius) var(--radius) 0;
  position: relative;
}

.prose blockquote::before {
  content: """;
  position: absolute;
  left: var(--space-3);
  top: 0;
  font-size: 3rem;
  color: var(--primary) / 20%;
  line-height: 1;
}
```

### 1.4 Mermaid Diagram Container

```tsx
<div className="mermaid-container my-6 rounded-xl border border-border/60 
               bg-card overflow-hidden">
  {/* Header with controls */}
  <div className="mermaid-header flex items-center justify-between 
                 px-4 py-2 bg-muted/30 border-b border-border/40">
    <span className="text-xs font-medium text-muted-foreground flex items-center gap-2">
      <GitBranch className="h-3.5 w-3.5" />
      Diagram
    </span>
    <div className="flex items-center gap-1">
      <Button variant="ghost" size="icon" onClick={handleZoomIn}>
        <ZoomIn className="h-3.5 w-3.5" />
      </Button>
      <Button variant="ghost" size="icon" onClick={handleZoomOut}>
        <ZoomOut className="h-3.5 w-3.5" />
      </Button>
      <Button variant="ghost" size="icon" onClick={handleFullscreen}>
        <Maximize2 className="h-3.5 w-3.5" />
      </Button>
    </div>
  </div>
  
  {/* Diagram content */}
  <div className="mermaid-content p-6 overflow-auto" 
       style={{ transform: `scale(${zoom})` }}>
    <div dangerouslySetInnerHTML={{ __html: svg }} />
  </div>
</div>
```

### 1.5 Input Area Refinement

```tsx
<div className="chat-input-container sticky bottom-0 bg-background/95 
               backdrop-blur-sm border-t border-border/40 p-4">
  <div className="chat-input-wrapper relative max-w-3xl mx-auto">
    <div className="chat-input group relative rounded-2xl border border-border
                   bg-card shadow-sm transition-all duration-200
                   focus-within:border-primary focus-within:shadow-[0_0_0_3px_rgba(var(--primary),0.1)]">
      
      {/* Textarea */}
      <textarea
        className="w-full resize-none bg-transparent px-4 py-3 pr-12
                  text-foreground placeholder:text-muted-foreground
                  focus:outline-none min-h-[48px] max-h-[200px]"
        placeholder="Ask a question..."
        rows={1}
      />
      
      {/* Send button */}
      <Button
        size="icon"
        className="absolute right-2 bottom-2 h-8 w-8 rounded-xl
                  bg-primary text-primary-foreground
                  disabled:opacity-40 disabled:cursor-not-allowed
                  transition-all duration-200 hover:scale-105"
        disabled={!hasContent}
      >
        <Send className="h-4 w-4" />
      </Button>
      
      {/* Character count (appears when typing) */}
      <span className="absolute right-14 bottom-3 text-xs text-muted-foreground
                      opacity-0 group-focus-within:opacity-100 transition-opacity">
        {charCount}
      </span>
    </div>
    
    {/* Mode selector below input */}
    <div className="flex items-center justify-between mt-2 px-1">
      <QueryModeSelector />
      <span className="text-xs text-muted-foreground">
        Enter to send · Shift+Enter for new line
      </span>
    </div>
  </div>
</div>
```

---

## Phase 2: Functional Enhancements

### 2.1 Collapsible Long Content

#### Long Code Blocks
```tsx
const MAX_VISIBLE_LINES = 20;

function CollapsibleCodeBlock({ code, language }) {
  const [expanded, setExpanded] = useState(false);
  const lines = code.split('\n');
  const isLong = lines.length > MAX_VISIBLE_LINES;
  
  const visibleCode = expanded 
    ? code 
    : lines.slice(0, MAX_VISIBLE_LINES).join('\n') + '\n...';
  
  return (
    <div className="code-block">
      {/* ... code content ... */}
      {isLong && (
        <button onClick={() => setExpanded(!expanded)}>
          {expanded ? 'Collapse' : `Show all ${lines.length} lines`}
        </button>
      )}
    </div>
  );
}
```

#### Long Responses
```tsx
const MAX_VISIBLE_PARAGRAPHS = 5;

function CollapsibleResponse({ content }) {
  const [expanded, setExpanded] = useState(false);
  const paragraphs = content.split('\n\n');
  const isLong = paragraphs.length > MAX_VISIBLE_PARAGRAPHS;
  
  // Similar pattern with "Read more" button
}
```

### 2.2 History Panel Enhancement

#### Grouped by Date
```tsx
function HistoryPanel({ conversations }) {
  const grouped = useMemo(() => {
    return conversations.reduce((acc, conv) => {
      const date = formatRelativeDate(conv.updatedAt);
      if (!acc[date]) acc[date] = [];
      acc[date].push(conv);
      return acc;
    }, {});
  }, [conversations]);
  
  return (
    <ScrollArea>
      {Object.entries(grouped).map(([date, items]) => (
        <div key={date}>
          <div className="sticky top-0 bg-background/95 backdrop-blur-sm
                         px-4 py-2 text-xs font-medium text-muted-foreground
                         border-b border-border/40">
            {date}
          </div>
          {items.map(conv => <ConversationItem key={conv.id} {...conv} />)}
        </div>
      ))}
    </ScrollArea>
  );
}
```

#### Enhanced Search
```tsx
<div className="history-search relative">
  <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
  <input
    type="text"
    placeholder="Search conversations..."
    className="w-full pl-10 pr-4 py-2 bg-muted/50 rounded-lg
              border border-border/40 text-sm
              focus:outline-none focus:border-primary"
  />
  {searchQuery && (
    <button 
      onClick={clearSearch}
      className="absolute right-3 top-1/2 -translate-y-1/2"
    >
      <X className="h-4 w-4" />
    </button>
  )}
</div>
```

### 2.3 Thinking State Enhancement

```tsx
function ThinkingIndicator({ duration, stage }) {
  return (
    <div className="thinking-indicator rounded-xl border border-border/60 
                   bg-muted/30 overflow-hidden">
      <div className="flex items-center justify-between px-4 py-3">
        <div className="flex items-center gap-3">
          <div className="relative">
            <Brain className="h-5 w-5 text-primary animate-pulse" />
            <span className="absolute -top-0.5 -right-0.5 flex h-2 w-2">
              <span className="animate-ping absolute h-full w-full rounded-full bg-primary/60" />
              <span className="relative rounded-full h-2 w-2 bg-primary" />
            </span>
          </div>
          <span className="text-sm font-medium">
            {stage === 'thinking' ? 'Reasoning...' : 'Generating response...'}
          </span>
        </div>
        <span className="text-xs text-muted-foreground font-mono">
          {formatDuration(duration)}
        </span>
      </div>
      
      {/* Progress bar */}
      <div className="h-1 bg-muted">
        <div className="h-full bg-primary/60 animate-progress" 
             style={{ width: `${Math.min(duration / 30 * 100, 95)}%` }} />
      </div>
    </div>
  );
}
```

---

## Phase 3: Delight Features

### 3.1 Smooth Scroll Behavior

```tsx
function useSmoothScrollToBottom(containerRef, messages) {
  useEffect(() => {
    if (containerRef.current) {
      containerRef.current.scrollTo({
        top: containerRef.current.scrollHeight,
        behavior: 'smooth'
      });
    }
  }, [messages]);
}
```

### 3.2 Keyboard Shortcuts

```tsx
const shortcuts = {
  'mod+n': { action: 'newConversation', label: 'New conversation' },
  'mod+k': { action: 'focusSearch', label: 'Search' },
  'mod+enter': { action: 'submit', label: 'Send message' },
  'mod+shift+c': { action: 'copyLast', label: 'Copy last response' },
  'escape': { action: 'clearInput', label: 'Clear input' },
};
```

### 3.3 Sound Feedback (Optional)

```tsx
const useSoundFeedback = (enabled: boolean) => {
  const play = useCallback((sound: 'send' | 'receive' | 'error') => {
    if (!enabled) return;
    
    const sounds = {
      send: '/sounds/send.mp3',
      receive: '/sounds/receive.mp3', 
      error: '/sounds/error.mp3',
    };
    
    const audio = new Audio(sounds[sound]);
    audio.volume = 0.3;
    audio.play().catch(() => {});
  }, [enabled]);
  
  return play;
};
```

---

## Component Specifications

### ChatMessage Component

| Property | Type | Description |
|----------|------|-------------|
| `message` | `Message` | Message data object |
| `isLast` | `boolean` | Whether this is the last message |
| `onCopy` | `() => void` | Copy callback |
| `onRegenerate` | `() => void` | Regenerate callback |
| `showMetadata` | `boolean` | Show tokens/timing info |

### CodeBlock Component

| Property | Type | Description |
|----------|------|-------------|
| `code` | `string` | Code content |
| `language` | `string` | Programming language |
| `showLineNumbers` | `boolean` | Display line numbers |
| `collapsible` | `boolean` | Allow collapse for long code |
| `maxLines` | `number` | Lines before collapse (default: 20) |

### MermaidDiagram Component

| Property | Type | Description |
|----------|------|-------------|
| `code` | `string` | Mermaid diagram code |
| `theme` | `'light' \| 'dark' \| 'auto'` | Color theme |
| `zoomable` | `boolean` | Enable zoom controls |
| `fullscreenable` | `boolean` | Enable fullscreen |

### ChatInput Component

| Property | Type | Description |
|----------|------|-------------|
| `value` | `string` | Input value |
| `onChange` | `(value: string) => void` | Change handler |
| `onSubmit` | `() => void` | Submit handler |
| `disabled` | `boolean` | Disable input |
| `maxLength` | `number` | Character limit |
| `showCharCount` | `boolean` | Show character count |

---

## Layout Specifications

### Desktop Layout (≥1024px)

```
┌─────────────────────────────────────────────────────────────────┐
│ Header (48px)                                                   │
├────────────────┬─────────────────────────────┬─────────────────┤
│ Sidebar (64px) │ History    │ Chat Area     │ Details Panel   │
│                │ (280px)    │ (flexible)    │ (320px, hidden) │
│                │            │ max-w: 800px  │                 │
│                │            │ centered      │                 │
├────────────────┴────────────┴───────────────┴─────────────────┤
│ Input Area (sticky bottom)                                      │
└─────────────────────────────────────────────────────────────────┘
```

### Tablet Layout (768px - 1023px)

```
┌─────────────────────────────────────────────────────────────────┐
│ Header (48px)                                                   │
├────────────────┬─────────────────────────────┴─────────────────┤
│ Sidebar (64px) │ Chat Area (flexible)                          │
│                │ max-w: 700px, centered                        │
│                │                                               │
│                │ [History toggle in header]                    │
├────────────────┴───────────────────────────────────────────────┤
│ Input Area (sticky bottom)                                      │
└─────────────────────────────────────────────────────────────────┘
```

### Mobile Layout (<768px)

```
┌─────────────────────────────────────────────────────────────────┐
│ Header (48px) [Menu] EdgeQuake [Search] [Actions]              │
├─────────────────────────────────────────────────────────────────┤
│ Chat Area (full width, padding: 16px)                          │
│                                                                 │
│                                                                 │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│ Input Area (sticky bottom, safe-area-inset)                    │
│ [Mode Selector] - horizontal scroll                             │
└─────────────────────────────────────────────────────────────────┘

[History: Bottom sheet, drag to reveal]
[Details: Bottom sheet, tap message to reveal]
```

---

## Responsive Design

### Breakpoints
```css
--bp-sm: 640px;   /* Small phones landscape */
--bp-md: 768px;   /* Tablets */
--bp-lg: 1024px;  /* Desktop */
--bp-xl: 1280px;  /* Large desktop */
--bp-2xl: 1536px; /* Ultra-wide */
```

### Container Widths
```css
/* Chat message area */
@media (max-width: 767px) {
  .chat-container { max-width: 100%; padding: 0 16px; }
}
@media (min-width: 768px) and (max-width: 1023px) {
  .chat-container { max-width: 700px; margin: 0 auto; }
}
@media (min-width: 1024px) {
  .chat-container { max-width: 800px; margin: 0 auto; }
}
```

### Touch Targets
```css
/* Minimum touch target: 44px (WCAG AA) */
/* Recommended: 48px */
.touch-target {
  min-height: 44px;
  min-width: 44px;
}

/* Mobile buttons */
@media (max-width: 767px) {
  .btn-mobile {
    min-height: 48px;
    padding: 12px 16px;
  }
}
```

---

## Animation Guidelines

### Message Animations
```css
/* Slide in from side */
@keyframes slideInFromRight {
  from { opacity: 0; transform: translateX(8px); }
  to { opacity: 1; transform: translateX(0); }
}

@keyframes slideInFromLeft {
  from { opacity: 0; transform: translateX(-8px); }
  to { opacity: 1; transform: translateX(0); }
}

.message-user { animation: slideInFromRight 0.2s ease-out; }
.message-assistant { animation: slideInFromLeft 0.2s ease-out; }
```

### Thinking Animation
```css
@keyframes thinkingPulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}

@keyframes progressBar {
  from { width: 0%; }
  to { width: 100%; }
}

.thinking-icon { animation: thinkingPulse 1s ease-in-out infinite; }
.progress-bar { animation: progressBar 30s linear; }
```

### Reduced Motion
```css
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
  }
}
```

---

## Implementation Checklist

### Phase 1: Visual Polish (Week 1)
- [ ] Update message bubble styles (shadows, gradients)
- [ ] Add slide-in animations
- [ ] Redesign code block component
- [ ] Implement line numbers
- [ ] Update typography styles
- [ ] Enhance blockquote styling
- [ ] Polish mermaid diagram container
- [ ] Refine input area design
- [ ] Add focus states

### Phase 2: Functional (Week 2)
- [ ] Implement collapsible code blocks
- [ ] Add copy button always-visible
- [ ] Group history by date
- [ ] Enhance search functionality
- [ ] Improve thinking indicator
- [ ] Add character count
- [ ] Implement auto-resize textarea

### Phase 3: Delight (Week 3)
- [ ] Smooth scroll behavior
- [ ] Keyboard shortcuts dialog
- [ ] Mobile-optimized layout
- [ ] Bottom sheet for history (mobile)
- [ ] Touch gestures
- [ ] Sound feedback (optional)

### Testing & QA
- [ ] Cross-browser testing
- [ ] Mobile device testing
- [ ] Accessibility audit
- [ ] Performance profiling
- [ ] Dark mode verification
- [ ] Reduced motion testing

---

## Files to Modify

| File | Changes |
|------|---------|
| `src/components/query/query-interface.tsx` | Message styling, animations |
| `src/components/query/markdown-renderer.tsx` | Code blocks, typography |
| `src/components/query/conversation-history-panel.tsx` | Grouping, search |
| `src/components/query/thinking-display.tsx` | Enhanced indicator |
| `src/app/globals.css` | Animation keyframes, prose styles |
| `src/app/design-tokens.css` | Chat-specific tokens |
| `src/app/(dashboard)/query/page.tsx` | Layout adjustments |

---

## New Files to Create

| File | Purpose |
|------|---------|
| `src/components/query/code-block.tsx` | Enhanced code block component |
| `src/components/query/mermaid-diagram.tsx` | Mermaid with controls |
| `src/components/query/chat-input.tsx` | Refined input component |
| `src/components/query/history-group.tsx` | Grouped history items |

---

*Last updated: December 26, 2025*
