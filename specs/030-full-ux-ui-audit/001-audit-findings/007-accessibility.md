# Audit: Accessibility / WCAG 2.1 AA

**Reference:** [WCAG 2.1 AA Quick Ref](https://www.w3.org/WAI/WCAG21/quickref/)

---

## Automated Checklist

| Criterion                      | Status    | Notes                                        |
| ------------------------------ | --------- | -------------------------------------------- |
| 1.1.1 Non-text Content         | ⚠ Partial | Graph node icons missing alt                 |
| 1.4.3 Contrast (4.5:1)         | ⚠ Partial | muted-foreground was 0.556 (fixed to 0.48)   |
| 1.4.11 Non-text Contrast (3:1) | ✅ Fixed   | border darkened to 0.86                      |
| 2.1.1 Keyboard                 | ⚠ Partial | Dropdown selector not searchable by keyboard |
| 2.4.3 Focus Order              | ✅         | Logical tab order in sidebar                 |
| 2.4.7 Focus Visible            | ⚠         | Dark mode ring may be low contrast           |
| 3.2.2 On Input                 | ✅         | Workspace switch shows toast                 |
| 4.1.2 Name, Role, Value        | ⚠         | Avatar "N" button missing aria-label         |

---

## Findings

### F-A11Y-01 · Avatar button at bottom of sidebar has no accessible name · HIGH
**Problem:** The "N" circle button at the bottom left of the sidebar (likely showing user initial) has no aria-label.

### F-A11Y-02 · Graph canvas (Sigma.js) has no keyboard navigation hook for all nodes · HIGH
**Problem:** The graph canvas uses `useGraphKeyboardNavigation` hook but the Sigma.js canvas element itself is not keyboard-accessible for users who cannot use a mouse.  
**Fix:** Ensure the entity browser panel on the left provides full keyboard access to all nodes via the list view. This is the accessible alternative path.

### F-A11Y-03 · Modal dialogs (create tenant/workspace) trap focus correctly · ✅
**Status:** The Dialog component from Radix UI handles focus trapping correctly.

### F-A11Y-04 · Skip link exists · ✅
**Status:** `<SkipLink />` component renders correctly in dashboard layout.

### F-A11Y-05 · Color is not the only differentiator for status · ✅ Partial
**Problem:** Status badges use color (green=completed, orange=processing) but also include text labels. Good. However, graph nodes use color as the only differentiator for entity type (with no pattern/shape difference).

### F-A11Y-06 · Toasts (sonner) are not announced to screen readers · MED
**Problem:** Toast notifications appear visually but may not be announced by screen readers if they lack `role="status"` or `aria-live`.  
**Fix:** Ensure the Sonner configuration includes `aria-live="polite"` for info toasts and `aria-live="assertive"` for errors.

---

## Keyboard Navigation Matrix

| Flow                     | Works? | Notes                         |
| ------------------------ | ------ | ----------------------------- |
| Tab through sidebar      | ✅      | Good focus rings              |
| Open workspace selector  | ✅      | Enter/Space opens dropdown    |
| Search workspace (fuzzy) | ❌      | Not implemented               |
| Navigate graph nodes     | ⚠      | Via list panel only           |
| Upload document          | ✅      | Dropzone has keyboard trigger |
| Dismiss dialogs          | ✅      | Escape closes                 |
| Tab to action buttons    | ✅      |                               |
