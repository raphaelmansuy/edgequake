# Query Page Screen Specification

> Complete screen design for the `/query` chat interface.

## Overview

The Query page is the primary interface for interacting with the EdgeQuake knowledge graph through natural language queries. It features a chat-based interface with conversation history, rich markdown rendering, and source citations.

---

## Screen Layout

### Desktop (≥1024px)

```
┌─────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ Header (48px)                                                                                           │
│ ┌─────────┐ ┌──────────────────────────────────────────────────────────────────────────────────────────┐│
│ │EdgeQuake│ │ [Test Workspace ▾]                                          ● v0.1.0  🌍  ☀️  👤       ││
│ └─────────┘ └──────────────────────────────────────────────────────────────────────────────────────────┘│
├───────────────────────────┬──────────────────────────────────────────────────────────┬─────────────────-┤
│ SIDEBAR (64px collapsed)  │  MAIN CONTENT AREA                                       │ DETAILS PANEL    │
│                           │                                                          │ (320px, hidden)  │
│ ┌─────────────────────┐   │  ┌──────────────────────────────────────────────────────┐│                  │
│ │ 🏠 Dashboard        │   │  │ Breadcrumb: EdgeQuake > Query                        ││ [Click node to   │
│ │ 🕸️ Knowledge Graph  │   │  └──────────────────────────────────────────────────────┘│  view details]   │
│ │ 📄 Documents        │   │                                                          │                  │
│ │ 💬 Query ◀──────────│   │  ┌──────────────────────────────────────────────────────┐│                  │
│ │ 🔌 API Explorer     │   │  │ Query     Ask questions...     [+New] [Mode] [⚙️]   ││                  │
│ │ ⚙️ Settings         │   │  └──────────────────────────────────────────────────────┘│                  │
│ │                     │   │                                                          │                  │
│ │ ◀ Collapse          │   │  ┌──────────────────────────────────────────────────────┐│                  │
│ │                     │   │  │                                                      ││                  │
│ │ ┌─────────────────┐ │   │  │              CHAT MESSAGE AREA                       ││                  │
│ │ │ 🌐 EdgeQuake    │ │   │  │              (Messages scroll here)                  ││                  │
│ │ │ v0.1.0         │ │   │  │                                                      ││                  │
│ │ └─────────────────┘ │   │  │  ┌────────────────────────────────────────────┐      ││                  │
│ └─────────────────────┘   │  │  │ User message bubble                       │ 👤   ││                  │
│                           │  │  └────────────────────────────────────────────┘      ││                  │
│                           │  │                                                      ││                  │
│                           │  │  ┌────────────────────────────────────────────┐      ││                  │
│                           │  │✨│ Assistant response with markdown...        │      ││                  │
│                           │  │  │ - Code blocks                              │      ││                  │
│                           │  │  │ - Mermaid diagrams                         │      ││                  │
│                           │  │  │ - KaTeX math                               │      ││                  │
│                           │  │  └────────────────────────────────────────────┘      ││                  │
│                           │  │  │ hybrid │ 234 tokens │ 3.2s     📋 🔄             ││                  │
│                           │  │  │ 📚 Sources: document.md (3 chunks)               ││                  │
│                           │  │                                                      ││                  │
│                           │  └──────────────────────────────────────────────────────┘│                  │
│                           │                                                          │                  │
│                           │  ┌──────────────────────────────────────────────────────┐│                  │
│                           │  │ ┌──────────────────────────────────────────────┐     ││                  │
│                           │  │ │ Ask a question...                       [→] │     ││                  │
│                           │  │ └──────────────────────────────────────────────┘     ││                  │
│                           │  │ [Local] [Global] [Hybrid] [Simple]  Enter to send   ││                  │
│                           │  └──────────────────────────────────────────────────────┘│                  │
├───────────────────────────┼──────────────────────────────────────────────────────────┴─────────────────-┤
│                           │                                                                             │
│ HISTORY PANEL (280px)     │                                                                             │
│                           │                                                                             │
│ ┌─────────────────────┐   │                                                                             │
│ │ HISTORY      [+] [◀]│   │                                                                             │
│ ├─────────────────────┤   │                                                                             │
│ │ 🔍 Search...        │   │                                                                             │
│ ├─────────────────────┤   │                                                                             │
│ │ Today               │   │                                                                             │
│ │ ┌─────────────────┐ │   │                                                                             │
│ │ │ 💬 What are the │ │   │                                                                             │
│ │ │ main entities...│ │   │                                                                             │
│ │ │ 2 msgs · 11:56  │ │   │                                                                             │
│ │ └─────────────────┘ │   │                                                                             │
│ │ ┌─────────────────┐ │   │                                                                             │
│ │ │ 💬 Summarize... │ │   │                                                                             │
│ │ │ 4 msgs · 10:30  │ │   │                                                                             │
│ │ └─────────────────┘ │   │                                                                             │
│ │                     │   │                                                                             │
│ │ Yesterday           │   │                                                                             │
│ │ ┌─────────────────┐ │   │                                                                             │
│ │ │ 💬 Find conn... │ │   │                                                                             │
│ │ │ 3 msgs · 3:45   │ │   │                                                                             │
│ │ └─────────────────┘ │   │                                                                             │
│ └─────────────────────┘   │                                                                             │
└───────────────────────────┴─────────────────────────────────────────────────────────────────────────────┘
```

### Tablet (768px - 1023px)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Header (48px)                                                               │
│ ┌─────────┐ ┌────────────────────────────────────────────────────────────┐ │
│ │EdgeQuake│ │ [Test Workspace ▾]                        ● 🌍 ☀️ 👤      │ │
│ └─────────┘ └────────────────────────────────────────────────────────────┘ │
├───────────┬─────────────────────────────────────────────────────────────────┤
│ SIDEBAR   │  MAIN CONTENT AREA                                             │
│ (64px)    │                                                                 │
│ ┌───────┐ │  ┌───────────────────────────────────────────────────────────┐ │
│ │ 🏠    │ │  │ Query                     [+New] [Mode▾] [History] [⚙️] │ │
│ │ 🕸️    │ │  └───────────────────────────────────────────────────────────┘ │
│ │ 📄    │ │                                                                 │
│ │ 💬 ◀  │ │  ┌───────────────────────────────────────────────────────────┐ │
│ │ 🔌    │ │  │                                                           │ │
│ │ ⚙️    │ │  │              CHAT MESSAGE AREA                            │ │
│ └───────┘ │  │              (Full width, max-w-700px centered)           │ │
│           │  │                                                           │ │
│           │  └───────────────────────────────────────────────────────────┘ │
│           │                                                                 │
│           │  ┌───────────────────────────────────────────────────────────┐ │
│           │  │ ┌───────────────────────────────────────────────────┐     │ │
│           │  │ │ Ask a question...                            [→] │     │ │
│           │  │ └───────────────────────────────────────────────────┘     │ │
│           │  └───────────────────────────────────────────────────────────┘ │
└───────────┴─────────────────────────────────────────────────────────────────┘

[History: Slide-out panel from right, triggered by History button]
```

### Mobile (<768px)

```
┌─────────────────────────────────────────────────┐
│ Header (48px)                                   │
│ ┌───────────────────────────────────────────┐   │
│ │ [≡] EdgeQuake                  ● ☀️ 👤   │   │
│ └───────────────────────────────────────────┘   │
├─────────────────────────────────────────────────┤
│ ┌───────────────────────────────────────────┐   │
│ │ Query    [+New] [Mode ▾] [📜]            │   │
│ └───────────────────────────────────────────┘   │
├─────────────────────────────────────────────────┤
│                                                 │
│              CHAT MESSAGE AREA                  │
│              (Full width, 16px padding)         │
│                                                 │
│  ┌───────────────────────────────────────┐      │
│  │ User message                     │ 👤       │
│  └───────────────────────────────────────┘      │
│                                                 │
│  ┌───────────────────────────────────────┐      │
│✨│ Assistant response...                 │      │
│  └───────────────────────────────────────┘      │
│                                                 │
│                                                 │
├─────────────────────────────────────────────────┤
│ INPUT AREA (with safe-area-inset)               │
│ ┌───────────────────────────────────────────┐   │
│ │ Ask a question...                    [→] │   │
│ └───────────────────────────────────────────┘   │
│ [🔍] [🌐] [⚡ Hybrid ▾] [📝]                    │
└─────────────────────────────────────────────────┘

[Sidebar: Off-canvas menu from left]
[History: Bottom sheet, swipe up to reveal]
```

---

## Component Breakdown

### 1. Page Header
- **Title**: "Query" with subtitle "Ask questions about your knowledge graph"
- **Actions**: 
  - New conversation button (+)
  - Mode selector toggle
  - Settings button
  - History toggle (tablet/mobile)

### 2. Chat Message Area
- **Scroll container**: Full height minus header/input
- **Max width**: 800px (desktop), 700px (tablet), 100% (mobile)
- **Padding**: 24px (desktop), 16px (mobile)
- **Auto-scroll**: Scroll to bottom on new messages

### 3. Empty State
- **Illustration**: AI/chat icon with subtle animation
- **Heading**: "Ask about your knowledge graph"
- **Description**: Encouraging text about capabilities
- **Suggestions**: 4 clickable suggestion cards

### 4. Chat Messages
- See `chat-message.md` specification

### 5. Input Area
- See `chat-input.md` specification
- **Position**: Sticky bottom
- **Background**: Semi-transparent with blur

### 6. History Panel
- **Width**: 280px (desktop), slide-out (tablet), bottom sheet (mobile)
- **Sections**: Search, grouped by date, conversation items
- **Actions**: New, collapse, delete

### 7. Details Panel (Optional)
- **Width**: 320px
- **Content**: Source details, entity info, related queries
- **Trigger**: Click on source citation or entity mention

---

## State Variations

### Empty State (No Conversation)

```
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│                          ┌─────────┐                            │
│                          │   ✨    │                            │
│                          └─────────┘                            │
│                                                                 │
│                  Ask about your knowledge graph                 │
│                                                                 │
│    I can help you explore entities, find connections,          │
│    and uncover insights from your documents.                   │
│                                                                 │
│                        Try asking:                              │
│                                                                 │
│    ┌─────────────────────────────────────────────────────┐     │
│    │ 🔍 What are the main entities in my knowledge graph?│     │
│    └─────────────────────────────────────────────────────┘     │
│                                                                 │
│    ┌─────────────────────────────────────────────────────┐     │
│    │ 💡 Summarize the key relationships between documents│     │
│    └─────────────────────────────────────────────────────┘     │
│                                                                 │
│    ┌─────────────────────────────────────────────────────┐     │
│    │ 🔗 Find connections between people and organizations│     │
│    └─────────────────────────────────────────────────────┘     │
│                                                                 │
│    ┌─────────────────────────────────────────────────────┐     │
│    │ 📚 What topics are covered in my documents?         │     │
│    └─────────────────────────────────────────────────────┘     │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Loading State (Processing Query)

```
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│  ┌───────────────────────────────────────────────────────┐      │
│  │ What are the main entities in my knowledge graph?     │ 👤  │
│  └───────────────────────────────────────────────────────┘      │
│                                                                 │
│  ┌───────────────────────────────────────────────────────┐      │
│✨│ 🧠 Reasoning...                                   2.1s│      │
│  │ ┌─────────────────────────────────────────────────┐   │      │
│  │ │ ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  │   │      │
│  │ │ Analyzing knowledge graph structure...          │   │      │
│  │ └─────────────────────────────────────────────────┘   │      │
│  │                                                       │      │
│  │ ┌─────────────────────────────────────────────────┐   │      │
│  │ │ Generating response...                          │   │      │
│  │ │ ● ● ●                                           │   │      │
│  │ └─────────────────────────────────────────────────┘   │      │
│  └───────────────────────────────────────────────────────┘      │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Streaming State (Receiving Response)

```
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│  ┌───────────────────────────────────────────────────────┐      │
│  │ What are the main entities in my knowledge graph?     │ 👤  │
│  └───────────────────────────────────────────────────────┘      │
│                                                                 │
│  ┌───────────────────────────────────────────────────────┐      │
│✨│ EdgeQuake  11:56 PM                                   │      │
│  │                                                       │      │
│  │ The main entities in your knowledge graph are:       │      │
│  │                                                       │      │
│  │ 1. **Reasoning-Trace-Augmented RAG** - A concept     │      │
│  │    with 4 connections to other entities.             │      │
│  │                                                       │      │
│  │ 2. **BITS Pilani** - An organization entity▋         │      │
│  │                                                       │      │
│  └───────────────────────────────────────────────────────┘      │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘

[Input disabled, Stop button visible]
```

### Error State

```
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│  ┌───────────────────────────────────────────────────────┐      │
│  │ What are the main entities in my knowledge graph?     │ 👤  │
│  └───────────────────────────────────────────────────────┘      │
│                                                                 │
│  ┌───────────────────────────────────────────────────────┐      │
│❌│ EdgeQuake  11:56 PM                                   │      │
│  │                                                       │      │
│  │ Sorry, I encountered an error while processing       │      │
│  │ your query. Please try again.                        │      │
│  │                                                       │      │
│  │ Error: Connection to LLM provider timed out.         │      │
│  │                                                       │      │
│  │                                    [🔄 Retry]        │      │
│  └───────────────────────────────────────────────────────┘      │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Interactions

### Submit Query
1. User types in input area
2. Character count appears (after threshold)
3. User presses Enter or clicks Send
4. Input clears immediately
5. User message appears with slide-in animation
6. Loading indicator appears
7. Thinking section shows (if enabled)
8. Response streams in progressively
9. Response complete, metadata and actions appear
10. Source citations expand below

### Switch Conversation
1. User clicks conversation in history
2. Current conversation saves automatically
3. Screen transitions to new conversation
4. Messages load (may show skeleton)
5. Scroll to bottom

### Copy Response
1. User hovers over message (or taps on mobile)
2. Actions become visible
3. User clicks copy button
4. Content copies to clipboard
5. Button shows checkmark for 2 seconds
6. Toast notification confirms

### Regenerate Response
1. User clicks regenerate on last message
2. Confirmation dialog (optional)
3. Previous response fades or gets replaced
4. New request sends
5. New response streams in

---

## Transitions & Animations

### Page Load
- Skeleton for history panel
- Empty state or messages fade in
- Input area slides up from bottom

### New Message
- User message: Slide in from right (200ms)
- Assistant message: Slide in from left (200ms)
- Smooth scroll to bottom

### Conversation Switch
- Cross-fade between conversations (150ms)
- History item highlights

### Panel Toggle
- History panel: Slide in/out (200ms)
- Details panel: Slide in/out (200ms)

### Loading States
- Shimmer animation on skeletons
- Pulsing dots for "thinking"
- Progress bar for long operations

---

## Accessibility

### Keyboard Navigation
- Tab through: History items → Messages → Input → Actions
- Escape: Close panels, clear input
- Enter: Submit query
- Arrow keys: Navigate history

### Screen Reader
- Page title: "Query - EdgeQuake"
- Live region for new messages
- Announce loading/streaming states
- Describe message roles and content

### Focus Management
- Auto-focus input on page load
- Return focus after actions
- Focus visible on all interactive elements

### Reduced Motion
- Disable slide-in animations
- Reduce shimmer effects
- Instant transitions

---

## Performance

### Lazy Loading
- History items beyond viewport
- Heavy components (Mermaid, code blocks)
- Previous conversation messages

### Virtualization
- History list (if >50 items)
- Long message lists (if >100 messages)

### Debouncing
- Search input: 300ms debounce
- Auto-resize: 100ms debounce
- Scroll events: RAF throttle

### Caching
- Conversation messages in memory
- Recent conversations in local storage
- Rendered Mermaid diagrams

---

## Error Handling

### Network Errors
- Show inline error in message area
- Offer retry button
- Keep user input for resend

### LLM Errors
- Display error message from API
- Suggest alternative actions
- Log for debugging

### Validation Errors
- Input too long: Warning color, disable send
- Empty input: Disable send button
- Invalid characters: Show error inline

---

## Files to Implement/Modify

| File | Purpose |
|------|---------|
| `src/app/(dashboard)/query/page.tsx` | Page layout and state |
| `src/components/query/query-interface.tsx` | Main chat container |
| `src/components/query/chat-message.tsx` | NEW: Message component |
| `src/components/query/chat-input.tsx` | NEW: Input component |
| `src/components/query/conversation-history-panel.tsx` | History sidebar |
| `src/components/query/empty-state.tsx` | Empty state design |

---

*Last updated: December 26, 2025*
