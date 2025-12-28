# ChatMessage Component Specification

> Enhanced chat message bubble with beautiful styling, animations, and interactions.

## Overview

The ChatMessage component displays individual messages in the chat interface with distinct styling for user and assistant messages, smooth animations, and interactive elements.

---

## Visual Design

### User Message

```
                                    ┌─────────────────────────────────────┐
                                    │ Question about knowledge graph?     │ 👤
                                    │                                     │
                                    └─────────────────────────────────────┘
```

**Styling:**
- Background: `bg-gradient-to-br from-primary to-primary/90`
- Text: `text-primary-foreground`
- Border radius: `rounded-2xl rounded-tr-sm` (tail on top-right)
- Shadow: `shadow-[0_2px_8px_rgba(0,0,0,0.08)]`
- Max width: `max-w-[85%]`
- Padding: `px-4 py-3`
- Animation: Slide-in from right, 200ms

**Avatar:**
- Position: Right of message
- Size: `h-8 w-8`
- Icon: User silhouette
- Ring: `ring-2 ring-background`

### Assistant Message

```
  ┌─────────────────────────────────────────────────────────────────┐
  │ EdgeQuake  11:56 PM                                             │
  ├─────────────────────────────────────────────────────────────────┤
  │ 🧠 Reasoning                                               2.1s │  ▼ Collapse
  │ ┌───────────────────────────────────────────────────────────┐   │
  │ │ Analyzing knowledge graph structure...                    │   │
  │ │ Found 3 main entities with connections.                   │   │
  │ └───────────────────────────────────────────────────────────┘   │
  ├─────────────────────────────────────────────────────────────────┤
✨│ The main entities in your knowledge graph are:                  │
  │                                                                 │
  │ 1. **Reasoning-Trace-Augmented RAG** - A concept with 4 conn.   │
  │ 2. **BITS Pilani** - An organization entity                     │
  │ 3. **Retrieval-Augmented Generation** - Core RAG concept        │
  │                                                                 │
  │ ```python                                                       │
  │ # Query example                                                 │
  │ entities = graph.get_entities(limit=10)                         │
  │ ```                                                             │
  └─────────────────────────────────────────────────────────────────┘
  │ hybrid │ 234 tokens │ 3.2s                      📋 Copy  🔄 Regen │
  ├─────────────────────────────────────────────────────────────────┤
  │ 📚 Sources: mega_rag.md (3 chunks) | from_fact.md (2 chunks)    │
  └─────────────────────────────────────────────────────────────────┘
```

**Styling:**
- Background: `bg-card`
- Border: `border border-border/60`
- Border radius: `rounded-2xl rounded-tl-sm` (tail on top-left)
- Shadow: `shadow-[0_1px_4px_rgba(0,0,0,0.04)]`
- Max width: `max-w-[85%]`
- Padding: `px-4 py-3`
- Animation: Slide-in from left, 200ms

**Avatar:**
- Position: Left of message
- Size: `h-9 w-9`
- Icon: Sparkles with gradient
- Ring: `ring-2 ring-primary/20`

---

## Component Structure

```tsx
interface ChatMessageProps {
  message: {
    id: string;
    role: 'user' | 'assistant';
    content: string;
    timestamp?: Date;
    isStreaming?: boolean;
    isError?: boolean;
    mode?: 'local' | 'global' | 'hybrid' | 'simple';
    tokensUsed?: number;
    durationMs?: number;
    thinkingTimeMs?: number;
    context?: QueryContext;
  };
  isLast?: boolean;
  onCopy?: () => void;
  onRegenerate?: () => void;
  showMetadata?: boolean;
}

function ChatMessage({ message, isLast, onCopy, onRegenerate, showMetadata = true }: ChatMessageProps) {
  // Implementation
}
```

---

## Sections

### 1. Message Header (Assistant only)
- Model name: "EdgeQuake"
- Timestamp: "11:56 PM" format
- Styling: `text-sm font-medium`

### 2. Thinking Section (Collapsible)
- Toggle button with chevron
- Brain icon with pulse animation
- Duration display
- Content: Pre-formatted reasoning text
- Border: Left border accent

### 3. Main Content
- Rendered via MarkdownRenderer
- Supports code, math, diagrams
- Break-word for long content

### 4. Metadata Bar
- Mode badge: `hybrid`, `local`, etc.
- Token count with Zap icon
- Duration with Clock icon
- Actions: Copy, Regenerate

### 5. Source Citations
- Document links
- Entity links
- Chunk indicators

---

## States

### Default
- Full message displayed
- Metadata visible on hover (or always if `isLast`)

### Streaming
- Cursor blinking at end
- No metadata yet
- Thinking section may be visible

### Error
- Red text color: `text-destructive`
- Error icon instead of sparkles

---

## Animations

### Slide In (Enter)
```css
@keyframes messageSlideIn {
  from {
    opacity: 0;
    transform: translateY(8px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.message-enter {
  animation: messageSlideIn 0.2s ease-out;
}
```

### User Message (From Right)
```css
.message-user {
  animation: slideInFromRight 0.2s ease-out;
}

@keyframes slideInFromRight {
  from { opacity: 0; transform: translateX(8px); }
  to { opacity: 1; transform: translateX(0); }
}
```

### Assistant Message (From Left)
```css
.message-assistant {
  animation: slideInFromLeft 0.2s ease-out;
}

@keyframes slideInFromLeft {
  from { opacity: 0; transform: translateX(-8px); }
  to { opacity: 1; transform: translateX(0); }
}
```

---

## Interactions

### Copy Button
1. Click → Copy content to clipboard
2. Show checkmark for 2 seconds
3. Reset to copy icon

### Regenerate Button (Last message only)
1. Click → Trigger regeneration
2. Replace last assistant message

### Thinking Toggle
1. Click → Expand/collapse reasoning
2. Animate height transition
3. Rotate chevron icon

### Source Links
1. Hover → Show tooltip with preview
2. Click → Navigate to document/entity

---

## Accessibility

- Role: `article` for each message
- aria-label: "User message" or "Assistant message"
- Focus outline on interactive elements
- Keyboard: Tab through actions, Enter to activate
- Screen reader: Announce new messages

---

## Responsive Behavior

### Desktop (≥1024px)
- Max width: 85%
- Full metadata visible on last message

### Tablet (768px - 1023px)
- Max width: 90%
- Compact metadata

### Mobile (<768px)
- Max width: 95%
- Metadata stacked vertically
- Larger touch targets (48px)

---

## Implementation Notes

1. Use `memo` for performance
2. Parse thinking content with `parseCOTContent`
3. Handle streaming with cursor animation
4. Lazy load MarkdownRenderer for long content
5. Debounce copy feedback

---

## Example Usage

```tsx
<ChatMessage
  message={{
    id: '1',
    role: 'assistant',
    content: 'Here is your answer with **markdown** support.',
    timestamp: new Date(),
    mode: 'hybrid',
    tokensUsed: 234,
    durationMs: 3200,
  }}
  isLast={true}
  onCopy={() => toast.success('Copied!')}
  onRegenerate={() => handleRegenerate()}
/>
```

---

## CSS Classes

```css
/* Message container */
.chat-message {
  @apply flex mb-6 group;
}

.chat-message-user {
  @apply justify-end;
}

.chat-message-assistant {
  @apply justify-start;
}

/* Message bubble */
.message-bubble {
  @apply max-w-[85%] px-4 py-3;
}

.message-bubble-user {
  @apply bg-gradient-to-br from-primary to-primary/90 
         text-primary-foreground rounded-2xl rounded-tr-sm
         shadow-[0_2px_8px_rgba(0,0,0,0.08)];
}

.message-bubble-assistant {
  @apply bg-card border border-border/60 
         rounded-2xl rounded-tl-sm
         shadow-[0_1px_4px_rgba(0,0,0,0.04)];
}

/* Animations */
.message-enter-user {
  animation: slideInFromRight 0.2s ease-out;
}

.message-enter-assistant {
  animation: slideInFromLeft 0.2s ease-out;
}
```

---

*Last updated: December 26, 2025*
