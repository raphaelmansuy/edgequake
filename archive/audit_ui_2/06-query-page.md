# UI Audit: Query Page

**Screen:** Query Interface Header  
**Date:** 2025-12-25  
**Priority:** High - Primary AI interaction

---

## Screenshot Analysis

Query page header showing:

- Breadcrumb navigation (EdgeQuake > Query)
- Page title "Query"
- Subtitle "Ask questions about your knowledge graph"
- Loading/spinner icon
- Floating action button (purple sparkle icon)

---

## Issues Identified

### Critical Issues

| ID     | Issue                                                                                     | Location          | Severity    |
| ------ | ----------------------------------------------------------------------------------------- | ----------------- | ----------- |
| QRY-01 | **Empty state unclear** - Large blank area with just a spinner, no guidance on what to do | Main content area | 🔴 Critical |
| QRY-02 | **Floating button purpose unclear** - Purple sparkle FAB has no label or context          | Bottom right      | 🔴 Critical |

### High Priority Issues

| ID     | Issue                                                                                          | Location     | Severity |
| ------ | ---------------------------------------------------------------------------------------------- | ------------ | -------- |
| QRY-03 | **Spinner with no context** - Loading indicator (↻) appears but doesn't explain what's loading | Below title  | 🟠 High  |
| QRY-04 | **No input visible** - Query input field not visible in screenshot, may be below fold          | Content area | 🟠 High  |
| QRY-05 | **"+" button purpose unclear** - Plus button in sidebar without label                          | Left sidebar | 🟠 High  |

### Medium Priority Issues

| ID     | Issue                                                                                      | Location      | Severity  |
| ------ | ------------------------------------------------------------------------------------------ | ------------- | --------- |
| QRY-06 | **Breadcrumb spacing** - "EdgeQuake > Query" has tight spacing                             | Breadcrumb    | 🟡 Medium |
| QRY-07 | **Title and subtitle styling** - Could use more visual hierarchy                           | Header        | 🟡 Medium |
| QRY-08 | **No conversation history visible** - If there are past queries, they should be accessible | Sidebar/Panel | 🟡 Medium |

### Low Priority Issues

| ID     | Issue                                                            | Location      | Severity |
| ------ | ---------------------------------------------------------------- | ------------- | -------- |
| QRY-09 | **Spinner icon generic** - Could use branded loading animation   | Loading state | 🟢 Low   |
| QRY-10 | **FAB shadow/elevation** - Floating button could have more depth | FAB           | 🟢 Low   |

---

## Improvement Plan

### Phase 1: Empty State Design (Week 1)

#### 1.1 Guided Empty State

```
Current:
┌─────────────────────────────────────────────────────────────┐
│ Query                                                       │
│ Ask questions about your knowledge graph                    │
│                                                             │
│                         ↻                                   │
│                    (loading...)                             │
│                                                             │
│                                                             │
│                                                             │
│                                                             │
│                                                    [✨]     │
└─────────────────────────────────────────────────────────────┘

Proposed (with data):
┌─────────────────────────────────────────────────────────────┐
│ Query                                            [⚙️] [📋]  │
│ Ask questions about your knowledge graph                    │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│         ╭─────────────────────────────────────────╮         │
│         │                                         │         │
│         │    ✨  Ask about your knowledge graph   │         │
│         │                                         │         │
│         │    Your graph contains:                 │         │
│         │    • 7 entities                         │         │
│         │    • 6 relationships                    │         │
│         │    • 3 entity types                     │         │
│         │                                         │         │
│         ╰─────────────────────────────────────────╯         │
│                                                             │
│    ╭─ Try asking: ─────────────────────────────────────╮    │
│    │ "What entities are related to Nemotron 3?"       │    │
│    │ "Summarize the main technologies in my graph"    │    │
│    │ "How are NVIDIA products connected?"             │    │
│    ╰──────────────────────────────────────────────────╯    │
│                                                             │
├─────────────────────────────────────────────────────────────┤
│ 💬 Ask a question about your knowledge graph...             │
│ ────────────────────────────────────────────────────────────│
│                                              [Send] [Mode ▼]│
└─────────────────────────────────────────────────────────────┘

Proposed (no data):
┌─────────────────────────────────────────────────────────────┐
│ Query                                                       │
│ Ask questions about your knowledge graph                    │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│         ╭─────────────────────────────────────────╮         │
│         │                                         │         │
│         │    📄  No documents yet                 │         │
│         │                                         │         │
│         │    Upload documents to start building   │         │
│         │    your knowledge graph, then ask       │         │
│         │    questions here.                      │         │
│         │                                         │         │
│         │         [📤 Upload Documents]           │         │
│         │                                         │         │
│         ╰─────────────────────────────────────────╯         │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

#### 1.2 Suggested Queries Component

```tsx
const suggestedQueries = [
  {
    icon: <Search className="h-4 w-4" />,
    text: "What entities are related to Nemotron 3?",
    category: "exploration",
  },
  {
    icon: <Lightbulb className="h-4 w-4" />,
    text: "Summarize the main technologies in my graph",
    category: "summary",
  },
  {
    icon: <GitBranch className="h-4 w-4" />,
    text: "How are NVIDIA products connected?",
    category: "relationships",
  },
];

function SuggestedQueries({ onSelect }) {
  return (
    <div className="space-y-3">
      <p className="text-sm font-medium text-muted-foreground">Try asking:</p>
      <div className="flex flex-col gap-2">
        {suggestedQueries.map((query, i) => (
          <Button
            key={i}
            variant="outline"
            className="justify-start text-left h-auto py-3 px-4"
            onClick={() => onSelect(query.text)}
          >
            {query.icon}
            <span className="ml-2">{query.text}</span>
          </Button>
        ))}
      </div>
    </div>
  );
}
```

### Phase 2: FAB Clarity (Week 1)

#### 2.1 Replace FAB with Clear Actions

```
Option A: Remove FAB, use inline actions
┌─────────────────────────────────────────────────────────────┐
│ 💬 Ask a question...                                        │
│ ────────────────────────────────────────────────────────────│
│ [📎 Attach] [🎯 Focus Entity]            [Mode ▼] [Send ➤] │
└─────────────────────────────────────────────────────────────┘

Option B: Keep FAB but with tooltip
┌──────┐
│  ✨  │  ← Tooltip: "New conversation" on hover
└──────┘
     │
     └─ Speed dial on click:
        • 📝 New conversation
        • 📋 View history
        • ⚙️ Query settings
```

#### 2.2 FAB Implementation (if keeping)

```tsx
<TooltipProvider>
  <Tooltip>
    <TooltipTrigger asChild>
      <Button
        size="lg"
        className={cn(
          "fixed bottom-6 right-6 h-14 w-14 rounded-full shadow-lg",
          "bg-gradient-to-br from-violet-500 to-purple-600",
          "hover:from-violet-600 hover:to-purple-700",
          "focus:ring-4 focus:ring-violet-500/50"
        )}
        onClick={handleNewConversation}
      >
        <Sparkles className="h-6 w-6 text-white" />
      </Button>
    </TooltipTrigger>
    <TooltipContent side="left">
      <p>Start new conversation</p>
      <p className="text-xs text-muted-foreground">⌘N</p>
    </TooltipContent>
  </Tooltip>
</TooltipProvider>
```

### Phase 3: Loading States (Week 2)

#### 3.1 Contextual Loading

```tsx
// Loading knowledge graph stats
function LoadingState() {
  return (
    <div className="flex flex-col items-center justify-center py-12 space-y-4">
      <div className="relative">
        <div className="h-12 w-12 rounded-full border-4 border-primary/20" />
        <div className="absolute inset-0 h-12 w-12 rounded-full border-4 border-t-primary animate-spin" />
      </div>
      <div className="text-center space-y-1">
        <p className="text-sm font-medium">Loading your knowledge graph...</p>
        <p className="text-xs text-muted-foreground">
          Preparing AI-powered search
        </p>
      </div>
    </div>
  );
}

// Waiting for AI response
function ThinkingState() {
  return (
    <div className="flex items-start gap-3 p-4 rounded-lg bg-muted/50">
      <Avatar className="h-8 w-8">
        <AvatarFallback className="bg-gradient-to-br from-violet-500 to-purple-600">
          <Sparkles className="h-4 w-4 text-white" />
        </AvatarFallback>
      </Avatar>
      <div className="space-y-2 flex-1">
        <div className="flex items-center gap-2">
          <span className="font-medium text-sm">EdgeQuake AI</span>
          <span className="text-xs text-muted-foreground">thinking...</span>
        </div>
        <div className="flex gap-1">
          <span className="h-2 w-2 rounded-full bg-primary animate-bounce" />
          <span className="h-2 w-2 rounded-full bg-primary animate-bounce delay-100" />
          <span className="h-2 w-2 rounded-full bg-primary animate-bounce delay-200" />
        </div>
      </div>
    </div>
  );
}
```

### Phase 4: Conversation History (Week 2)

#### 4.1 Sidebar History Panel

```
┌─ Conversations ─────────────────────────────────────────────┐
│ [+ New]                                         [🔍 Search] │
├─────────────────────────────────────────────────────────────┤
│ TODAY                                                       │
│ ┌───────────────────────────────────────────────────────┐   │
│ │ ● What entities are related to Nemotron?              │   │
│ │   3 messages · 2 min ago                              │   │
│ └───────────────────────────────────────────────────────┘   │
│                                                             │
│ YESTERDAY                                                   │
│ ┌───────────────────────────────────────────────────────┐   │
│ │ Summarize NVIDIA technologies                         │   │
│ │   5 messages · Dec 24                                 │   │
│ └───────────────────────────────────────────────────────┘   │
│                                                             │
│ LAST WEEK                                                   │
│ ┌───────────────────────────────────────────────────────┐   │
│ │ How are entities connected?                           │   │
│ │   2 messages · Dec 18                                 │   │
│ └───────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

#### 4.2 History Component

```tsx
function ConversationHistory({ conversations, onSelect }) {
  const grouped = groupByDate(conversations);

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="font-semibold">Conversations</h3>
        <Button variant="ghost" size="sm">
          <Plus className="h-4 w-4 mr-1" />
          New
        </Button>
      </div>

      <ScrollArea className="h-[calc(100vh-200px)]">
        {Object.entries(grouped).map(([date, convos]) => (
          <div key={date} className="mb-4">
            <p className="text-xs font-medium text-muted-foreground mb-2 uppercase">
              {date}
            </p>
            <div className="space-y-1">
              {convos.map((convo) => (
                <Button
                  key={convo.id}
                  variant="ghost"
                  className="w-full justify-start h-auto py-2"
                  onClick={() => onSelect(convo)}
                >
                  <div className="text-left">
                    <p className="text-sm font-medium truncate">
                      {convo.title}
                    </p>
                    <p className="text-xs text-muted-foreground">
                      {convo.messageCount} messages ·{" "}
                      {formatRelative(convo.updatedAt)}
                    </p>
                  </div>
                </Button>
              ))}
            </div>
          </div>
        ))}
      </ScrollArea>
    </div>
  );
}
```

### Phase 5: Input Improvements (Week 2)

#### 5.1 Enhanced Query Input

```tsx
function QueryInput({ onSubmit, isLoading }) {
  const [query, setQuery] = useState("");
  const textareaRef = useRef(null);

  return (
    <div className="border-t bg-background p-4">
      <form onSubmit={handleSubmit} className="max-w-3xl mx-auto">
        <div className="relative">
          <Textarea
            ref={textareaRef}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Ask a question about your knowledge graph..."
            className="min-h-[60px] max-h-[200px] resize-none pr-24 py-4"
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                handleSubmit(e);
              }
            }}
          />

          <div className="absolute right-2 bottom-2 flex items-center gap-2">
            <QueryModeSelector />
            <Button
              type="submit"
              size="sm"
              disabled={!query.trim() || isLoading}
            >
              {isLoading ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                <Send className="h-4 w-4" />
              )}
            </Button>
          </div>
        </div>

        <p className="text-xs text-muted-foreground mt-2">
          Press Enter to send, Shift+Enter for new line
        </p>
      </form>
    </div>
  );
}
```

---

## Proposed Page Layout

```tsx
function QueryPage() {
  return (
    <div className="flex h-full">
      {/* Conversation History Sidebar */}
      <aside className="w-72 border-r p-4 hidden lg:block">
        <ConversationHistory
          conversations={conversations}
          onSelect={setActiveConversation}
        />
      </aside>

      {/* Main Query Area */}
      <main className="flex-1 flex flex-col min-h-0">
        {/* Header */}
        <header className="border-b px-6 py-4">
          <div className="flex items-center justify-between">
            <div>
              <h1 className="text-xl font-semibold">Query</h1>
              <p className="text-sm text-muted-foreground">
                Ask questions about your knowledge graph
              </p>
            </div>
            <div className="flex items-center gap-2">
              <Button variant="ghost" size="icon">
                <History className="h-4 w-4" />
              </Button>
              <Button variant="ghost" size="icon">
                <Settings className="h-4 w-4" />
              </Button>
            </div>
          </div>
        </header>

        {/* Messages Area */}
        <div className="flex-1 overflow-y-auto">
          {messages.length === 0 ? (
            <EmptyQueryState
              graphStats={graphStats}
              onSuggestionClick={handleSuggestion}
            />
          ) : (
            <MessageList messages={messages} />
          )}
        </div>

        {/* Input */}
        <QueryInput onSubmit={handleSubmit} isLoading={isLoading} />
      </main>
    </div>
  );
}
```

---

## Accessibility Improvements

1. **Screen Reader:**

   - "Query page. Your knowledge graph has 7 entities and 6 relationships."
   - "Suggested query: What entities are related to Nemotron 3?"
   - Announce when AI response begins and completes

2. **Keyboard Navigation:**

   - ⌘N for new conversation
   - ⌘K for quick command palette
   - Arrow keys to navigate suggestions
   - Enter to send query

3. **Focus Management:**
   - Auto-focus query input on page load
   - Focus latest message after AI response

---

## Success Metrics

| Metric               | Current         | Target                |
| -------------------- | --------------- | --------------------- |
| Empty state guidance | None            | Full onboarding       |
| FAB discoverability  | Unclear         | Labeled + tooltip     |
| Loading context      | Generic spinner | Branded + explanatory |
| Input accessibility  | Basic           | Full keyboard support |
