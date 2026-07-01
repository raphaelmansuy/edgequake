# SPEC-037 — UX/UI Designer Lens

**Lens:** UX/UI Designer  
**Reference:** User screenshots (Query Settings clip, truncated passages)  
**Design system:** shadcn/ui Sheet, ScrollArea, Switch — match existing Query Settings patterns

---

## Issue 1 — Non-Scrollable Settings Panel

### Current State (broken)

```text
┌─ Query Settings ──────────────── ✕ ┐
│ CONTEXT                             │
│   Provider, Document Filter, Scope  │
│ RESPONSE MODE                       │
│   Streaming toggle                  │
│ RETRIEVAL                           │
│   Top K slider                      │
│ GENERATION                          │
│   Temperature, Max Tokens           │
│ SYSTEM PROMPT  ← clipped, no scroll  │
└─────────────────────────────────────┘
```

### Target State (fixed)

```text
┌─ Query Settings ──────────────── ✕ ┐
│ CONTEXT                    ▲ shadow │
│ ...                                 │
│ GENERATION                          │
│ SYSTEM PROMPT                       │
│   [ Custom Instructions textarea ]  │
│                              ▼ scroll│
└─────────────────────────────────────┘
```

### Layout Spec

| Element | Classes | Notes |
| ------- | ------- | ----- |
| `SheetContent` | `flex flex-col p-0 overflow-hidden` | Constrain height to viewport |
| `SheetHeader` | `shrink-0` | Already correct |
| `ScrollArea` | `flex-1 min-h-0` + `showShadows` | Copy `right-panel.tsx:140` |
| Section spacing | `space-y-5` in scroll body | Unchanged |
| Bottom padding | `pb-6` on scroll inner div | Prevent last control flush against edge |

### Interaction

- **Keyboard:** Tab order flows through all controls inside scroll region; focus scrolls into view (native browser behavior).
- **Touch:** Momentum scroll on trackpad/touch inside ScrollArea viewport.
- **Close:** X button remains fixed (Radix `SheetPrimitive.Close` absolute top-right).

### Accessibility

| Requirement | Implementation |
| ----------- | -------------- |
| Scroll region label | `aria-label="Query settings options"` on ScrollArea or inner `role="group"` |
| Section headings | Keep existing `h3` uppercase labels |
| System Prompt | `Label` + `htmlFor="system-prompt"` — already present |

---

## Issue 2 — Full Passage Text Toggle

### Placement

**Section:** Response Mode (with Streaming toggle)  
**Rationale:** Both control *how results are delivered*, not retrieval depth (Top K) or generation (Temperature).

```text
RESPONSE MODE
┌─────────────────────────────────────────┐
│ Streaming          [====●]              │
│ Show response as it generates           │
│                                         │
│ Full passage text  [  ●====]  ← NEW     │
│ Show complete retrieved chunks in       │
│ citations (uses more bandwidth)       │
└─────────────────────────────────────────┘
```

### Control Spec

| Property | Value |
| -------- | ----- |
| Component | `Switch` (same as Streaming) |
| ID | `full-chunk-toggle` |
| Default | OFF (`citation`) |
| Disabled when | Query in progress (`disabled` prop) |
| i18n keys | `query.settings.fullPassageText`, `query.settings.fullPassageTextDescription` |

### Visual Hierarchy

- **OFF:** Passage cards use `line-clamp-3`, ~3 lines visible, ellipsis — current density.
- **ON:** Passage cards use `line-clamp-none` or higher clamp (e.g. `line-clamp-6`), `whitespace-pre-wrap`, `break-words`. Full text from API displayed.
- **Score badge:** Unchanged (right-aligned %).
- **Expand "+N more passages":** Unchanged behavior.

### Citation Card — Truncated vs Full

```text
OFF (citation):                    ON (agent):
┌─ passage 1 ─────────── 100% ─┐  ┌─ passage 1 ─────────── 100% ─┐
│ ...woven into prose. DO NOT: │  │ [full paragraph text        │
│ Quote paragraphs from the... │  │  wrapping across multiple   │
└──────────────────────────────┘  │  lines without mid-word cut]│
                                  └──────────────────────────────┘
```

### Tooltip Copy

Update passage hover tooltip when full mode ON:

- OFF: "Click to open and highlight this passage in the document viewer"
- ON: Same — click still opens document viewer for PDF highlight

### Mobile (≤640px)

- Settings sheet width: `w-[400px]` — acceptable
- Full passages: vertical scroll within citation accordion; no horizontal scroll
- Toggle label wraps; description `text-[11px]` matches Streaming subtext

---

## Issue 3 — Settings Page Parity (optional P2)

Global Settings page (`settings/page.tsx`) has query defaults. **Recommend:** add same toggle there for users who never open Query sheet. Single source: `use-settings-store.querySettings.fullChunkContent`.

---

## Design Tokens (unchanged)

| Token | Usage |
| ----- | ----- |
| `text-muted-foreground` | Section headers, descriptions |
| `bg-muted/20` | Section card backgrounds |
| `text-amber-500` | Response Mode icon (Zap) |
| `text-emerald-500` | System Prompt icon (FileText) |

---

## UX Acceptance Checklist

- [ ] System Prompt reachable without browser zoom on 768px height
- [ ] Scroll shadows visible when content overflows
- [ ] Full passage toggle adjacent to Streaming — logical grouping
- [ ] OFF state visually identical to current production
- [ ] ON state shows materially more text (user can read full sentence)
- [ ] No layout shift when toggling mid-conversation (applies to *next* query only — show helper text if needed)

---

## Anti-Patterns (do not)

- ❌ Put full-chunk toggle under Retrieval (wrong mental model)
- ❌ Use a second ScrollArea nested inside sections
- ❌ Remove `line-clamp` in citation mode (regression for density)
- ❌ Fetch full chunks via separate API call from UI (latency + complexity)

---

## REQ Mapping

| REQ | UX Deliverable |
| --- | -------------- |
| REQ-037-01 | Scroll layout spec above |
| REQ-037-02 | Toggle in Response Mode section |
| REQ-037-06 | Citation card display modes |
