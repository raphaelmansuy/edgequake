# Query/Search Screen Audit

## Screen: Query Interface (`/query`)

**Screenshot References:**
- [`03-query-initial.png`](../audit_ui/screenshots/03-query-initial.png)
- [`03-query-with-input.png`](../audit_ui/screenshots/03-query-with-input.png)
- [`03-query-with-response.png`](../audit_ui/screenshots/03-query-with-response.png)
- [`03-query-mode-selector.png`](../audit_ui/screenshots/03-query-mode-selector.png)
- [`03-query-right-panel.png`](../audit_ui/screenshots/03-query-right-panel.png)

**Component Files:**
- Page: [`src/app/(dashboard)/query/page.tsx`](../edgequake_webui/src/app/(dashboard)/query/page.tsx)
- Main Component: [`src/components/query/query-interface.tsx`](../edgequake_webui/src/components/query/query-interface.tsx)
- Related: [`src/components/query/`](../edgequake_webui/src/components/query/)

---

## What I Reviewed

### UI Regions Analyzed:
1. **Left Sidebar** - Standard navigation (shared layout)
2. **Chat Area** (Main Content)
   - Message history scroll area
   - User messages (right-aligned)
   - Assistant messages (left-aligned)
   - Loading states
   - COT (Chain of Thought) display
3. **Input Area** (Bottom)
   - Query textarea
   - Mode selector dropdown
   - Send button
   - Settings sheet (Advanced options)
4. **Right Panel** - Source citations and context
   - Collapsible sources section
   - Document chunks
   - Entities
   - Relationships
5. **Empty State** - Initial view before first query

### Measurements:
- Chat area: Takes most of horizontal space
- Input area: Fixed at bottom (~100px height)
- Right panel: ~400px width (when open - detected from screenshot)
- Message max-width: 85% of chat area

---

## Issues

### 🔴 Critical

**C1. Right Panel Not Persistent/Collapsible**
- **Location:** Right panel with source citations
- **Issue:** Based on code review, right panel exists but:
  - Not independently collapsible (user can't hide it to get more chat space)
  - Always visible when context exists (can't toggle on/off)
  - No collapse button or toggle mechanism
- **Impact:** 
  - Wastes space when user doesn't need to see sources
  - Can't maximize chat area for reading long responses
  - Inconsistent with documents page (which lacks right panel entirely)

**C2. Input Area Too Small for Long Queries**
- **Location:** Bottom query textarea
- **Issue:** 
  - Textarea appears to be single or few lines
  - No auto-expand as user types
  - Can't resize manually (no resize handle)
- **Impact:** User can't see full query when typing long questions
- **Evidence:** Common pattern in chat UIs (ChatGPT, Claude) - textarea expands

**C3. No Chat History Persistence**
- **Location:** Message history
- **Issue:** Based on store implementation review:
  - Messages stored in Zustand but no indication of multi-conversation support
  - Clearing messages loses entire history
  - No "conversation threads" or "chat sessions"
- **Impact:** 
  - Can't return to previous conversations
  - Can't compare responses for different queries
  - Lost work if page refreshes

### 🟡 Major

**M1. Empty State Too Minimal**
- **Location:** Initial query page view
- **Issue:** 
  - Just shows input box at bottom
  - No example queries or suggestions
  - No onboarding hints
  - Misses opportunity to guide new users
- **Expected:** Large, centered empty state with:
  - Welcome message
  - Example query cards/buttons
  - Mode explanations
  - Quick start guide

**M2. Mode Selector Buried and Unclear**
- **Location:** Dropdown next to input
- **Issue:** 
  - Small dropdown (screenshot shows ~40px tall)
  - No explanation of modes without opening dropdown
  - No visual differentiation between modes
  - Users don't understand when to use which mode
- **Better UX:** Tab or segmented control with mode descriptions

**M3. Source Citations UX Weak**
- **Location:** Right panel collapsible sections
- **Issue:** 
  - Collapsed by default - users might not notice them
  - "Sources: X chunks · Y entities" text too compact
  - No preview of most relevant source
  - Entities/relationships not visually distinct from chunks
- **Better UX:** 
  - Show top 1-2 most relevant sources expanded by default
  - Use cards/visual hierarchy
  - Highlight most relevant entity mentions

**M4. No Conversation Management**
- **Location:** Missing UI entirely
- **Issue:** 
  - No "New Chat" button
  - No conversation list/history sidebar
  - No way to name or organize chats
  - No conversation search
- **Expected:** Left panel or dropdown with conversation list

**M5. Loading State Not Informative Enough**
- **Location:** While waiting for response
- **Issue:** 
  - LoadingMessage component exists (good!) but:
  - No indication of what step we're in (retrieving, thinking, generating)
  - No estimated time remaining
  - No option to cancel long-running queries
- **Better UX:** Show progressive status (seen in advanced chat UIs)

**M6. Message Actions Limited**
- **Location:** Assistant message actions
- **Issue:** 
  - Only Copy and Regenerate actions
  - No Edit (user message)
  - No Share
  - No Bookmark/Favorite
  - No Export conversation
- **Expected:** More message actions in dropdown

**M7. Settings/Advanced Options Hidden**
- **Location:** Settings sheet (triggered by button)
- **Issue:** 
  - Advanced options (temperature, top_k, etc.) hidden in sheet
  - Users don't know these options exist
  - No indication that query can be customized
- **Better UX:** 
  - Show common settings directly (temperature slider)
  - Use collapsible panel instead of full sheet
  - Add "Advanced" badge or hint

**M8. No Keyboard Shortcuts**
- **Location:** Query interface
- **Issue:** 
  - Enter to send query (probably implemented)
  - But no shortcuts for:
    - New chat
    - Focus input
    - Scroll to bottom
    - Toggle right panel
    - Copy last response
- **Expected:** Comprehensive keyboard navigation

### 🟢 Minor

**m1. User Avatar Generic**
- **Location:** User message avatar
- **Issue:** Just shows generic User icon
- **Recommendation:** Show actual user initials or uploaded avatar

**m2. Timestamp Format Inconsistent**
- **Location:** Message timestamps
- **Issue:** Only shows HH:MM time, not date
- **Problem:** Can't tell if message from today or yesterday
- **Recommendation:** Show relative time ("2 hours ago") with tooltip

**m3. COT Thinking Section Styling**
- **Location:** Chain-of-thought reasoning display
- **Issue:** 
  - Purple color scheme might not match overall design
  - Border styling different from rest of UI
- **Recommendation:** Use theme colors (primary/muted) instead of hardcoded purple

**m4. No Markdown Rendering Preview**
- **Location:** User input area
- **Issue:** User types raw markdown but can't preview before sending
- **Recommendation:** Add "Preview" tab or side-by-side view

**m5. Source Citations Don't Link to Graph**
- **Location:** Entity mentions in right panel
- **Issue:** Entities shown but no way to view them in graph visualization
- **Recommendation:** Add "View in Graph" link for each entity

**m6. No Query Suggestions/Autocomplete**
- **Location:** Input textarea
- **Issue:** No suggestions based on past queries or common patterns
- **Recommendation:** Show query suggestions as user types

**m7. Response Doesn't Highlight Cited Sources**
- **Location:** Assistant response text
- **Issue:** Response text doesn't indicate which sources it used
- **Recommendation:** Add superscript citations [1] [2] that link to sources in right panel

**m8. No Mobile Optimization Evident**
- **Location:** Overall chat layout
- **Issue:** Two-panel layout won't work well on mobile
- **Recommendation:** Stack panels on mobile, make right panel a bottom sheet

**m9. Settings Sheet Too Many Options at Once**
- **Location:** Advanced settings sheet
- **Issue:** All parameters shown at once (temperature, top_k, top_p, etc.)
- **Recommendation:** Group into sections (Response Quality, Performance, Context)

**m10. No Export Chat Option**
- **Location:** Chat history
- **Issue:** Can't export conversation as PDF, MD, or JSON
- **Recommendation:** Add "Export" button in header

---

## Recommendations

### For Right Panel

**R1. Make Right Panel Independently Collapsible** ⭐ **PRIORITY**
```tsx
// Add collapse button in right panel header
<div className="flex items-center justify-between p-3 border-b">
  <h3 className="text-sm font-medium">Sources & Context</h3>
  <Button 
    variant="ghost" 
    size="sm" 
    onClick={onTogglePanel}
    aria-label="Collapse panel"
  >
    <ChevronRight className="h-4 w-4" />
  </Button>
</div>
```

**Default State:**
- Open when query has context/sources
- Collapsed when no context (empty)
- State persists across queries

**Keyboard Shortcut:** `Ctrl/Cmd + B` to toggle

**R2. Improve Source Citations Display**
```tsx
<div className="space-y-4 p-3">
  {/* Top Source - Always Visible */}
  <Card className="border-primary/20 bg-primary/5">
    <CardHeader className="pb-2">
      <div className="flex items-center justify-between">
        <Badge variant="outline">Most Relevant</Badge>
        <span className="text-xs text-muted-foreground">98% match</span>
      </div>
    </CardHeader>
    <CardContent>
      <p className="text-sm line-clamp-3">{topChunk.content}</p>
      <div className="flex items-center gap-2 mt-2">
        <Button variant="link" size="sm" className="h-auto p-0">
          View Document
        </Button>
      </div>
    </CardContent>
  </Card>

  {/* Other Sources - Collapsed */}
  <Collapsible>
    <CollapsibleTrigger asChild>
      <Button variant="ghost" size="sm" className="w-full">
        <ChevronDown className="h-4 w-4 mr-2" />
        View {otherSources.length} More Sources
      </Button>
    </CollapsibleTrigger>
    <CollapsibleContent className="space-y-2">
      {/* Other source cards */}
    </CollapsibleContent>
  </Collapsible>

  {/* Entities Section */}
  <Separator />
  <div>
    <h4 className="text-sm font-medium mb-2">Related Entities</h4>
    <div className="flex flex-wrap gap-2">
      {entities.map(entity => (
        <Badge 
          key={entity.id} 
          variant="secondary"
          className="cursor-pointer hover:bg-primary/10"
          onClick={() => handleEntityClick(entity.id)}
        >
          {entity.name}
          <ExternalLink className="h-3 w-3 ml-1" />
        </Badge>
      ))}
    </div>
  </div>
</div>
```

### For Input Area

**R3. Auto-Expanding Textarea** ⭐ **PRIORITY**
```tsx
<Textarea
  ref={textareaRef}
  value={query}
  onChange={(e) => {
    setQuery(e.target.value);
    // Auto-expand
    e.target.style.height = 'auto';
    e.target.style.height = `${Math.min(e.target.scrollHeight, 200)}px`;
  }}
  placeholder="Ask a question about your documents..."
  className="min-h-[52px] max-h-[200px] resize-none"
  rows={1}
  onKeyDown={(e) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  }}
/>
```

**Features:**
- Starts at ~52px (1 line)
- Expands as user types
- Max height 200px (then scrolls)
- Shift+Enter for new line
- Enter to send

**R4. Mode Selector as Segmented Control**
```tsx
<div className="flex gap-1 p-1 bg-muted rounded-lg mb-3">
  {modes.map(mode => (
    <button
      key={mode.value}
      onClick={() => setMode(mode.value)}
      className={cn(
        "flex-1 px-3 py-2 rounded-md text-sm font-medium transition-colors",
        selectedMode === mode.value
          ? "bg-background shadow-sm text-foreground"
          : "text-muted-foreground hover:text-foreground"
      )}
    >
      <mode.icon className="h-4 w-4 inline-block mr-1.5" />
      {mode.label}
    </button>
  ))}
</div>

{/* Mode description below segmented control */}
<p className="text-xs text-muted-foreground mb-2">
  {modes.find(m => m.value === selectedMode)?.description}
</p>
```

**Modes:**
- 🔍 **Search** - Quick keyword search
- 🧠 **Understand** - Deep semantic analysis (default)
- ⚡ **Fast** - Quick response, less context
- 🎯 **Precise** - Exact citations, no inference

**R5. Add Common Settings to Input Area**
```tsx
<div className="flex items-center gap-2 mb-2">
  <div className="flex items-center gap-2 flex-1">
    <Thermometer className="h-4 w-4 text-muted-foreground" />
    <Label className="text-xs">Temperature:</Label>
    <Slider
      value={[temperature]}
      onValueChange={([v]) => setTemperature(v)}
      min={0}
      max={1}
      step={0.1}
      className="w-24"
    />
    <span className="text-xs text-muted-foreground w-8">{temperature}</span>
  </div>
  
  <Button 
    variant="ghost" 
    size="sm"
    onClick={() => setShowAdvanced(!showAdvanced)}
  >
    <Settings2 className="h-4 w-4 mr-1" />
    Advanced
  </Button>
</div>
```

### For Empty State

**R6. Rich Empty State** ⭐ **PRIORITY**
```tsx
<div className="flex-1 flex items-center justify-center">
  <div className="max-w-2xl mx-auto px-6 text-center space-y-8">
    {/* Hero Section */}
    <div className="space-y-3">
      <div className="inline-flex h-16 w-16 items-center justify-center rounded-2xl bg-gradient-to-br from-primary/20 to-primary/5 mb-4">
        <MessageSquare className="h-8 w-8 text-primary" />
      </div>
      <h2 className="text-3xl font-bold">Ask Anything</h2>
      <p className="text-lg text-muted-foreground max-w-md mx-auto">
        Search your documents using natural language. I'll find relevant information and explain it clearly.
      </p>
    </div>

    {/* Example Queries */}
    <div className="space-y-3">
      <p className="text-sm font-medium text-muted-foreground">Try asking:</p>
      <div className="grid grid-cols-2 gap-3">
        {exampleQueries.map((example, idx) => (
          <Button
            key={idx}
            variant="outline"
            className="h-auto py-4 px-4 text-left justify-start hover:border-primary hover:bg-primary/5"
            onClick={() => setQuery(example.query)}
          >
            <example.icon className="h-5 w-5 mr-3 shrink-0 text-primary" />
            <div className="space-y-1 text-sm">
              <p className="font-medium">{example.title}</p>
              <p className="text-xs text-muted-foreground">{example.query}</p>
            </div>
          </Button>
        ))}
      </div>
    </div>

    {/* Mode Info */}
    <div className="flex items-center justify-center gap-6 text-sm text-muted-foreground">
      <div className="flex items-center gap-2">
        <Brain className="h-4 w-4" />
        <span>Semantic Search</span>
      </div>
      <div className="flex items-center gap-2">
        <Zap className="h-4 w-4" />
        <span>Real-time Responses</span>
      </div>
      <div className="flex items-center gap-2">
        <FileText className="h-4 w-4" />
        <span>Source Citations</span>
      </div>
    </div>
  </div>
</div>
```

**Example Queries:**
```tsx
const exampleQueries = [
  {
    icon: FileText,
    title: "Summarize a topic",
    query: "What are the main features of EdgeQuake?"
  },
  {
    icon: Search,
    title: "Find information",
    query: "How do I configure the LLM provider?"
  },
  {
    icon: Brain,
    title: "Compare concepts",
    query: "What's the difference between local and naive search?"
  },
  {
    icon: Network,
    title: "Explore relationships",
    query: "Show me how documents are connected"
  },
];
```

### For Conversation Management

**R7. Add Conversation History** ⭐ **PRIORITY**

**Option A: Sidebar Drawer**
```tsx
<Sheet>
  <SheetTrigger asChild>
    <Button variant="outline" size="sm">
      <History className="h-4 w-4 mr-2" />
      History
    </Button>
  </SheetTrigger>
  <SheetContent side="left" className="w-80">
    <SheetHeader>
      <SheetTitle>Conversation History</SheetTitle>
      <SheetDescription>
        View and manage your past conversations
      </SheetDescription>
    </SheetHeader>
    <div className="mt-6 space-y-2">
      <Button 
        className="w-full justify-start" 
        variant="outline"
        onClick={handleNewChat}
      >
        <Plus className="h-4 w-4 mr-2" />
        New Conversation
      </Button>
      
      <Separator className="my-4" />
      
      <ScrollArea className="h-[calc(100vh-200px)]">
        <div className="space-y-1">
          {conversations.map(conv => (
            <button
              key={conv.id}
              onClick={() => loadConversation(conv.id)}
              className={cn(
                "w-full text-left px-3 py-2 rounded-lg hover:bg-muted",
                currentConvId === conv.id && "bg-muted"
              )}
            >
              <p className="text-sm font-medium truncate">{conv.title}</p>
              <p className="text-xs text-muted-foreground">{conv.timestamp}</p>
            </button>
          ))}
        </div>
      </ScrollArea>
    </div>
  </SheetContent>
</Sheet>
```

**Option B: Collapsible Left Section** (Like ChatGPT)
- Push main sidebar slightly left
- Show conversation list between main sidebar and chat
- Can collapse to give more chat space

**R8. Auto-Title Conversations**
```tsx
// After first message, generate title using LLM or simple heuristic
const generateTitle = (firstQuery: string) => {
  // Option 1: Use first 50 chars
  return firstQuery.slice(0, 50) + (firstQuery.length > 50 ? '...' : '');
  
  // Option 2: Ask LLM to summarize (better)
  // return await queryApi({ 
  //   content: `Summarize this in 5 words: "${firstQuery}"`,
  //   mode: 'fast'
  // });
};
```

### For Loading States

**R9. Progressive Loading Indicator**
```tsx
const [loadingPhase, setLoadingPhase] = useState<LoadingPhase>('retrieving');

<div className="flex items-center gap-3">
  <div className="relative">
    {loadingPhase === 'retrieving' && <Search className="h-5 w-5 text-blue-500 animate-pulse" />}
    {loadingPhase === 'thinking' && <Brain className="h-5 w-5 text-purple-500 animate-pulse" />}
    {loadingPhase === 'generating' && <Sparkles className="h-5 w-5 text-primary animate-pulse" />}
  </div>
  
  <div className="flex-1">
    <p className="text-sm font-medium">
      {phaseLabels[loadingPhase]}
    </p>
    <Progress value={loadingProgress} className="h-1 mt-1" />
  </div>
  
  <Button 
    variant="ghost" 
    size="sm"
    onClick={handleCancel}
  >
    <StopCircle className="h-4 w-4" />
  </Button>
</div>
```

**Phases:**
1. **Retrieving** (0-30%) - Searching documents
2. **Thinking** (30-60%) - COT reasoning (if enabled)
3. **Generating** (60-100%) - LLM generating response

### For Message Actions

**R10. Comprehensive Message Actions**
```tsx
<div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
  <TooltipProvider>
    <Tooltip>
      <TooltipTrigger asChild>
        <Button variant="ghost" size="sm" onClick={handleCopy}>
          {copied ? <Check className="h-3 w-3" /> : <Copy className="h-3 w-3" />}
        </Button>
      </TooltipTrigger>
      <TooltipContent>Copy response</TooltipContent>
    </Tooltip>
  </TooltipProvider>
  
  {isLast && (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button variant="ghost" size="sm" onClick={handleRegenerate}>
          <RefreshCw className="h-3 w-3" />
        </Button>
      </TooltipTrigger>
      <TooltipContent>Regenerate response</TooltipContent>
    </Tooltip>
  )}
  
  <Tooltip>
    <TooltipTrigger asChild>
      <Button variant="ghost" size="sm" onClick={handleFavorite}>
        <Star className={cn("h-3 w-3", isFavorite && "fill-current")} />
      </Button>
    </TooltipTrigger>
    <TooltipContent>Bookmark this exchange</TooltipContent>
  </Tooltip>
  
  <DropdownMenu>
    <DropdownMenuTrigger asChild>
      <Button variant="ghost" size="sm">
        <MoreVertical className="h-3 w-3" />
      </Button>
    </DropdownMenuTrigger>
    <DropdownMenuContent align="end">
      <DropdownMenuItem onClick={handleEdit}>
        <Edit className="h-3 w-3 mr-2" />
        Edit query
      </DropdownMenuItem>
      <DropdownMenuItem onClick={handleShare}>
        <Share className="h-3 w-3 mr-2" />
        Share exchange
      </DropdownMenuItem>
      <DropdownMenuItem onClick={handleViewInGraph}>
        <Network className="h-3 w-3 mr-2" />
        View entities in graph
      </DropdownMenuItem>
      <DropdownMenuSeparator />
      <DropdownMenuItem onClick={handleDelete} className="text-destructive">
        <Trash2 className="h-3 w-3 mr-2" />
        Delete exchange
      </DropdownMenuItem>
    </DropdownMenuContent>
  </DropdownMenu>
</div>
```

---

## Rationale

### Why Collapsible Right Panel
- **Flexibility:** User controls their workspace
- **Focus:** Can hide when not needed, expand when investigating sources
- **Reading long responses:** More horizontal space for text
- **Consistency:** Matches pattern from other pages

### Why Auto-Expanding Textarea
- **Usability:** See full query as you type
- **Standard pattern:** Used by ChatGPT, Claude, Perplexity, etc.
- **Better UX:** No manual resizing needed
- **Accessibility:** Easier for users with visual impairments

### Why Rich Empty State
- **Onboarding:** Helps new users understand what they can do
- **Discoverability:** Shows features they might not know about
- **Efficiency:** One-click example queries speed up first interaction
- **Professionalism:** Empty white space feels unfinished

### Why Conversation Management
- **Context switching:** Users work on multiple topics/projects
- **Reference:** Return to previous conversations for comparison
- **Organization:** Find past answers without re-querying
- **Standard expectation:** All modern chat UIs have this (ChatGPT, Claude, etc.)

### Why Progressive Loading States
- **Transparency:** User knows system is working and what it's doing
- **Perceived performance:** Feels faster when you see progress
- **Control:** Can cancel if taking too long
- **Trust:** Clear indication of processing builds confidence

---

## Acceptance Criteria

### AC1: Collapsible Right Panel
- [ ] Right panel has collapse button (ChevronRight icon)
- [ ] Panel slides out smoothly (200ms transition)
- [ ] Collapsed state shows only thin bar with "Sources" label (vertically)
- [ ] Click bar or press Ctrl+B to expand
- [ ] State persists across page navigation
- [ ] When collapsed, chat area expands to full width
- [ ] Mobile: Right panel becomes bottom sheet

### AC2: Auto-Expanding Textarea
- [ ] Input starts at 52px height (single line)
- [ ] Expands automatically as user types
- [ ] Max height 200px, then scrollbar appears
- [ ] Shift+Enter adds new line
- [ ] Enter (without Shift) sends query
- [ ] Smooth height transition (100ms)
- [ ] Maintains cursor position during expansion

### AC3: Mode Selector as Segmented Control
- [ ] Shows 4 modes: Search, Understand, Fast, Precise
- [ ] Each mode has icon + label
- [ ] Active mode has distinct background (shadow)
- [ ] Description text shows below selector
- [ ] Click changes mode immediately
- [ ] Keyboard: Tab to focus, Arrow keys to navigate modes

### AC4: Rich Empty State
- [ ] Large icon (64px) at top
- [ ] "Ask Anything" headline (text-3xl)
- [ ] Descriptive subtitle
- [ ] 4 example query cards (2x2 grid)
- [ ] Clicking example fills input and focuses it (doesn't auto-send)
- [ ] Feature highlights at bottom (Semantic Search, etc.)
- [ ] Responsive: Stack query cards on mobile

### AC5: Conversation History
- [ ] "History" button in header
- [ ] Opens left sidebar sheet with conversation list
- [ ] "New Conversation" button at top
- [ ] Current conversation highlighted
- [ ] Each conversation shows: title, timestamp, message count
- [ ] Click conversation loads it and closes sheet
- [ ] Long-press or right-click shows context menu (Rename, Delete, Export)
- [ ] Search bar at top to filter conversations

### AC6: Auto-Title Conversations
- [ ] First user message becomes title (truncated to 50 chars)
- [ ] Title editable (click to edit inline)
- [ ] Default title format: "Chat from [Date]" if no messages yet
- [ ] Title updates immediately on edit
- [ ] Escape to cancel edit, Enter to save

### AC7: Progressive Loading Indicator
- [ ] Shows current phase: Retrieving → Thinking → Generating
- [ ] Progress bar indicates completion percentage
- [ ] Icon changes per phase (Search, Brain, Sparkles)
- [ ] "Cancel" button appears during loading
- [ ] Canceling shows toast: "Query cancelled"
- [ ] Loading message has animated dots (already implemented ✓)

### AC8: Enhanced Source Citations
- [ ] Top source always visible (not collapsed)
- [ ] Shows relevance score (e.g., "98% match")
- [ ] "View Document" link opens document in Documents page
- [ ] Other sources collapsed by default ("View X More Sources")
- [ ] Entities section separate with badge pills
- [ ] Clicking entity badge:
  - Option A: Filters view to that entity's connections
  - Option B: Opens graph page with entity focused
  - Option C: Shows hover card with entity details

### AC9: Comprehensive Message Actions
- [ ] Actions visible on message hover (opacity transition)
- [ ] Copy button (with checkmark feedback)
- [ ] Regenerate button (only on last assistant message)
- [ ] Bookmark/favorite button (star icon, fills when active)
- [ ] More dropdown with: Edit, Share, View in Graph, Delete
- [ ] Edit: Allows editing user message and re-submitting
- [ ] Share: Generates shareable link or exports exchange
- [ ] View in Graph: Navigates to /graph with related entities

### AC10: Common Settings Inline
- [ ] Temperature slider visible below mode selector
- [ ] Range: 0-1, step 0.1, default 0.7
- [ ] Label shows current value
- [ ] "Advanced" button opens collapsible section (not full sheet)
- [ ] Advanced section shows: top_p, top_k, max_tokens, only_need_context
- [ ] Settings persist per conversation (local storage)

### AC11: Keyboard Shortcuts
- [ ] `Ctrl/Cmd + N` - New conversation
- [ ] `Ctrl/Cmd + K` - Focus input
- [ ] `Ctrl/Cmd + B` - Toggle right panel
- [ ] `Ctrl/Cmd + H` - Toggle conversation history
- [ ] `Ctrl/Cmd + /` - Show keyboard shortcuts help
- [ ] `Esc` - Cancel current query (if loading) or clear input
- [ ] `↑` (in input) - Load previous query from history
- [ ] `↓` (in input) - Navigate forward in query history

### AC12: Export Conversation
- [ ] "Export" button in header or conversation context menu
- [ ] Options: Markdown, PDF, JSON
- [ ] Markdown: Includes timestamps, role labels, sources
- [ ] PDF: Formatted with syntax highlighting
- [ ] JSON: Raw data structure for import/analysis
- [ ] Filename format: `EdgeQuake_Chat_[title]_[date].ext`

---

## ASCII Layout Diagrams

### Default State (Right Panel Open)
```
┌──────────┬─────────────────────────────────────┬──────────────────┐
│ Sidebar  │         Chat Messages               │  Sources &       │
│          │                                     │  Context         │
│  • Nav   │  ┌──────────────────────────────┐  │                  │
│  • Mode  │  │ User: Question about X       │  │  ┌────────────┐  │
│          │  └──────────────────────────────┘  │  │ Most       │  │
│          │                                     │  │ Relevant   │  │
│          │  ┌──────────────────────────────┐  │  │ Source     │  │
│          │  │ 🤖 EdgeQuake                 │  │  └────────────┘  │
│          │  │                              │  │                  │
│          │  │ [▼ Reasoning (2.3s)]         │  │  [More Sources]  │
│          │  │                              │  │                  │
│          │  │ Here's the answer...         │  │  Entities:       │
│          │  │                              │  │  [Tag] [Tag]     │
│          │  │ [Copy] [⭐] [•••]           │  │                  │
│          │  └──────────────────────────────┘  │  [Collapse ›]    │
│          │                                     │                  │
│          │                                     │                  │
│          ├─────────────────────────────────────┤                  │
│          │ [Search][Understand][Fast][Precise] │                  │
│          │                                     │                  │
│          │ 🌡️ Temp: ━━●━━━ 0.7  [Advanced▾]  │                  │
│          │                                     │                  │
│          │ ┌───────────────────────────────┐  │                  │
│          │ │ Ask a question...             │  │                  │
│          │ │                               │  │                  │
│          │ └───────────────────────────────┘  │                  │
│          │                           [Send ↵]  │                  │
└──────────┴─────────────────────────────────────┴──────────────────┘
```

### Right Panel Collapsed
```
┌──────────┬────────────────────────────────────────────────────┬─┐
│ Sidebar  │         Chat Messages (Expanded)                   │S│
│          │                                                    │o│
│  • Nav   │  ┌──────────────────────────────────────────────┐ │u│
│  • Mode  │  │ User: Question about X                       │ │r│
│          │  └──────────────────────────────────────────────┘ │c│
│          │                                                    │e│
│          │  ┌──────────────────────────────────────────────┐ │s│
│          │  │ 🤖 EdgeQuake (12:34 PM)                      │ │ │
│          │  │                                              │ │◀│
│          │  │ [▼ Reasoning (2.3s)]                         │ │ │
│          │  │                                              │ │ │
│          │  │ Here's the answer with more space for text   │ │ │
│          │  │ to display clearly...                        │ │ │
│          │  │                                              │ │ │
│          │  │ [Copy] [⭐] [•••]                           │ │ │
│          │  └──────────────────────────────────────────────┘ │ │
│          │                                                    │ │
│          ├────────────────────────────────────────────────────┤ │
│          │ [Input Area - Same as above]                      │ │
└──────────┴────────────────────────────────────────────────────┴─┘
```

### Empty State
```
┌──────────┬────────────────────────────────────────────────────┐
│ Sidebar  │                                                    │
│          │                   💬 (Large Icon)                  │
│  • Nav   │                                                    │
│  • Mode  │                  Ask Anything                      │
│          │      Search your documents using natural language  │
│          │                                                    │
│          │              Try asking:                           │
│          │    ┌──────────────┐  ┌──────────────┐             │
│          │    │ 📄 Summarize │  │ 🔍 Find Info │             │
│          │    │ What are...  │  │ How do I...  │             │
│          │    └──────────────┘  └──────────────┘             │
│          │    ┌──────────────┐  ┌──────────────┐             │
│          │    │ 🧠 Compare   │  │ 🕸️ Explore   │             │
│          │    │ What's the...│  │ Show me how..│             │
│          │    └──────────────┘  └──────────────┘             │
│          │                                                    │
│          │   🧠 Semantic • ⚡ Real-time • 📄 Citations       │
│          │                                                    │
│          ├────────────────────────────────────────────────────┤
│          │ [Input Area]                                       │
└──────────┴────────────────────────────────────────────────────┘
```

### With Conversation History Open
```
┌──────────┬───────────────┬────────────────────────────┬───────┐
│ Sidebar  │  History      │    Chat Messages           │Sources│
│          │               │                            │       │
│  • Nav   │ [+ New Chat]  │  [Messages as usual]       │[panel]│
│  • Mode  │               │                            │       │
│          │ ───────────   │                            │       │
│          │               │                            │       │
│          │ ● Today       │                            │       │
│          │ • EdgeQuake.. │                            │       │
│          │ • API config  │                            │       │
│          │               │                            │       │
│          │ ● Yesterday   │                            │       │
│          │ • LLM setup   │                            │       │
│          │ • Graph query │                            │       │
│          │               │                            │       │
│          │ [Search...]   │                            │       │
└──────────┴───────────────┴────────────────────────────┴───────┘
     256px      280px              ~1024px             400px
```

---

## Related Files & Components

### Components to Modify:
- ✏️ [`src/components/query/query-interface.tsx`](../edgequake_webui/src/components/query/query-interface.tsx) - Major refactor
- ✏️ [`src/components/query/query-mode-selector.tsx`](../edgequake_webui/src/components/query/query-mode-selector.tsx) - Convert to segmented control
- ✏️ [`src/components/query/source-citations.tsx`](../edgequake_webui/src/components/query/source-citations.tsx) - Improve display

### New Components to Create:
- 🆕 `src/components/query/query-empty-state.tsx` - Rich empty state
- 🆕 `src/components/query/conversation-list.tsx` - History sidebar
- 🆕 `src/components/query/message-actions.tsx` - Reusable action bar
- 🆕 `src/components/query/progressive-loading.tsx` - Multi-phase loader
- 🆕 `src/components/query/query-examples.tsx` - Example query cards
- 🆕 `src/components/query/right-panel.tsx` - Collapsible context panel

### Store Updates:
- ✏️ [`src/stores/use-query-store.ts`](../edgequake_webui/src/stores/use-query-store.ts) - Add conversation management

### API Changes:
- Add conversation CRUD endpoints
- Add cancel query endpoint
- Add export conversation endpoint

---

## Priority Summary

**🔥 Must Do (Quick Wins):**
1. ✅ Auto-expanding textarea (R3) - 1 hour
2. ✅ Rich empty state (R6) - 2-3 hours
3. ✅ Collapsible right panel (R1) - 2 hours
4. ✅ Mode selector as segmented control (R4) - 2 hours

**📌 Should Do (Next Sprint):**
5. Conversation management (R7, R8) - 4-6 hours
6. Improved source citations (R2) - 3 hours
7. Progressive loading states (R9) - 2 hours
8. Comprehensive message actions (R10) - 3 hours
9. Keyboard shortcuts - 2-3 hours

**💡 Nice to Have (Later):**
10. Export conversation
11. Inline settings (temperature, etc.)
12. Query history/autocomplete
13. Cited sources in response text
14. Mobile optimization (bottom sheet for sources)
