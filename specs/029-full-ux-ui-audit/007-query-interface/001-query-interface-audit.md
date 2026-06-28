# 001 — Query Interface Audit

**First Principle: Flow** — Common tasks require minimum decisions.

---

## Interface Layout Diagram

```
┌──────────────────────────────────────────────────────────────────────┐
│ HEADER (h-12, sticky, backdrop-blur)                                │
│  [☰] Query  ·  Ask questions...    [New] [Provider▼] [Mode▼] [Filter▼] [⚙] │
├────────────────────┬─────────────────────────────────────────────────┤
│                    │                                                 │
│  HISTORY PANEL     │  CHAT AREA                                      │
│  (hidden mobile)   │                                                 │
│                    │  [Empty state OR messages]                      │
│  Conversations:    │                                                 │
│  - Today           │                                                 │
│    · Chat 1        │                                                 │
│    · Chat 2        │  ─────────────────────────────────────────────  │
│  - Yesterday       │  INPUT AREA (sticky bottom)                     │
│    · Chat 3        │  📎  [Type your question...    ] [↑ Send]       │
│                    │                                                 │
└────────────────────┴─────────────────────────────────────────────────┘
```

---

## Issues

### QI-01 · Header Toolbar: 5 Controls in 48px Height

The query header packs **5 interactive controls** into a single 48px tall row:

```
Controls audit:
1. [New Conversation]   — low frequency (once per session)
2. [Provider/Model ▼]   — rare (user sets once, rarely changes)
3. [Mode ▼]             — occasional (users may switch per query)
4. [Document Filter ▼]  — moderate (users filter specific docs)
5. [⚙ Settings sheet]   — rare (power users only)
```

**Frequency analysis:** Most users will use controls 3 and 4 occasionally. Controls 1, 2, and 5 are low-frequency. Yet all 5 take equal visual weight in the header.

**Impact:** On 768-1024px viewports, these controls compress and the page title truncates. The subtitle ("Ask questions about your knowledge graph") is only visible `hidden md:inline` — it's the most user-friendly text and it's hidden.

**Fix: Progressive disclosure for low-frequency controls**

```
PRIMARY (always visible):
  [New]  [Mode: Hybrid ▼]  [⚙]

SECONDARY (inside ⚙ settings sheet):
  - Provider / Model
  - Document filter
  - Streaming toggle
  - Max tokens
  - Temperature
  - System prompt
```

The Mode selector is the one control worth keeping prominent as it has direct, visible impact on response quality.

### QI-02 · Mode Selector: 4 Buttons with Undefined Defaults

```
Current modes: [Local] [Global] [Hybrid] [Simple]
```

The mode buttons are displayed as an icon button group with tooltips. Issues:

1. **No visual default recommendation** — "Hybrid" is best for most users but nothing indicates this
2. **Technical labels** — "Local," "Global," "Hybrid" are RAG system terms, not user-friendly descriptors
3. **"Simple" is actually "Naive mode"** — the label "Simple" undersells it; it's "Direct AI" (no graph context)

**Better labels:**
```
Current → Proposed
─────────────────────────────────────────────
Local   → "Focused"   (narrow, targeted search)
Global  → "Broad"     (full knowledge graph)
Hybrid  → "Smart"     (recommended, default)
Simple  → "Direct"    (no graph, fastest)
```

Add visual default indicator:
```
[Focused] [Broad] [● Smart ✓ Default] [Direct]
```

### QI-03 · Chat Message Width Inconsistency

User messages use `max-w-[95%] sm:max-w-[85%]` while the message container uses `max-w-4xl lg:max-w-5xl`. This can create situations where:

- User message bubble is very wide (95% of viewport)
- Assistant message renders in full container width with no max-width on the prose itself

**Fix:** Standardize on the design token:

```typescript
// query-interface.tsx — message container
<div style={{ maxWidth: 'var(--chat-message-max-width)' }} className="mx-auto px-4 sm:px-6 py-6">

// chat-message.tsx — user message  
<div className="flex justify-end mb-6">
  <div className="max-w-[75%]">  {/* consistent, not 95% */}
    ...message content...
  </div>
</div>
```

### QI-04 · Image Attachment UX

The image attachment area renders inline with the text input:

```typescript
// query-interface.tsx:190
<div role="list" aria-label="Attached images">
  {attachedImages.map((img, idx) => (
    <div role="listitem">
      ...preview + remove button...
    </div>
  ))}
</div>
```

**Issues:**
1. Image previews push the text area down — the input area layout jumps
2. No size limit indicator (user doesn't know max image count until they hit it)
3. The `📎` clip icon for image attachment is slightly ambiguous — `ImagePlus` icon (already imported) is clearer

**Fix:** Render image previews above the textarea, not inline:

```
┌─────────────────────────────────────────────────────────┐
│  📷 image1.png [×]  📷 image2.png [×]                  │  ← above textarea
│ ─────────────────────────────────────────────────────── │
│  [Type your question...                        ] [↑]    │
│  [📷 Add image]                         0/4 images      │  ← count indicator
└─────────────────────────────────────────────────────────┘
```

### QI-05 · Conversation History Panel: No Empty State

The history panel (`ConversationHistoryPanelV2`) shows a list of conversations. When there are no conversations:

- Panels shows an empty list (white space)
- No guidance text like "Your conversations will appear here"
- No keyboard shortcut hint

### QI-06 · Source Citations: No Visual Hierarchy

The `SourceCitations` component shows document references. Without seeing the implementation, common issues in citation UI:

- Citations rendered as full URLs or long document names (hard to scan)
- No click-to-navigate to document behavior
- No relevance scoring indicator

### QI-07 · "Stop" Button Placement

During streaming, a `StopCircle` button appears. Its placement next to the `Send` button (which is now hidden) is correct but:

- The transition from [Send] to [Stop] should be animated (`transition-all`)
- The `Stop` button should be more visually distinctive (destructive color, not just an icon change)

---

## Positive Findings

```
✅ role="log" on message container with aria-live="polite"
✅ role="form" + aria-label on query form
✅ Keyboard submission (Enter to send, Shift+Enter for newline)
✅ Auto-scroll to latest message
✅ Empty state with suggestion chips
✅ Streaming indicator with accessible status
✅ Message copy button
✅ Regenerate last message button
✅ Thinking/reasoning section collapsible
✅ Paste and drag-drop image support
```

---

## Recommended Query Interface Redesign

```
┌──────────────────────────────────────────────────────────────────┐
│ Query                      [Smart ▼]           [+ New]  [⚙]    │
│            Minimal, focused header                               │
├────────────────────┬─────────────────────────────────────────────┤
│ History            │                                             │
│ ─────────────────  │  [Empty state or messages]                  │
│ Today              │                                             │
│ · Session 1        │                                             │
│ · Session 2        │                                             │
│                    │  ───────────────────────────────────────    │
│                    │  ┌─────────────────────────────────────┐   │
│                    │  │  Type your question...              │   │
│                    │  │                              [↑ Ask] │   │
│                    │  └─────────────────────────────────────┘   │
│                    │  [📷 Filter docs ▼]  Smart mode · GPT-4o   │
└────────────────────┴─────────────────────────────────────────────┘
```

The subtle context row below the input (mode indicator, model name, document filter as text) communicates current settings without taking header space.

---

## External References

- [ChatGPT UX Analysis — UX Collective](https://uxdesign.cc/chatgpt-ux-analysis)
- [Chat Interface Best Practices — NNGroup](https://www.nngroup.com/articles/ai-chat-interface-usability/)
- [Progressive Disclosure in Settings — NNGroup](https://www.nngroup.com/articles/progressive-disclosure/)
- [Linear.app command palette](https://linear.app/) — reference for minimal, keyboard-first UI
