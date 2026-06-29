# SPEC-031 / 002 — UX/UI Design: Document Scope Selector

> **Lens**: UX/UI Designer  
> **Principle**: Progressive disclosure · Minimal surface area · Semantic clarity  
> **Cross-refs**: [001-problem-analysis.md](001-problem-analysis.md), [004-frontend-spec.md](004-frontend-spec.md)

---

## 1. Design Philosophy

Three constraints drive every choice:

1. **No visual noise when scope = full workspace** — the default state must be completely silent. No empty pills, no "All documents" badge cluttering the header.
2. **Instant recognition when scope is narrowed** — as soon as any documents are selected, the user must see it before typing a single character.
3. **Removal is one click** — dismissing a document from scope requires exactly one interaction, no confirmation dialog.

---

## 2. Query Page Layout — Current vs. Proposed

### 2a. Current Layout (Discoverability Problem)

```
+------------------------------------------------------------------+
| [=] Query   Ask questions about your knowledge graph     [+New]  |
|                                          [Mode v] [Settings ⚙]  |
+------------------------------------------------------------------+
| (messages area)                                                   |
|                                                                   |
+------------------------------------------------------------------+
| [img+] [text area: Ask a question...]              [Send ->]     |
+------------------------------------------------------------------+
```

**Problem**: Scope is buried inside Settings ⚙ → scroll → "Document Scope".
There is **zero discoverable affordance** at the default state.
The scope bar only rendered when docs were already chosen — a chicken-and-egg trap.

---

### 2b. Revised Layout — Always-Visible Scope Toolbar

The scope toolbar is **permanently visible** above the textarea.
It uses two distinct visual states to balance discoverability and visual noise.

**Empty state — full workspace (default):**

```
+------------------------------------------------------------------+
| [=] Query  Ask questions...                  [+New] [Mode] [⚙]  |
+------------------------------------------------------------------+
| (messages area)                                                   |
+--[Scope toolbar: always rendered]--------------------------------+
|  [≡ All docs ▾]                                                  |
+------------------------------------------------------------------+
|  [img+]  [Ask a question...                      ]  [Send ->]    |
|                         [Press Enter to send...]                 |
+------------------------------------------------------------------+
```

- "All docs ▾" is a low-contrast pill (muted text, no border fill)
- Clicking it opens `DocumentPickerPopover` directly — no settings required
- The label "All docs" communicates it's a filter that can be narrowed

**Active state — documents selected:**

```
+--[Scope toolbar: active, tinted bg]--------------------------+
|  Scope: [report-q1.pdf x] [contract.pdf x] [+ Add]  [x All] |
+-------------------------------------------------------------+
|  [img+]  [Ask a question...               ]  [Send ->]       |
+-------------------------------------------------------------+
```

**Key invariants:**
- Scope toolbar occupies **same vertical space** in both states — zero layout shift
- Empty state: muted/ghost style so it doesn't compete with the input
- Active state: secondary-color pills, subtle tinted background
- Single click to activate from empty state (no drill-in required)
- Settings sheet retains the scope section as secondary access

---

## 3. Scope States

### State 1: No Scope (Default — Full Workspace)

```
+------------------------------------------------------------------+
| [≡ All docs ▾]                                                   |
+------------------------------------------------------------------+
| [img+] [Ask a question...                         ]  [->]        |
+------------------------------------------------------------------+
```

- Scope toolbar **always visible** in this low-prominence form
- `[≡ All docs ▾]` is a ghost/muted button — visible but not attention-grabbing
- Clicking opens the `DocumentPickerPopover` immediately

---

### State 2: One or More Documents Selected

```
+--------------------------------------------------------------------+
| Scope: [report-q1.pdf x] [contract-2025.pdf x]  [+ Add]  [x All] |
+--------------------------------------------------------------------+
| [img+] [Ask a question...                              ]  [->]     |
+--------------------------------------------------------------------+
```

- Pills show truncated document title (max 24 chars) + `×` dismiss button
- `[+ Add]` opens the `DocumentPickerPopover`
- `[× All]` clears the entire scope (returns to full workspace)
- Scope bar uses a subtle background tint to distinguish from the input area

---

### State 3: Scope Active + Loading (Query in Progress)

```
+--------------------------------------------------------------------+
| Scope: [report-q1.pdf x] [contract-2025.pdf x]  [+ Add]  [x All] |
+--------------------------------------------------------------------+
| [img+] [___ thinking... ___                            ]  [stop]  |
+--------------------------------------------------------------------+
```

- Pills are **dimmed** (opacity 0.6) during query execution
- `[+ Add]` and `[× All]` are disabled during query
- `[×]` on each pill is disabled during query

---

## 4. DocumentPickerPopover Detailed Design

Triggered by `[+ Add]` or initial `[+ Scope]` button.

### 4a. Popover Open State — Empty Search

```
+---------------------------+
| Add documents to scope    |
+---------------------------+
| [Search documents...    ] |   <- debounced 300ms, min 1 char
+---------------------------+
| No documents selected yet.|
|                           |
| Recent:                   |
|  O report-q1.pdf          |   <- 5 most recently uploaded
|  O contract-2025.pdf      |
|  O invoice-march.pdf      |
|  O data-analysis.pdf      |
|  O summary-june.pdf       |
+---------------------------+
```

- Shows 5 most recently uploaded `completed` documents as quick suggestions
- No results message until user types
- Checkboxes use `O` (unchecked) / `●` (checked) visually

---

### 4b. Popover Open State — Search Active

```
+---------------------------+
| Add documents to scope    |
+---------------------------+
| [report_______________ x] |   <- clear button appears when text > 0
+---------------------------+
| 3 results                 |
|  ● report-q1-2025.pdf     |   <- already selected (checked)
|  O report-q2-2025.pdf     |
|  O report-q3-2025.pdf     |
|                           |
| "report-q4" — not found   |   <- shown when no additional results
+---------------------------+
```

- Real-time filtering as user types (debounced 300ms)
- Documents already in scope show as pre-checked (●)
- `×` on the search input clears the search (not the selection)
- Shows "N results" counter for feedback
- "Not found" hint encourages spelling check

---

### 4c. Popover with Selection Summary

```
+---------------------------+
| Add documents to scope    |
+---------------------------+
| [____________________ ]   |
+---------------------------+
| 2 selected:               |
| report-q1-2025.pdf  [×]  |
| contract-2025.pdf   [×]  |
+---------------------------+
| Showing 5 of 42           |
|  O invoice-march.pdf      |
|  O data-analysis.pdf      |
|  O summary-june.pdf       |
|  O report-q3-2025.pdf     |
|  O financial-q1.pdf       |
+---------------------------+
| [Clear]          [Done ✓] |
+---------------------------+
```

- "N selected" section at top shows current selection with inline remove
- Scrollable list below (max height 240px, overflow scroll)
- `[Clear]` removes all selections
- `[Done]` commits and closes popover (selection visible as pills)

---

## 5. Scope Pill Design

Each pill in the scope bar:

```
 +----------------------------+
 | [doc-icon] report-q1.pd... [x] |
 +----------------------------+
```

- **Icon**: file-text icon (or PDF icon if type is PDF) — 14px
- **Label**: document title truncated at 22 chars with ellipsis
- **Dismiss button**: `×` — 14px, hover reveals subtle red tint
- **Tooltip**: Full document title on hover
- **Max pills visible**: 3 pills, then `+N more` chip
  ```
  [report-q1 x] [contract x] [invoice x] [+4 more]
  ```
  Clicking `+N more` expands to show all pills or opens the picker

---

## 6. Accessibility (ARIA)

| Element             | ARIA attributes                                                   |
| ------------------- | ----------------------------------------------------------------- |
| Scope bar container | `role="region" aria-label="Query scope"`                          |
| Individual pill     | `role="listitem"`                                                 |
| Pill dismiss button | `aria-label="Remove {document title} from scope"`                 |
| `[+ Add]` button    | `aria-label="Add documents to query scope"`                       |
| `[× All]` button    | `aria-label="Clear all document scope filters"`                   |
| Search input        | `aria-label="Search documents by title" aria-autocomplete="list"` |
| Result list         | `role="listbox" aria-label="Document search results"`             |
| Result item         | `role="option" aria-selected={isChecked}`                         |

---

## 7. Scope Bar Placement Logic

```
Query page renders:
  if (selectedDocumentIds.length === 0):
    render: null  (no scope bar)
  else:
    render: <QueryScopeBar ids={selectedDocumentIds} />
            placed between messages area and text input
```

The `QueryScopeBar` component is a sibling to the text input container, rendered conditionally. This means the page layout shifts slightly when scope is active — the text input area shrinks by ~36px to make room. This is acceptable because:
- The shift only happens when the user has actively chosen to scope
- It's immediately visible and provides clear feedback of the active state

---

## 8. Scope Entry Points

Two entry points to activate scope selection:

### Entry 1: Settings Sheet (existing path, minimal change)

In `QuerySettingsSheet`, add a `[+ Add documents to scope]` button alongside the existing `QueryDocumentFilter`. Clicking it opens the `DocumentPickerPopover` anchored to the settings sheet.

### Entry 2: Direct Scope Bar `[+ Add]` (new, primary path)

When the scope bar is visible (docs already selected), `[+ Add]` is always present. For first-time scope creation when scope bar is hidden, the settings sheet is the discovery path.

**First-discovery affordance**: When a user first uses the query page, after their first query, a dismissible tooltip appears:
```
  "Tip: You can scope queries to specific documents via ⚙ Settings → Document scope"
```
This is shown once per user (persisted in `localStorage`) and does not block the UI.

---

## 9. Mobile Behavior

On viewports < 640px:

- Pills are shown in a horizontally scrollable strip (no wrapping)
- Max 2 pills visible, then `+N`
- `[+ Add]` button is an icon-only `[+]` with tooltip
- The picker popover becomes a full-width bottom sheet (`<Sheet>` component)

---

## 10. Empty State in Picker

If the workspace has zero documents:

```
+---------------------------+
| Add documents to scope    |
+---------------------------+
| [Search documents...    ] |
+---------------------------+
| No documents in this      |
| workspace yet.            |
|                           |
| [Go to Documents →]       |
+---------------------------+
```

If the workspace has documents but search returns zero results:

```
+---------------------------+
| [xyz___________________] x|
+---------------------------+
| No documents match "xyz". |
| Try a different search    |
| term.                     |
+---------------------------+
```

---

## 11. Visual Design Tokens

| Element                | Token                            | Value                  |
| ---------------------- | -------------------------------- | ---------------------- |
| Scope bar background   | `bg-muted/40`                    | subtle gray tint       |
| Scope bar border       | `border-b`                       | same as other dividers |
| Pill background        | `bg-secondary`                   | secondary button color |
| Pill text              | `text-secondary-foreground`      |                        |
| Pill hover background  | `bg-secondary/80`                |                        |
| Pill dismiss `×` color | `text-muted-foreground`          |                        |
| Pill dismiss `×` hover | `text-destructive`               | red on hover           |
| Picker popover width   | `w-80` (320px)                   |                        |
| Picker max list height | `max-h-60` (240px)               |                        |
| "+N more" chip         | `bg-muted text-muted-foreground` |                        |
