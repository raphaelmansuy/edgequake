# Phase 2: Design Strategy - Query Page UX/UI Improvement

> **Date**: December 27, 2025  
> **Foundation**: [Audit Findings](./01_audit_findings.md)  
> **Objective**: Define SLICKness & MINIMALISM principles for EdgeQuake Query Page

---

## 1. Design Principles: SLICK Minimalism

### 1.1 Core Principles Definition

| #   | Principle                  | Definition                                           | Application Example                                                     |
| --- | -------------------------- | ---------------------------------------------------- | ----------------------------------------------------------------------- |
| 1   | **Instant Feedback**       | Every interaction provides immediate visual response | Button press → ripple effect; Query submit → skeleton appears in <100ms |
| 2   | **Progressive Disclosure** | Show only what's needed, reveal complexity on demand | Conversation list shows title + time; expand for details                |
| 3   | **Content-First**          | Maximize content area, minimize chrome               | Chat bubbles use 85% max-width; sidebar collapses on mobile             |
| 4   | **Consistent Motion**      | Animations follow physics-based easing               | All transitions: 200ms ease-out; streaming text fades in                |
| 5   | **Forgiving Interactions** | Errors are recoverable, actions are reversible       | Delete → undo toast; streaming error → retry button inline              |
| 6   | **Visual Hierarchy**       | Guide attention through size, weight, and color      | User message: bold primary; Assistant: neutral; Metadata: muted         |
| 7   | **Subtle Delight**         | Micro-interactions that reward engagement            | Successful query → subtle sparkle; copy → checkmark morph               |

### 1.2 Visual Language Specifications

```
┌─────────────────────────────────────────────────────────────────────┐
│                        SPACING SYSTEM                               │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│   Base Unit: 4px                                                    │
│                                                                     │
│   spacing-0:  0px      (none)                                       │
│   spacing-1:  4px      (tight inline)                               │
│   spacing-2:  8px      (component internal)                         │
│   spacing-3:  12px     (related elements)                           │
│   spacing-4:  16px     (section padding)                            │
│   spacing-6:  24px     (card padding)                               │
│   spacing-8:  32px     (page margins)                               │
│   spacing-12: 48px     (section separation)                         │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────┐
│                        TYPOGRAPHY SCALE                             │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│   Font: Inter Variable (400-700 weight)                             │
│   Code: JetBrains Mono (400 weight)                                 │
│                                                                     │
│   text-xs:   12px / 16px  (metadata, timestamps)                    │
│   text-sm:   14px / 20px  (secondary text, labels)                  │
│   text-base: 16px / 24px  (body, chat content)                      │
│   text-lg:   18px / 28px  (conversation title)                      │
│   text-xl:   20px / 28px  (page heading)                            │
│   text-2xl:  24px / 32px  (empty state heading)                     │
│                                                                     │
│   prose-width: max 65ch  (optimal reading length)                   │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────┐
│                        COLOR SYSTEM (OKLCH)                         │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│   LIGHT MODE                        DARK MODE                       │
│   ──────────                        ─────────                       │
│   background: oklch(0.99 0 0)       oklch(0.12 0.01 280)            │
│   surface:    oklch(0.97 0 0)       oklch(0.16 0.01 280)            │
│   card:       oklch(1 0 0)          oklch(0.20 0.01 280)            │
│   border:     oklch(0.90 0.01 260)  oklch(0.30 0.02 280)            │
│                                                                     │
│   primary:    oklch(0.55 0.20 260)  oklch(0.70 0.18 260)            │
│   success:    oklch(0.60 0.15 145)  oklch(0.70 0.14 145)            │
│   warning:    oklch(0.75 0.15 85)   oklch(0.80 0.14 85)             │
│   error:      oklch(0.55 0.20 25)   oklch(0.70 0.18 25)             │
│                                                                     │
│   text-primary:   oklch(0.15 0 0)   oklch(0.95 0 0)                 │
│   text-secondary: oklch(0.45 0 0)   oklch(0.65 0 0)                 │
│   text-muted:     oklch(0.60 0 0)   oklch(0.50 0 0)                 │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### 1.3 Animation Standards

| Animation Type  | Duration | Easing              | Use Case                    |
| --------------- | -------- | ------------------- | --------------------------- |
| Micro-feedback  | 150ms    | ease-out            | Button hover, icon swap     |
| Panel expand    | 200ms    | ease-out            | Sidebar toggle, accordion   |
| Content reveal  | 300ms    | ease-in-out         | Modal open, toast appear    |
| Streaming text  | 100ms    | linear              | Token-by-token fade         |
| Auto-scroll     | 60fps    | spring(1, 0.8, 0.2) | Smooth scroll during stream |
| Loading shimmer | 1500ms   | linear infinite     | Skeleton animation          |

---

## 2. Information Architecture

### 2.1 Query Page Structure

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ ┌─────────────────────┐ ┌─────────────────────────────────────────────────┐ │
│ │                     │ │                    HEADER BAR                   │ │
│ │    HISTORY PANEL    │ │  [Mode Selector] [Graph Stats]    [Settings ⚙] │ │
│ │    (Collapsible)    │ ├─────────────────────────────────────────────────┤ │
│ │                     │ │                                                 │ │
│ │  ┌───────────────┐  │ │                                                 │ │
│ │  │ 🔍 Search     │  │ │                                                 │ │
│ │  └───────────────┘  │ │                  CHAT AREA                      │ │
│ │                     │ │               (Messages Scroll)                 │ │
│ │  📌 Pinned         │ │                                                 │ │
│ │  ├─ Conversation 1 │ │    ┌──────────────────────────────────────┐     │ │
│ │  └─ Conversation 2 │ │    │ 👤 User Message                      │     │ │
│ │                     │ │    │ "What entities are in my graph?"    │     │ │
│ │  📅 Today          │ │    └──────────────────────────────────────┘     │ │
│ │  ├─ Conversation 3 │ │                                                 │ │
│ │  ├─ Conversation 4 │ │    ┌──────────────────────────────────────┐     │ │
│ │  └─ Conversation 5 │ │    │ ✨ Assistant Response                │     │ │
│ │                     │ │    │ [Rendered Markdown Content]          │     │ │
│ │  📅 Yesterday      │ │    │ ───────────────────────────          │     │ │
│ │  └─ Conversation 6 │ │    │ 📊 local • 234 tokens • 1.2s        │     │ │
│ │                     │ │    └──────────────────────────────────────┘     │ │
│ │  📅 Last Week      │ │                                                 │ │
│ │  └─ ... (virtual)  │ │                                                 │ │
│ │                     │ ├─────────────────────────────────────────────────┤ │
│ └─────────────────────┘ │  ┌───────────────────────────────────────────┐  │ │
│     ↕ 280px (min)       │  │ 📝 Ask anything about your knowledge...   │  │ │
│     ↕ 400px (max)       │  │                                   [Send ➤]│  │ │
│                         │  └───────────────────────────────────────────┘  │ │
│                         └─────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 2.2 History Panel Metadata Display

| Level           | Visible Information                                  | Expand Reveals                   |
| --------------- | ---------------------------------------------------- | -------------------------------- |
| **List Item**   | Title (truncated 30ch), Time relative, Message count | —                                |
| **Hover State** | + Full title tooltip, + First message preview        | —                                |
| **Expanded**    | + Mode badge, + Folder path, + Actions dropdown      | Edit title, Pin, Archive, Delete |

### 2.3 Conversation Grouping Taxonomy

```
TEMPORAL GROUPS (Default View)
├── 📌 Pinned (always top)
├── 📅 Today
├── 📅 Yesterday
├── 📅 This Week
├── 📅 This Month
├── 📅 Older (collapsed by default)
└── 📦 Archived (hidden, filterable)

FOLDER VIEW (Alternative)
├── 📁 Folder 1
│   ├── Conversation A
│   └── Conversation B
├── 📁 Folder 2
│   └── Conversation C
└── 📄 Unfiled
    └── Conversation D

FILTER OPTIONS
├── Mode: [All] [Local] [Global] [Hybrid] [Naive]
├── Date: [From] ─── [To]
├── Archived: [Show] [Hide] [Only]
└── Search: [Full-text query]

SORT OPTIONS
├── Updated (default)
├── Created
├── Title A-Z
└── Title Z-A
```

---

## 3. Interaction Design

### 3.1 Streaming Markdown Behavior

```
┌─────────────────────────────────────────────────────────────────────┐
│                    STREAMING STATES TIMELINE                        │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  T+0ms      T+100ms    T+300ms    T+500ms    T+2000ms    T+Done    │
│    │           │          │          │           │          │       │
│    ▼           ▼          ▼          ▼           ▼          ▼       │
│ ┌─────┐   ┌─────────┐ ┌────────┐ ┌─────────┐ ┌─────────┐ ┌──────┐  │
│ │Think│   │Skeleton │ │ First  │ │ Content │ │ Content │ │ Done │  │
│ │ ing │   │ Pulse   │ │ Token  │ │Streaming│ │ Growing │ │ Fade │  │
│ │Dots │   │ + Brain │ │ Fades  │ │ + Cursor│ │ + Scroll│ │Cursor│  │
│ └─────┘   └─────────┘ └────────┘ └─────────┘ └─────────┘ └──────┘  │
│                                                                     │
│  BEHAVIOR RULES:                                                    │
│  ─────────────────                                                  │
│  1. Show "Thinking..." immediately after submit                     │
│  2. Transition to skeleton after 100ms of no content               │
│  3. First token triggers fade-in of content area                   │
│  4. Auto-scroll follows content if user is at bottom               │
│  5. User scroll-up disables auto-scroll (re-enables at bottom)     │
│  6. Cursor blinks at 800ms interval during streaming               │
│  7. Cursor fades out over 200ms on completion                      │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### 3.2 Complex Element Buffering Strategy

````
PROBLEM: Incomplete markdown structures break rendering during streaming

SOLUTION: Element-type-specific buffering

┌─────────────────────────────────────────────────────────────────────┐
│                      BUFFERING RULES                                │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ELEMENT TYPE    │ BUFFER UNTIL                 │ FALLBACK          │
│  ────────────────┼──────────────────────────────┼─────────────────  │
│  Regular text    │ No buffering                 │ —                 │
│  Bold/Italic     │ Closing marker found         │ Render as text    │
│  Code span       │ Closing backtick             │ Render as text    │
│  Code block      │ Closing ```                  │ Skeleton block    │
│  Table           │ Complete row (│...│\n)       │ Skeleton table    │
│  Math inline     │ Closing $                    │ Render as text    │
│  Math block      │ Closing $$                   │ Skeleton block    │
│  Mermaid         │ Closing ``` + valid syntax   │ "Rendering..."    │
│  List            │ Next non-list element        │ Partial render    │
│  Blockquote      │ Next non-quote element       │ Partial render    │
│                                                                     │
│  IMPLEMENTATION:                                                    │
│  ─────────────────                                                  │
│  1. Tokenize incoming content with marked.lexer()                  │
│  2. Check last token for incomplete state                          │
│  3. If incomplete, hold in buffer, show skeleton                   │
│  4. Once complete, render with transition                          │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
````

### 3.3 Pagination & Infinite Scroll Patterns

```
┌─────────────────────────────────────────────────────────────────────┐
│                    CONVERSATION LIST PAGINATION                     │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  STRATEGY: Cursor-based with virtual scrolling                      │
│                                                                     │
│  ┌─────────────────────────────────────────┐                        │
│  │ Conversation 1                          │ ◄── Visible Window    │
│  │ Conversation 2                          │                        │
│  │ Conversation 3                          │                        │
│  │ Conversation 4                          │                        │
│  │ Conversation 5                          │                        │
│  │ ────────────────────────────────────────│ ◄── Trigger: 80% scroll│
│  │ 🔄 Loading more...                      │                        │
│  └─────────────────────────────────────────┘                        │
│                                                                     │
│  PARAMETERS:                                                        │
│  ─────────────                                                      │
│  • Page size: 20 conversations                                      │
│  • Prefetch: Load next page at 80% scroll                          │
│  • Max cached: 100 conversations in memory                         │
│  • Virtualizer: @tanstack/react-virtual                            │
│                                                                     │
│  API CALL PATTERN:                                                  │
│  GET /conversations?cursor={last_id}&limit=20&sort=updated_at      │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────┐
│                    MESSAGE HISTORY PAGINATION                       │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  STRATEGY: Reverse infinite scroll (newest at bottom)              │
│                                                                     │
│  ┌─────────────────────────────────────────┐                        │
│  │ ⬆️ Load earlier messages               │ ◄── Trigger: scroll top │
│  │ ────────────────────────────────────────│                        │
│  │ [Message 1] First visible              │                        │
│  │ [Message 2]                            │ ◄── Visible Window     │
│  │ [Message 3]                            │                        │
│  │ [Message 4]                            │                        │
│  │ [Message 5] Most recent                │                        │
│  └─────────────────────────────────────────┘                        │
│                                                                     │
│  PARAMETERS:                                                        │
│  ─────────────                                                      │
│  • Initial load: 50 messages                                       │
│  • Load more: 30 messages per fetch                                │
│  • Maintain scroll position on prepend                             │
│  • No upper limit (conversations can be very long)                 │
│                                                                     │
│  API CALL PATTERN:                                                  │
│  GET /conversations/{id}/messages?cursor={first_id}&limit=30       │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### 3.4 Filter & Sort Interaction Design

```
┌─────────────────────────────────────────────────────────────────────┐
│                    FILTER INTERACTION PATTERN                       │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  TYPE: Immediate (no "Apply" button)                                │
│                                                                     │
│  ┌─────────────────────────────────────────┐                        │
│  │ 🔍 Search conversations...              │ ◄── Debounced 300ms   │
│  └─────────────────────────────────────────┘                        │
│                                                                     │
│  ┌─────────────────────────────────────────┐                        │
│  │ Mode:    [All ▾] [Local] [Global] [Hyb] │ ◄── Toggle chips      │
│  └─────────────────────────────────────────┘                        │
│                                                                     │
│  ┌─────────────────────────────────────────┐                        │
│  │ Sort:    Updated ▾   │  Order: ↓ Desc   │ ◄── Dropdown + toggle │
│  └─────────────────────────────────────────┘                        │
│                                                                     │
│  ACTIVE FILTERS DISPLAY:                                            │
│  ─────────────────────────                                          │
│  When filters active, show pill summary:                            │
│                                                                     │
│  ┌─────────────────────────────────────────┐                        │
│  │ Filters: [local ×] [this week ×]  Clear │                        │
│  └─────────────────────────────────────────┘                        │
│                                                                     │
│  EMPTY STATE (no results):                                          │
│  ────────────────────────                                           │
│  "No conversations match your filters"                              │
│  [Clear filters] button                                             │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 4. Component Design Specifications

### 4.1 Message Bubble Anatomy

````
USER MESSAGE BUBBLE
───────────────────
┌──────────────────────────────────────────────────────┐
│                                                      │
│  "What are the key relationships between Sarah      │
│   and the research team?"                           │
│                                                      │
└──────────────────────────────────────────────────────┘
                                              ┌────────┐
                                              │ Avatar │
                                              └────────┘
STYLES:
- Background: primary gradient (from-primary to-primary/90)
- Text: primary-foreground
- Border-radius: 16px (rounded-2xl rounded-tr-sm)
- Max-width: 85%
- Padding: 12px 16px (py-3 px-4)
- Shadow: subtle (0_2px_8px_rgba(0,0,0,0.08))


ASSISTANT MESSAGE BUBBLE
────────────────────────
┌────────┐
│ Avatar │
└────────┘
┌──────────────────────────────────────────────────────────────────┐
│  💭 THINKING (collapsible)                           ▼ 1.2s     │
├──────────────────────────────────────────────────────────────────┤
│  │ "Let me analyze the relationships in the graph..."           │
│  │ "Looking at entities: SARAH, RESEARCH_TEAM..."               │
└──────────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────────┐
│                                                                  │
│  ## Key Relationships                                            │
│                                                                  │
│  Sarah Chen has the following connections:                       │
│                                                                  │
│  | Entity | Relationship | Strength |                           │
│  |--------|-------------|----------|                            │
│  | Dr. Lee | Collaborates | 0.92    |                           │
│  | AI Lab  | Member of    | 0.88    |                           │
│                                                                  │
│  ```mermaid                                                      │
│  graph LR                                                        │
│      SARAH --> DR_LEE                                            │
│      SARAH --> AI_LAB                                            │
│  ```                                                             │
│                                                                  │
├──────────────────────────────────────────────────────────────────┤
│  📊 hybrid • 458 tokens • 2.3s         [📋 Copy] [🔄 Regenerate] │
└──────────────────────────────────────────────────────────────────┘

STYLES:
- Background: card (bg-card)
- Border: subtle (border rounded-2xl rounded-tl-sm)
- Markdown prose styles with dark mode overrides
- Metadata bar: muted, appears on hover
````

### 4.2 Empty State Design

```
┌─────────────────────────────────────────────────────────────────────┐
│                                                                     │
│                     ╭──────────────────────╮                        │
│                     │   ✨ (gradient glow) │                        │
│                     │   ⬜ (sparkle icon)  │                        │
│                     ╰──────────────────────╯                        │
│                                                                     │
│                  Ask about your knowledge graph                     │
│                                                                     │
│      I can help you explore entities, find connections,            │
│        and uncover insights from your documents.                   │
│                                                                     │
│     ┌──────────────────────────────────────────────────┐           │
│     │  🟢 42 entities  •  🟡 156 relationships  •  🔵 8 types   │   │
│     └──────────────────────────────────────────────────┘           │
│                                                                     │
│                        Try asking:                                  │
│                                                                     │
│  ┌────────────────────────────┐ ┌────────────────────────────┐     │
│  │ 🔍 What are the main       │ │ 💡 Summarize the key       │     │
│  │    entities in my graph?   │ │    relationships          │     │
│  └────────────────────────────┘ └────────────────────────────┘     │
│  ┌────────────────────────────┐ ┌────────────────────────────┐     │
│  │ 🌿 Find connections        │ │ 📖 What topics are        │     │
│  │    between people          │ │    covered?               │     │
│  └────────────────────────────┘ └────────────────────────────┘     │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### 4.3 Loading States Catalog

| State                     | Visual                                    | Duration        | Transition In | Transition Out  |
| ------------------------- | ----------------------------------------- | --------------- | ------------- | --------------- |
| **Thinking**              | Brain icon + animated dots + shimmer bars | 100ms+          | Fade in       | Fade to content |
| **Generating**            | Content + blinking cursor                 | Variable        | From thinking | Cursor fade     |
| **Loading conversations** | Skeleton list items                       | 100ms+          | Instant       | Fade out        |
| **Loading conversation**  | Skeleton messages                         | 100ms+          | Instant       | Fade out        |
| **Sending message**       | User bubble + thinking state              | 0ms             | Instant add   | —               |
| **Error**                 | Error icon + message + retry              | Until dismissed | Fade in       | Slide out       |

---

## 5. Responsive Design Strategy

### 5.1 Breakpoint Behavior

| Breakpoint  | Width       | History Panel          | Layout                       |
| ----------- | ----------- | ---------------------- | ---------------------------- |
| **Mobile**  | <640px      | Hidden (sheet trigger) | Single column                |
| **Tablet**  | 640-1024px  | Collapsed (icon rail)  | Single column                |
| **Desktop** | 1024-1440px | Expanded (280px)       | Two column                   |
| **Wide**    | >1440px     | Expanded (320px)       | Two column, centered content |

### 5.2 Mobile History Panel

```
┌──────────────────────────────────────┐
│ [≡]  New Chat                   [+]  │  ◄── Header with menu trigger
├──────────────────────────────────────┤
│                                      │
│        (Chat Area Full Width)        │
│                                      │
├──────────────────────────────────────┤
│ [📝 Type your message...]      [➤]   │  ◄── Sticky input
└──────────────────────────────────────┘

MENU SHEET (slide from left):
┌──────────────────────────────────────┐
│ [←]  History                         │
├──────────────────────────────────────┤
│ 🔍 Search...                         │
├──────────────────────────────────────┤
│ 📌 Pinned                            │
│ ├─ Research Discussion               │
│ └─ Project Alpha Analysis            │
├──────────────────────────────────────┤
│ 📅 Today                             │
│ ├─ Entity exploration                │
│ └─ Graph structure query             │
├──────────────────────────────────────┤
│ [Show archived conversations]        │
└──────────────────────────────────────┘
```

---

## 6. Accessibility Considerations

### 6.1 Keyboard Navigation

| Key           | Context           | Action              |
| ------------- | ----------------- | ------------------- |
| `Enter`       | Input area        | Submit query        |
| `Shift+Enter` | Input area        | New line            |
| `Escape`      | Streaming         | Stop generation     |
| `↑`/`↓`       | Conversation list | Navigate items      |
| `Enter`       | Conversation item | Select conversation |
| `Cmd+K`       | Anywhere          | Open search         |
| `Cmd+N`       | Anywhere          | New conversation    |
| `Cmd+C`       | Message focused   | Copy content        |

### 6.2 Screen Reader Announcements

| Event               | Announcement                                         |
| ------------------- | ---------------------------------------------------- |
| Query submitted     | "Sending query. Please wait."                        |
| Response started    | "Receiving response."                                |
| Response complete   | "Response complete. {token_count} tokens generated." |
| Error occurred      | "Error: {message}. Press Enter to retry."            |
| Conversation loaded | "Loaded {title}. {count} messages."                  |

---

## References

- [Audit Findings](./01_audit_findings.md)
- [Technical Specification](./03_technical_spec.md)
- [Implementation Roadmap](./04_implementation_roadmap.md)
- [Design Mockups](./05_design_mockups.md)

---

_Document Version: 1.0 | Last Updated: December 27, 2025_
