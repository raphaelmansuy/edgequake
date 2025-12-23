# UX/UI Improvement: Query Page

## Current State Analysis

### Page Structure

- **Header**: Title with query mode selector (Local, Global, Hybrid, Simple)
- **Main Area**: Conversation interface with starter prompts
- **Input Area**: Text input with send button, hint text
- **History Sidebar**: Recent queries with favorite/delete actions

### Positive Observations

- Chat-like interface is intuitive
- Starter prompts help users begin
- Query mode selector is clear
- History sidebar enables quick re-queries
- Favorite functionality for saved queries

---

## UX Issues Identified

### Critical

1. **Query Mode Explanation**

   - **Issue**: Local/Global/Hybrid/Simple modes are not explained
   - **Impact**: Users don't know which mode to use
   - **Recommendation**:
     - Add tooltips explaining each mode
     - Show brief description below buttons
     - Add "Learn more" link to documentation

2. **No Response Display Area**
   - **Issue**: Empty area shows starter prompts but no clear response zone
   - **Impact**: Unclear where responses will appear
   - **Recommendation**:
     - Add subtle separator between input and response area
     - Show loading indicator during query
     - Animate response appearance

### High Priority

3. **Query Input UX**

   - **Issue**: Single line input with shift+enter hint is easy to miss
   - **Impact**: Multi-line queries are awkward
   - **Recommendation**:
     - Auto-expand textarea as user types
     - Move hint inside input as placeholder
     - Add character/token count

4. **Starter Prompts**

   - **Issue**: Good prompts but no visual distinction
   - **Impact**: May not be noticed as clickable
   - **Recommendation**:
     - Add arrow or button styling
     - Add hover effects
     - Randomize order or show contextual prompts

5. **History Sidebar Width**

   - **Issue**: Query text is truncated ("What is Code2Do...")
   - **Impact**: Can't see full query
   - **Recommendation**:
     - Show full text on hover tooltip
     - Wider sidebar or collapsible
     - Search within history

6. **Send Button State**
   - **Issue**: Send button is disabled when empty, but no visual feedback
   - **Impact**: Button appears broken
   - **Recommendation**:
     - Clear disabled state styling
     - Enable on first character
     - Show "Ask" or "Send" text

### Medium Priority

7. **Response Streaming**

   - **Issue**: Settings show "Enable Streaming" toggle but behavior unclear
   - **Impact**: Users don't know what to expect
   - **Recommendation**:
     - Default to streaming for better UX
     - Show typing indicator during generation
     - Allow stop generation

8. **Source Attribution**

   - **Issue**: Responses may not show which documents/entities were used
   - **Impact**: Lack of transparency and trust
   - **Recommendation**:
     - Show "Sources:" section in responses
     - Link to source documents
     - Highlight relevant graph nodes

9. **Conversation Context**

   - **Issue**: Unclear if follow-up questions maintain context
   - **Impact**: Users may repeat information
   - **Recommendation**:
     - Show conversation thread
     - Add "New Conversation" button
     - Indicate context window size

10. **Query Error Handling**
    - **Issue**: No visible error states
    - **Impact**: Failed queries may appear stuck
    - **Recommendation**:
      - Show error messages clearly
      - Add "Retry" button
      - Suggest troubleshooting steps

### Low Priority

11. **History Organization**

    - **Issue**: Only "Recent" shown, no favorites section
    - **Impact**: Favorites mixed with recent
    - **Recommendation**:
      - Separate Favorites and Recent tabs
      - Add date grouping
      - Add search in history

12. **Export Conversation**

    - **Issue**: No way to save/share conversations
    - **Impact**: Can't document findings
    - **Recommendation**:
      - Export as Markdown
      - Copy response button
      - Share link

13. **Keyboard Shortcuts**
    - **Issue**: Only Enter/Shift+Enter mentioned
    - **Impact**: Power users limited
    - **Recommendation**:
      - Cmd+K for quick mode change
      - Cmd+L for clear conversation
      - Up arrow for last query

---

## Recommendations

### Short Term (Sprint 1)

- [ ] Add tooltips for query modes
- [ ] Improve input area with auto-expand
- [ ] Add clear loading and response states
- [ ] Fix history truncation with tooltips

### Medium Term (Sprint 2)

- [ ] Implement response streaming UI
- [ ] Add source attribution to responses
- [ ] Add conversation history/context
- [ ] Improve error handling

### Long Term

- [ ] Add advanced query builder
- [ ] Implement conversation export
- [ ] Add query templates
- [ ] Voice input option

---

## Wireframe: Query Mode Tooltips

```
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│  Query Mode:                                                │
│  ┌───────┐ ┌────────┐ ┌────────┐ ┌────────┐                │
│  │ Local │ │ Global │ │ Hybrid │ │ Simple │                │
│  └───────┘ └────────┘ └────────┘ └────────┘                │
│      │                                                      │
│      ▼                                                      │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ 🔍 Local Mode                                       │   │
│  │ ────────────────                                    │   │
│  │ Searches within specific entity neighborhoods.      │   │
│  │ Best for: Questions about specific people/orgs     │   │
│  │                                                     │   │
│  │ Example: "What does Dr. Chen work on?"              │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## Wireframe: Response with Sources

```
┌─────────────────────────────────────────────────────────────┐
│ 💬 You                                                      │
│ ────                                                        │
│ What are the main organizations in my knowledge graph?      │
│                                                             │
│ 🤖 EdgeQuake                                                │
│ ────                                                        │
│ Based on your knowledge graph, the main organizations are:  │
│                                                             │
│ 1. **Stanford University** - Research institution where     │
│    Dr. Sarah Chen leads the AI Research Lab                │
│                                                             │
│ 2. **Google DeepMind** - Collaborates on AI projects        │
│                                                             │
│ 3. **OpenAI** - Partner organization                        │
│                                                             │
│ 4. **Microsoft Research** - Funding partner                 │
│                                                             │
│ 5. **NSF** - National Science Foundation, provides grants   │
│                                                             │
│ ─────────────────────────────────────────────────────────── │
│ 📚 Sources: knowledge_test.md                               │
│ 🔗 View in Graph: Stanford University, OpenAI...           │
└─────────────────────────────────────────────────────────────┘
```

---

## Wireframe: Improved Input Area

```
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ Ask about your knowledge graph...                   │   │
│  │                                                     │   │
│  │ [Auto-expands as you type]                          │   │
│  │                                                     │   │
│  │                                      ⌘↵ Send  [➤]   │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  💡 Try: "Find connections between people and projects"    │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## Acceptance Criteria

- [ ] Query modes have explanatory tooltips
- [ ] Input auto-expands for multi-line queries
- [ ] Loading state is clearly visible
- [ ] Responses show source documents
- [ ] History items show full text on hover
- [ ] Streaming responses work smoothly
- [ ] Errors are handled gracefully
- [ ] Favorites are separated from recent
