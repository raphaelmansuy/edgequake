# 001 — Right Panel Deep Audit

**First Principle: Clarity** — a detail panel should answer "what is this?" in 3 seconds.

---

## Current State Analysis (from live screenshots)

```
┌──────────────────────────────────────────────────────────────────┐
│ 📄 European Union Artificial Intelligence Act...  [›] [×]      │  ← HEADER
│ ID: 31451aaf-416...                                              │  ← WASTED ROW
├──────────────────────────────────────────────────────────────────┤
│ 📄 European Union Artificial Intelligence Act... (G) ← truncated│
│    ✓ Completed                                                   │
├──────────────────────────────────────────────────────────────────┤
│ DETAILS                              (uppercase label)           │
│ 📄 ID         31451aaf-416...        (truncated, copy button)    │
│ 📅 Created    about 2 hours          (truncated at right edge)  │
│ ⏰ Updated    about 2 hours          (truncated at right edge)  │
│ 🔗 Entities   2                      (only 4 visible)            │
├──────────────────────────────────────────────────────────────────┤
│ $ PROCESSING COST                    (uppercase label)           │
│ ┌─────────────────────────────────┐                              │
│ │ Total Cost         $0           │ ← ORANGE (wrong!)            │
│ │ ⚡ Total Tokens    353...       │ ← TRUNCATED                  │
│ │ Input Tokens       133          │ ← TRUNCATED                  │
│ │ Output Tokens      220          │ ← TRUNCATED                  │
│ │ LLM Model          mistral-smal │ ← TRUNCATED                  │
│ │ Embedding          mistral-em...│ ← TRUNCATED                  │
│ └─────────────────────────────────┘                              │
├──────────────────────────────────────────────────────────────────┤
│ CONTENT PREVIEW                      [Copy][…]                   │
│ # Bird & Bird                        ← Raw markdown              │
│ ## European Union AI Act: a guide                                │
│ *7 April 2025*                                                   │
│ # Contents                                                       │
└──────────────────────────────────────────────────────────────────┘
```

---

## Critical Issues

### RP-01 · ID in Header Bar: Wasted Prime Real Estate

**Severity: High**

The panel header shows:
```
European Union Artificial Intelligence Act...  [›][×]
ID: 31451aaf-416...
```

The `ID: 31451aaf-416...` on the second line of the header:
- Is never needed for routine document preview workflows
- Is already accessible inside Details section
- Makes the header 2 lines tall unnecessarily
- Uses valuable real estate to show a UUID fragment

**Code ref:** `right-panel.tsx` — the subtitle prop renders below the title.

### RP-02 · Universal Value Truncation

**Severity: Critical**

Every single value in the panel is truncated because:
1. The panel renders values in a `flex items-center justify-between` row
2. `flex justify-between` puts the value at the far right
3. The panel is only ~480px wide but the values have no `max-width` or `truncate`

Affected fields: Created, Updated, Total Tokens, Input Tokens, Output Tokens, LLM Model, Embedding.

The issue: `justify-between` layout means the label gets all the left space and the value all the right space — but the right side is the panel edge, so values overflow and clip.

### RP-03 · "$0" Rendered in Orange — Wrong Color Semantics

**Severity: High**

```typescript
function getCostColor(cost: number | undefined): string {
  if (cost === undefined || cost === null || cost === 0) return 'text-muted-foreground';
  ...
}
```

The function returns `text-muted-foreground` for zero cost — so the color should be correct. But the actual rendered color in the screenshot shows **orange for "$0"**. Investigation needed.

**Root cause:** The `document.cost_usd` field is `0` (number zero) which should trigger the `text-muted-foreground` path. But if the API returns `null` instead of `0`, then:
- `cost === null` → `text-muted-foreground` ✓  
- BUT the `formatCost` function returns `'-'` for null — yet the screenshot shows `$0` in orange.

This suggests the API returns `cost_usd: 0` (actual zero) but something is wrong in the render path. The field `document.cost_usd` may be loaded from `fullDocument` vs `document` inconsistently.

**Fix:** The `getCostColor` function already handles `0` correctly — but the total cost shown in orange at `$0` suggests `cost_usd = 0` falls into a later branch. Looking more carefully: `$0` in orange means `formatCost(0)` = `'Free'` or it's being calculated differently.

Wait — looking at the screenshot again from the user: the Total Cost shows `$0` in **orange**, but `formatCost(0)` returns `'Free'`. This means the cost is NOT zero — it's a very small positive number that triggers `text-orange-500` because `cost >= 0.1`. But then it displays as `$0` which is the `< 0.0001` branch that shows `< $0.0001`.

The real bug: `getCostColor` has these thresholds:
- `< 0.001` → green (cheap)
- `< 0.01` → blue  
- `< 0.1` → yellow
- else → **orange (expensive!)**

If `cost_usd = 0.158` (the value shown in the documents table for the EU AI Act doc), then `getCostColor(0.158)` = `text-orange-500`. But that's NOT "$0" — the panel isn't loading the full cost properly, or `document.cost_usd` is different from what's displayed in the table.

**The real issue:** The detail panel shows `$0` in orange because it reads `document.cost_usd` (which might be null/undefined in the list-view doc object) and then falls through to a different code path. The `fullDocument` has the real cost but it loads asynchronously.

### RP-04 · Section Labels Bureaucratic

**Severity: Medium**

`DETAILS`, `$ PROCESSING COST`, `CONTENT PREVIEW` in ALL CAPS with tracking-wider is the design language of admin panels from 2015. It creates visual noise and unnecessary formality.

**Fix:** Lowercase, lighter weight section labels match the clean/minimalist aesthetic.

### RP-05 · Redundant Document Title (shown twice)

**Severity: Medium**

The document title appears in:
1. The right panel header bar (truncated)
2. The `<h3>` inside the panel content (also truncated)

The header title duplicates what's already visible in the content area. The header should show only the minimal context needed for navigation (status indicator + close button).

### RP-06 · Separator Overuse

**Severity: Low**

Three `<Separator />` components in a scrollable side panel:
- Between header area and Details
- Between Details and Processing Cost
- (implied) between Processing Cost and Content Preview

Section whitespace already creates visual separation. Horizontal rules add visual noise without semantic value in a confined side panel.

### RP-07 · Content Preview: Raw Markdown

**Severity: Medium**

The content preview renders raw markdown text (`# Bird & Bird`, `## European Union AI Act`). This is correct for a technical preview, but:
- Raw `#` heading syntax is jarring when users expect a readable preview
- No syntax highlighting for code blocks
- The `pre` or `monospace` font is implied but not consistently applied

**Options:**
- Render markdown with a lightweight renderer (already have one in the query interface)
- Show plain text with headings stripped (simpler)
- Show "formatted" mode toggle

### RP-08 · Panel Actions Not Visible Without Scrolling

**Severity: Medium**

The action buttons (Open Full, View in Graph, Reprocess, Delete) are presumably at the bottom of the panel content, below the content preview. Users have to scroll down to find actions. For a quick-preview panel, actions should be immediately accessible.

### RP-09 · No Tooltip for Full Model Name

**Severity: Low**

`LLM Model: mistral-small-late...` truncates a technical value that users might care about. Should show full name on hover via tooltip.

---

## Proposed Redesign

```
┌──────────────────────────────────────────────────────────────────┐
│ [📄 icon] Document                        [Open ↗] [×]          │  ← lean header
├──────────────────────────────────────────────────────────────────┤
│ European Union Artificial Intelligence Act_Guide_202504 (6).pdf  │  ← full title (wraps)
│ ✓ Completed · 2,604 entities · about 2 hours ago                │  ← compact meta line
├──────────────────────────────────────────────────────────────────┤
│ Details                                   (lowercase, muted)     │
│ Created      about 2 hours ago                                   │  ← label left, value right with overflow-hidden
│ File size    24.3 MB                                             │
│ ID           31451aaf... [copy icon]                             │
├──────────────────────────────────────────────────────────────────┤
│ Cost                                      (lowercase, muted)     │
│ $0.158       35.3K tokens    mistral-small-latest                │  ← single row summary
├──────────────────────────────────────────────────────────────────┤
│ Content                                   (lowercase, muted)     │
│ Bird & Bird                               (h1 stripped)          │
│ European Union AI Act: a guide            (h2 stripped)          │
│ 7 April 2025                                                     │
│ ...                                       [View full document]   │
├──────────────────────────────────────────────────────────────────┤
│ [View in Graph] [Reprocess]            [Delete]                  │  ← actions ALWAYS visible at bottom
└──────────────────────────────────────────────────────────────────┘
```

---

## External References
- [Side Panel UX Patterns — NNGroup](https://www.nngroup.com/articles/side-panel-ux/)
- [Gmail Detail Pane](https://mail.google.com) — reference for efficient metadata display
- [Linear Issue Detail](https://linear.app) — reference for clean side panel layout
- [GitHub PR Files Changed](https://github.com) — truncation + tooltip pattern
