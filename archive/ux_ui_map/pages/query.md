# Page: Query

## Overview

- **Route**: `/query`
- **Title**: "Requête" (Query)
- **Layout**: Split layout with main query area and conversation history sidebar
- **Source File**: [src/app/(dashboard)/query/page.tsx](../../edgequake_webui/src/app/(dashboard)/query/page.tsx)

## Layout Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│ ┌───────────────┬─────────────────────────────────────────────────┐ │
│ │               │ Header (64px)                                   │ │
│ │               ├─────────────────────────────────────────────────┤ │
│ │   Sidebar     │ Breadcrumb: EdgeQuake > Query                   │ │
│ │   (64px)      ├───────────────────────────────┬─────────────────┤ │
│ │               │                               │                 │ │
│ │   Nav Icons   │ Page Header                   │ Historique      │ │
│ │               │ "Requête" + mode selector     │ (History Panel) │ │
│ │               │                               │ 320px           │ │
│ │               │ ┌───────────────────────────┐ │                 │ │
│ │               │ │ Empty State / Chat Area   │ │ - Search input  │ │
│ │               │ │                           │ │ - Conversation  │ │
│ │               │ │ - Welcome message         │ │   list          │ │
│ │               │ │ - "Try asking:" prompts   │ │                 │ │
│ │               │ │                           │ │                 │ │
│ │               │ └───────────────────────────┘ │                 │ │
│ │               │                               │                 │ │
│ │               │ ┌───────────────────────────┐ │                 │ │
│ │               │ │ Input Area (fixed bottom) │ │                 │ │
│ │               │ │ [Textarea] [Send]         │ │                 │ │
│ │               │ └───────────────────────────┘ │                 │ │
│ └───────────────┴───────────────────────────────┴─────────────────┘ │
└─────────────────────────────────────────────────────────────────────┘
```

## Screenshots

| Viewport | Screenshot |
|----------|------------|
| Desktop (1440px) | [query-desktop.png](../screenshots/query/query-desktop.png) |
| Tablet (768px) | [query-tablet.png](../screenshots/query/query-tablet.png) |
| Mobile (375px) | [query-mobile.png](../screenshots/query/query-mobile.png) |

---

## Region: Page Header

- **Position**: Top of main content area
- **Layout**: Flex row, space-between alignment
- **Content**: Title + subtitle on left, actions on right

### Container: Title Block

- **Content**: 
  - H1: "Requête" (24px, bold)
  - Subtitle: "Posez des questions sur votre graphe de connaissances"

### Container: Actions Bar

- **Position**: Right side of header
- **Layout**: Flex row with gap

#### Component: New Button

- **Type**: Button, outline variant
- **Icon**: Plus icon (left)
- **Text**: "New"
- **Function**: Creates new conversation

#### Component: Query Mode Selector

- **Type**: Segmented button group
- **Source File**: [src/components/query/query-mode-selector.tsx](../../edgequake_webui/src/components/query/query-mode-selector.tsx)
- **Options**:
  - Local (circle dot icon)
  - Global (globe icon)
  - Hybrid (toggle icon) - default/pressed state
  - Simple (sparkles icon)
- **Styling**: 
  - Height: 36px
  - Border: 1px solid border
  - Border-radius: rounded-md
  - Selected: bg-accent, pressed state

#### Component: Settings Sheet Trigger

- **Type**: Icon button
- **Icon**: Sliders icon
- **Function**: Opens advanced query settings sheet

---

## Region: Main Chat Area

- **Position**: Center, flexible height
- **Layout**: Flex column with scroll
- **Background**: `var(--background)`

### Container: Empty State

- **Type**: Centered content block
- **Visibility**: Shown when no messages in conversation

#### Component: Welcome Icon

- **Type**: Decorative icon container
- **Dimensions**: 64px × 64px
- **Background**: Gradient (violet-500 to purple-600)
- **Border Radius**: 16px
- **Icon**: Sparkles (animated pulse)

#### Component: Welcome Message

- **Type**: Text block
- **Content**: 
  - H2: "Ask about your knowledge graph" (20px, semibold)
  - Paragraph: Description text (14px, muted-foreground)

#### Component: Document Status Alert

- **Type**: Alert card
- **Background**: Amber/warning style
- **Content**: "No documents yet" + "Upload" link button
- **Border Radius**: 12px

#### Component: Suggested Prompts

- **Type**: 2×2 grid of prompt buttons
- **Layout**: Grid with gap
- **Items**:
  - "What are the main entities in my knowledge graph?"
  - "Summarize the key relationships between documents"
  - "Find connections between people and organizations"
  - "What topics are covered in my documents?"

##### Prompt Button

- **Type**: Button, ghost variant
- **Layout**: Icon + text
- **Border**: 1px solid border
- **Border Radius**: 12px
- **Padding**: 16px
- **Hover**: bg-muted

---

## Region: Input Area

- **Position**: Bottom of main content, fixed
- **Layout**: Flex column
- **Background**: `var(--card)` with border-top
- **Padding**: 16px

### Container: Input Form

- **Type**: Form with textarea
- **Layout**: Flex row

#### Component: Query Textarea

- **Type**: Textarea, auto-resize
- **Placeholder**: "Posez une question..."
- **Height**: Min 40px, auto-expand
- **Border**: 1px solid border
- **Border Radius**: 12px
- **Focus**: Ring 2px primary

#### Component: Send Button

- **Type**: Icon button
- **Icon**: Send icon (arrow)
- **Background**: `var(--primary)` when enabled
- **States**:
  - Disabled: Opacity 50%, cursor not-allowed
  - Enabled: Full opacity, cursor pointer
  - Loading: Stop icon for cancel

### Container: Input Help Text

- **Type**: Helper text
- **Content**: "Press Enter to send, Shift+Enter for new line"
- **Typography**: 12px, muted-foreground

---

## Region: Conversation History Panel

- **Position**: Right sidebar
- **Dimensions**: 320px width
- **Border**: 1px solid border on left
- **Background**: `var(--card)`
- **Source File**: [src/components/query/conversation-history-panel.tsx](../../edgequake_webui/src/components/query/conversation-history-panel.tsx)

### Container: Panel Header

- **Layout**: Flex row, space-between
- **Content**: 
  - H2: "Historique" (16px, semibold)
  - Action buttons (New, Collapse)

### Container: Search Input

- **Type**: Search input with icon
- **Placeholder**: "Search conversations..."
- **Icon**: Search icon (left)
- **Height**: 36px

### Container: Conversation List

- **Type**: Scrollable list
- **Layout**: Vertical stack

#### Component: Conversation Item

- **Type**: Button/clickable row
- **Layout**: Icon + text block + menu button
- **Content**:
  - Chat icon (20px)
  - Title: Chat date or first message excerpt
  - Subtitle: Message count + time
- **States**:
  - Default: Transparent background
  - Hover: bg-muted
  - Selected: bg-accent, pressed state
- **Actions**: "More options" button on hover

---

## Chat Message Components

### Component: User Message

- **Position**: Right-aligned
- **Layout**: Message bubble + avatar
- **Background**: `var(--primary)`
- **Text Color**: `var(--primary-foreground)`
- **Border Radius**: 16px, 4px top-right
- **Max Width**: 85%

### Component: Assistant Message

- **Position**: Left-aligned
- **Layout**: Avatar + message content
- **Avatar**: Purple gradient with Sparkles icon
- **Background**: `var(--card)` with border
- **Border Radius**: 16px, 4px top-left
- **Max Width**: 85%

### Component: Loading Message

- **Type**: Animated skeleton
- **Content**: Brain icon (pulsing) + "Processing..." text + animated dots
- **Shimmer Effect**: CSS animation on skeleton bars

---

## Responsive Behavior

| Breakpoint | Layout | History Panel |
|------------|--------|---------------|
| Mobile (<768px) | Full width, history as overlay | Sheet from right |
| Tablet (768-1024px) | Split 60/40 | Visible, narrower |
| Desktop (>1024px) | Split with 320px sidebar | Full width visible |

---

## Component Cross-References

- [Button](../components/buttons.md) — Send button, action buttons
- [Input](../components/inputs.md) — Query textarea, search input
- [Card](../components/cards.md) — Message bubbles, panel containers
- [Avatar](../components/dialogs.md) — User and assistant avatars
- [Sheet](../components/dialogs.md) — Mobile history panel, settings
- [ScrollArea](../components/navigation.md) — Chat scroll, history scroll
