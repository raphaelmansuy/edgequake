# SPEC-043 — Model Picker List: Keyboard & Scroll UX

**Spec:** `043-update-edgequake-llm` · doc `010`  
**Date:** 2026-07-06  
**Method:** Code is law — behavior maps to `model-picker-panel.tsx`, `use-scroll-contained-wheel.ts`, E2E screenshots  
**Trigger:** User report — arrow keys and mouse wheel poorly implemented in model search dropdown (Settings + Workspace)

---

## Assessment (current gaps)

| Issue | Root cause | User impact |
| ----- | ---------- | ----------- |
| **Wheel scrolls page, not list** | Popover is portaled inside scrollable Settings/Workspace pages; wheel events bubble to parent `ScrollArea` | Long provider catalogs (Mistral + LM Studio + …) feel “stuck”; user must drag scrollbar |
| **Keyboard highlight lost during fetch** | `searchLoading` / `providerCatalogLoading` **unmount** the entire `CommandGroup` tree | ↑↓ does nothing while “Loading … models…” spinner shows |
| **No loop at list ends** | `Command` missing `loop` prop | At last item, ↓ stops; at first, ↑ does not wrap |
| **Focus not guaranteed on open** | Radix `PopoverContent` default auto-focus may land on trigger, not search input | Typing after open may miss the filter field |
| **No scroll-into-view for keyboard focus** | cmdk selects items but long lists can leave highlight off-screen | Highlight exists but is invisible below fold |

### What works today

- `cmdk` + `shouldFilter={false}` with client-side filter + remote `/models/search` (correct split)
- `Escape` closes popover (Radix)
- `Enter` selects highlighted item (cmdk `onSelect`)
- `data-testid` on search, options, trigger (E2E-ready)

---

## Interaction contract (target)

| Input | Context | Behavior |
| ----- | ------- | -------- |
| **Open** | Click trigger or `Space`/`Enter` on combobox | Popover opens; **search input receives focus** |
| **Type** | Search focused | Filter local list (<2 chars) or debounced remote search (≥2 chars); list stays mounted |
| **↓ / ↑** | Search or list | Move `data-selected` highlight; **loop** at ends |
| **Enter** | Item highlighted | Select model; close popover; emit `onChange` |
| **Escape** | Any | Close popover; restore trigger focus |
| **Wheel** | Pointer over list | Scroll **list only** (`overscroll-behavior: contain` + stop propagation) |
| **Tab** | Popover open | Tab cycles input → list → out (no focus trap beyond popover) |

### Non-goals

- Virtualized lists (catalog ≤50 per provider is acceptable)
- Replacing `cmdk` with custom listbox (reuse shadcn `Command` stack)

---

## Implementation plan

### P4.1 — Shared scroll containment (DRY)

**File:** `edgequake_webui/src/hooks/use-scroll-contained-wheel.ts`

- `onWheel`: `stopPropagation()` so parent page does not scroll
- `overscroll-contain` class on scroll container
- Optional `scrollSelectedIntoView(listRef)` helper for keyboard nav (query `[cmdk-item][data-selected="true"]`)

**SRP:** One hook reused by any popover list (model picker first consumer).

### P4.2 — Model picker panel fixes (SOLID)

**File:** `edgequake_webui/src/components/models/model-picker-panel.tsx`

| Change | Rationale |
| ------ | --------- |
| `Command loop` | Wrap keyboard navigation |
| `PopoverContent onOpenAutoFocus` → focus search | Predictable type-to-filter |
| Loading = **banner overlay**, not conditional unmount | Preserve cmdk item registry for ↑↓ |
| `CommandList` + hook + `data-testid="model-picker-panel-list"` | E2E scroll assertions |
| `aria-label` on list | a11y |

### P4.3 — E2E proof

**File:** `edgequake_webui/e2e/spec043-llm-model-picker.spec.ts`

| Screenshot | Test |
| ---------- | ---- |
| `08-model-picker-keyboard-focus.png` | Open dropdown → ↓×3 → capture highlighted row |
| `09-model-picker-wheel-scroll.png` | Wheel over list → `scrollTop > 0` |

**Output dir:** `specs/043-update-edgequake-llm/e2e/screenshots/` (via `spec043Screenshot()`)

---

## Component touchpoints

```
ModelPickerPanel
└── PopoverContent (onOpenAutoFocus → search)
    └── Command (shouldFilter={false}, loop)
        ├── CommandInput (ref, auto-focus)
        └── CommandList (useScrollContainedWheel, max-h 320px)
            ├── [loading banner — absolute, non-blocking]
            ├── CommandEmpty
            └── CommandGroup → CommandItem (data-testid per option)
```

---

## Verification

```bash
# Unit (hook — if extracted tests added)
cd edgequake_webui && bun test src/hooks/__tests__/use-scroll-contained-wheel.test.ts

# E2E (live stack)
make dev-bg
cd edgequake_webui
EQ_BACKEND_URL=http://localhost:8081 E2E_LIVE_STACK=1 \
  pnpm exec playwright test e2e/spec043-llm-model-picker.spec.ts -g "keyboard|wheel"
```

---

## Cross-refs

- [006-ux-ui-model-picker.md](./006-ux-ui-model-picker.md) — interaction table (updated)
- [008-implementation-plan.md](./008-implementation-plan.md) — P4 UX phase
- [009-cross-reference-matrix.md](./009-cross-reference-matrix.md) — FEAT traceability
