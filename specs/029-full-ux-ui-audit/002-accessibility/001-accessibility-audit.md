# 001 — Accessibility Audit (WCAG 2.1 AA)

**First Principle: Trust** — Every user deserves equal access to meaning.

---

## WCAG 2.1 AA Checklist

### Perceivable

| Criterion                     | Status    | Notes                                                                                  |
| ----------------------------- | --------- | -------------------------------------------------------------------------------------- |
| 1.1.1 Non-text Content        | ⚠️ Partial | Icons have `aria-hidden="true"` ✓, but status badge dots lack text alternatives        |
| 1.3.1 Info and Relationships  | ⚠️ Partial | Table uses `<Table>` component ✓, but sidebar lacks `<aside>` landmark                 |
| 1.3.3 Sensory Characteristics | ✅ Pass    | No instructions rely on color alone                                                    |
| 1.4.1 Use of Color            | ⚠️ Fail    | Status badges use color as the PRIMARY differentiator (12 color variants)              |
| 1.4.3 Contrast (Normal)       | ⚠️ Fail    | `muted-foreground` OKLCH(0.556) on white ≈ 4.2:1 — passes AA but fails with small text |
| 1.4.4 Resize Text             | ✅ Pass    | Responsive layouts, rem units                                                          |
| 1.4.10 Reflow                 | ✅ Pass    | Mobile layouts present                                                                 |
| 1.4.11 Non-text Contrast      | ⚠️ Partial | Input borders `OKLCH(0.922)` on white = ~1.5:1 — fails 3:1 requirement                 |
| 1.4.12 Text Spacing           | ✅ Pass    | No fixed heights that would break text spacing                                         |

### Operable

| Criterion                 | Status    | Notes                                                                             |
| ------------------------- | --------- | --------------------------------------------------------------------------------- |
| 2.1.1 Keyboard            | ⚠️ Partial | Skip link ✓, keyboard shortcuts ✓, but modal focus traps need verification        |
| 2.1.2 No Keyboard Trap    | ⚠️ Risk    | `Sheet` (mobile sidebar) — Radix handles trapping ✓, but custom modals need audit |
| 2.4.1 Bypass Blocks       | ✅ Pass    | `SkipLink` component → `#main-content`                                            |
| 2.4.3 Focus Order         | ⚠️ Partial | Header renders before sidebar visually, but DOM order unclear                     |
| 2.4.6 Headings and Labels | ⚠️ Fail    | Settings page uses multiple `<CardTitle>` (h3) without clear h1/h2 hierarchy      |
| 2.4.7 Focus Visible       | ⚠️ Partial | `focus-visible:ring-2` applied inconsistently — login page missing on some inputs |
| 2.5.3 Label in Name       | ✅ Pass    | Button text matches `aria-label` where both present                               |
| 2.5.5 Target Size         | ⚠️ Fail    | Sidebar items `min-h-[40px]` — below 44px recommendation                          |

### Understandable

| Criterion                    | Status    | Notes                                                                                  |
| ---------------------------- | --------- | -------------------------------------------------------------------------------------- |
| 3.1.1 Language of Page       | ⚠️ Partial | `<html lang>` not audited — likely set by Next.js layout                               |
| 3.2.1 On Focus               | ✅ Pass    | No context changes on focus                                                            |
| 3.3.1 Error Identification   | ⚠️ Partial | Login error shown in UI ✓, but not associated to specific input via `aria-describedby` |
| 3.3.2 Labels or Instructions | ⚠️ Fail    | Login form: `<label>` present but no `aria-required` on required inputs                |
| 3.3.3 Error Suggestion       | ⚠️ Partial | Generic "Login failed" message — no corrective suggestion                              |

### Robust

| Criterion               | Status    | Notes                                                                                   |
| ----------------------- | --------- | --------------------------------------------------------------------------------------- |
| 4.1.1 Parsing           | ✅ Pass    | React generates valid HTML                                                              |
| 4.1.2 Name, Role, Value | ⚠️ Partial | `role="button"` on `<div>` elements in folder-sidebar.tsx (line 153, 366)               |
| 4.1.3 Status Messages   | ⚠️ Partial | `aria-live="polite"` on backend banner ✓, but toast notifications (`sonner`) need audit |

---

## Critical Issues

### A11Y-01 · Status Badges: Color as Sole Differentiator

```
CURRENT: 12 status colors with no icon OR shape differentiation
- uploading (blue)
- queued (amber)
- converting (indigo)
- preprocessing (blue)     ← same hue family as "uploading"
- chunking (blue)          ← same hue family
- extracting (purple)
- gleaning (purple)        ← same hue family as "extracting"
- merging (amber)          ← same hue family as "queued"
- completed (green)
- failed (red)
- partial_failure (orange)
- partial_success (amber)  ← same as "merging", "queued"
```

**Fix:** Each status needs a **distinct icon** (already has one via `statusConfig`) AND a **visible text label** — the color dot alone must not be the only indicator.

```typescript
// status-badge.tsx — ensure text labels are ALWAYS shown
// CURRENT: Badge shows dot + label text
// AUDIT: Verify the dot is supplemental, not primary, signal

// Add aria-label to badge that includes the text:
<Badge aria-label={`Status: ${config.label}`}>
  <dot />
  <span>{config.label}</span>
</Badge>
```

### A11Y-02 · Login Form: Missing `aria-describedby` for Errors

```typescript
// CURRENT (login/page.tsx):
const [error, setError] = useState<string | null>(null);
// ...
<Input id="username" ... />
// ...
{error && <div className="...">{error}</div>}
```

**Fix:**

```typescript
// ADD: error alert with role and ID for association
{error && (
  <div 
    role="alert" 
    aria-live="assertive"
    id="login-error"
    className="rounded-md bg-destructive/10 p-3 text-sm text-destructive"
  >
    <AlertCircle className="h-4 w-4 inline mr-1.5" aria-hidden="true" />
    {error}
  </div>
)}

// ADD: aria-describedby on form
<form 
  onSubmit={handleSubmit} 
  aria-describedby={error ? "login-error" : undefined}
>

// ADD: aria-required and aria-invalid
<Input
  id="username"
  type="text"
  required
  aria-required="true"
  aria-invalid={error ? "true" : undefined}
  ...
/>
```

### A11Y-03 · Input Border Contrast Fails WCAG 1.4.11

The `--input` token is `OKLCH(0.922)` on white background `OKLCH(1 0 0)`. This gives approximately 1.4:1 contrast ratio for the input border — WCAG 1.4.11 requires **3:1** for non-text visual components.

```css
/* CURRENT (globals.css):
--input: oklch(0.922 0 0);

/* FIX: Darken input border to achieve 3:1 against white */
--input: oklch(0.75 0 0);  /* ~3.1:1 against white */
--input-dark: oklch(0.4 0 0); /* dark mode */
```

### A11Y-04 · `<div role="button">` in Folder Sidebar

```typescript
// folder-sidebar.tsx:153, 366
<div role="button" ...>
```

**Fix:** Replace `<div role="button">` with `<button>` element. `<button>` gets keyboard events, `type="button"`, and inherits focus ring for free.

```typescript
// BEFORE:
<div role="button" onClick={...} className="...">

// AFTER:
<button type="button" onClick={...} className="...">
```

### A11Y-05 · Sidebar Landmark Missing

```typescript
// sidebar.tsx — outer wrapper
// CURRENT:
<div className="flex h-full flex-col w-[var(--sidebar-width)]">

// FIX: Use semantic <aside> with label
<aside 
  className="flex h-full flex-col w-[var(--sidebar-width)]"
  aria-label="Application navigation"
>
```

### A11Y-06 · Settings Page Heading Hierarchy

The settings page renders multiple `<CardTitle>` elements which resolve to `<h3>` (or similar) without a parent `<h1>`. Screen readers announce h3 without context.

```typescript
// settings/page.tsx — add page-level h1
export default function SettingsPage() {
  return (
    <ScrollArea>
      <div className="...">
        {/* ADD: visually hidden h1 for screen readers */}
        <h1 className="sr-only">{t('settings.pageTitle', 'Settings')}</h1>
        
        {/* Existing content ... */}
      </div>
    </ScrollArea>
  );
}
```

Similarly: Dashboard, Documents, Query, Graph pages all need a `<h1>`.

---

## Positive Findings

```
✅ SkipLink component → #main-content (correct)
✅ aria-current="page" on active nav items
✅ aria-live="polite" on chat messages (role="log")
✅ aria-live="polite" on backend status banner
✅ aria-hidden="true" on all decorative icons
✅ Graph has dedicated accessibility announcer component
✅ focus-visible:ring-2 on sidebar nav items
✅ Radix UI primitives (Dialog, Select, Tooltip) handle ARIA automatically
✅ Keyboard shortcuts hook present
```

---

## Improvement Plan by Priority

### P0 — Immediate (blocks screen reader users)

1. **A11Y-04**: Replace `div[role="button"]` → `<button>` in `folder-sidebar.tsx`
2. **A11Y-02**: Add `role="alert"` + `aria-describedby` to login form errors
3. **A11Y-05**: Wrap sidebar in `<aside aria-label="...">`

### P1 — High Impact

4. **A11Y-06**: Add `<h1 className="sr-only">` to all dashboard pages
5. **A11Y-01**: Audit all status badges — icon must accompany color, not replace it
6. **A11Y-03**: Darken input borders to 3:1 contrast ratio

### P2 — Polish

7. Add `lang` attribute audit to root layout
8. Audit `sonner` toast accessibility (should have `role="status"` or `role="alert"`)
9. Verify focus-visible ring on all interactive elements (lint pass)

---

## Testing Approach

```bash
# Install axe-core for automated audits
pnpm add -D @axe-core/playwright

# Add to e2e tests:
# edgequake_webui/e2e/accessibility.spec.ts

# Manual testing checklist:
# 1. Navigate entire app with Tab/Shift+Tab only
# 2. Use macOS VoiceOver (Cmd+F5) on login, documents, query pages
# 3. Run axe-core against each route
# 4. Test with Windows Narrator (NVDA) if possible
```

---

## External References

- [WCAG 2.1 Quick Reference](https://www.w3.org/WAI/WCAG21/quickref/)
- [Deque — ARIA Authoring Practices](https://www.deque.com/axe/)
- [Radix UI Accessibility](https://www.radix-ui.com/docs/primitives/overview/accessibility)
- [axe DevTools](https://www.deque.com/axe/devtools/)
- [WebAIM Contrast Checker](https://webaim.org/resources/contrastchecker/)
- [WCAG 1.4.11 Non-text Contrast](https://www.w3.org/WAI/WCAG21/Understanding/non-text-contrast)
