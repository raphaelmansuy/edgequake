# LENS — UI Designer

**Job:** visual system for one ingestion run card; remove conflicting Done/Queued chrome.  
**Cites:** LAW-22, LAW-25 · `ux086_dual_progress_ui` · UX lens copy pins

---

## 1. Visual hierarchy (one card)

```text
┌─────────────────────────────────────────────────────────┐
│  filename.md          [markdown]              [Cancel]  │
│  ● Uploading ✓  ○ Converting (skipped)  ● Chunking …    │
│  ████████░░░░  Chunking 72%                             │
│  ░░████░░░░░░  Overall (est.) 18%                       │
│  Chunking — Step 3: Processing pipeline…                │
└─────────────────────────────────────────────────────────┘
```

PDF during convert adds nested page line:

```text
│  Converting PDF — page 12 / 40                          │
```

---

## 2. Stepper states

| Visual | Meaning |
|--------|---------|
| Filled check / green | Stage completed |
| Pulsing / brand accent | Current stage |
| Muted + “skipped” | Not applicable (converting for non-PDF) |
| Grey outline | Upcoming |
| Red | Failed terminal on that stage |

**Never** use green “Done” for admission/queued.

---

## 3. Conflict audit (must fix)

| Anti-pattern | Fix |
|--------------|-----|
| Legend green Done + message Queued | Remove mini-legend or bind color to ui_phase |
| Compact MD bar without stages | Always show stepper (compact = smaller type, not fewer stages) |
| “Converting PDF” active on MD | Mark skipped / hide |
| Two cards for same track_id | Dedupe upload list vs ActiveRuns |

---

## 4. Density / tokens

- Reuse existing ActiveRuns / ServerStageStepper tokens (no new purple/glow aesthetic).  
- Compact mode: reduce padding and font size; keep stage chips.  
- Motion: pulse on current stage only (2–3 intentional motions max).

---

## 5. Accessibility

| Rule | Detail |
|------|--------|
| Color not sole signal | Text stage name always visible |
| `aria-live` | Polite updates on stage change |
| Cancel | Named button, keyboard reachable |
| Contrast | Skipped steps still readable |

---

## 6. Acceptance

| ID | Gate |
|----|------|
| UI-086-01 | Screenshot/e2e: no Done+Queued on same row |
| UI-086-02 | MD and PDF cards share stepper component |
| UI-086-03 | Skipped converting visually distinct from blocked |
