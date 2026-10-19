# 03 — Keyboard specification

Single source of truth in code:

| Concern | File |
| ------- | ---- |
| Destinations (sidebar, palette, chords) | `src/lib/keyboard/destinations.ts` |
| Chord state machine (pure, unit-tested) | `src/lib/keyboard/chords.ts` |
| DOM guards (editable target, overlay open) | `src/lib/keyboard/dom.ts` |
| Rendered catalog (help dialog) | `src/lib/keyboard/catalog.ts` |
| Global handler | `src/hooks/use-keyboard-shortcuts.ts` |

## Global bindings

| Keys | Action | Notes |
| ---- | ------ | ----- |
| ⌘/Ctrl + K | Toggle command palette | Works from inside inputs. Only global combo (browser-safe). |
| `?` | Keyboard help | Ignored while typing / dialog open. |
| `[` | Collapse / expand sidebar | |
| `g` then `h d q g k w p c a s` | Go to Dashboard, Documents, Query, Graph, Knowledge, Workspace, Pipeline, Costs, API Explorer, Settings | 1.2 s window; a "Go to…" popup lists the second keys. |
| Esc | Close the top-most dialog / menu / panel | Radix default. |

Rules:

1. A bare key never fires inside `input`, `textarea`, `select`, contenteditable, or `role=textbox|searchbox|combobox`.
2. A bare key never fires while a dialog, alertdialog, menu or listbox is open.
3. No browser-reserved combos (⌘G, ⌘D, ⌘,, ⌘⇧Q are retired).
4. One handler owns each key; page handlers must `preventDefault` and the global layer skips defaulted events.

## Page bindings

- Documents: `R` refresh, ⌘/Ctrl + A select all, Esc clear selection / close preview.
- Query: `/` focus composer, `@` scope to a document.
- Graph: `/` search nodes, arrows move between connected nodes, Enter focus, 1-3 depth, + − 0 zoom, F fullscreen.

## Focus rules

- Skip link is the first tab stop and lands on `<main id="main-content">`.
- Every interactive element shows a visible focus ring (≥ 2 px, ≥ 3:1 contrast).
- Sidebar links: roving order top→bottom; tooltips appear on keyboard focus.
- Palette: focus lands in the input, ↑/↓ moves, Enter runs, Esc closes and returns focus to the trigger.

## Verification

`e2e/spec168/keyboard.spec.ts` (mock-api project) plus `src/lib/keyboard/__tests__/`.
