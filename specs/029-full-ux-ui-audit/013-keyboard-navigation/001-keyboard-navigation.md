# 001 — Keyboard Navigation Audit

**First Principle: Flow** — Power users should never need to reach for the mouse.

---

## Tab Order Audit

### Dashboard Layout Tab Sequence

```
Expected tab order:
1. Skip link (sr-only, appears on focus)
2. Sidebar logo link
3. Workspace selector
4. Sidebar nav items (10 items × 1 tab stop each)
5. Sidebar collapse toggle
6. Header: mobile sidebar trigger (mobile only)
7. Header: tenant selector (desktop)
8. Header: theme toggle
9. Header: user menu
10. Main content area (tabIndex={-1}, skip with programmatic focus)
11. Content-specific interactive elements
```

**KN-01 · The Skip Link Doesn't Skip Enough**

The skip link jumps to `#main-content`. However, the main content has `tabIndex={-1}` — this means after pressing Enter on the skip link, focus moves to the main content div but then the next Tab press goes back to the normal tab order (after the `#main-content` element in DOM order).

For the skip link to work properly:
1. `#main-content` needs `tabIndex={-1}` (already set) ✓
2. After the skip link activates, focus must be managed to land inside the first interactive element in main content

**Fix:**
```typescript
// skip-link.tsx
<a
  href="#main-content"
  onClick={(e) => {
    e.preventDefault();
    const main = document.getElementById('main-content');
    if (main) {
      main.focus(); // Focus the element
      // Then Tab will move to first child interactive element
    }
  }}
  className="sr-only focus:not-sr-only ..."
>
  Skip to main content
</a>
```

**KN-02 · Sidebar Has 10+ Tab Stops**

Every nav item is a separate tab stop. Reducing to 5-7 items (as recommended in navigation audit) directly reduces the tab burden.

Additionally, when the sidebar is collapsed, the tooltips should be triggered by focus (already handled by `TooltipProvider` + Radix).

### Document Table Keyboard Navigation

**KN-03 · Table Rows Are Not Navigable by Arrow Keys**

Standard data table keyboard navigation:
- `Tab` → moves between interactive controls (not rows)
- `↑ ↓` → moves between rows
- `Enter` / `Space` → activates row action

The current implementation uses `onClick` and `onDoubleClick` on rows but there's no arrow key navigation between rows.

**Fix:**

```typescript
// document-table-row.tsx
<TableRow
  tabIndex={0}
  onKeyDown={(e) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      onRowClick(doc);
    }
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      (e.currentTarget.nextSibling as HTMLElement)?.focus();
    }
    if (e.key === 'ArrowUp') {
      e.preventDefault();
      (e.currentTarget.previousSibling as HTMLElement)?.focus();
    }
  }}
  ...
>
```

### Modal/Dialog Focus Management

**KN-04 · Verify Focus Trap in All Dialogs**

Radix UI's `Dialog` component handles focus trap automatically. However, custom modal-like components need verification:

Dialogs to audit:
- `DuplicateUploadDialog` — uses Radix `Dialog` ✓ (likely)
- `ReprocessDialog` — uses Radix `Dialog` ✓ (likely)
- `BulkReprocessDialog` — uses Radix `Dialog` ✓ (likely)
- `DocumentDetailDialog` — uses Radix `Dialog` ✓ (likely)
- `DocumentViewerDialog` — check for focus trap
- Graph `EntityEditDialog` — check for focus trap
- `ClearDocumentsDialog` — check for focus trap

**Verify pattern:**
```typescript
// All dialogs should use the shadcn Dialog primitive:
import { Dialog, DialogContent } from "@/components/ui/dialog";
// NOT: custom div with role="dialog" without focus management
```

**KN-05 · Alert Dialog Focus on Cancel vs. Destructive Button**

When `AlertDialog` opens for a destructive action (delete), focus should land on the **Cancel** button, not the destructive **Confirm** button:

```typescript
// alert-dialog.tsx (shadcn/ui)
// VERIFY: Default focus lands on cancel, not confirm
// The Radix AlertDialog spec requires focus to start on the
// least destructive action
```

---

## Keyboard Shortcuts Audit

### Existing Shortcuts (from `useKeyboardShortcuts`)

The `useKeyboardShortcuts` hook is used in `DashboardLayout`. Without reading its implementation, common patterns are:

```typescript
// Likely shortcuts:
Ctrl/Cmd + K  → Command palette / search
Ctrl/Cmd + N  → New conversation
?            → Show keyboard shortcut help
```

**KN-06 · Keyboard Shortcuts Not Discoverable**

Users who don't know about keyboard shortcuts won't find them. Add a keyboard shortcut help panel accessible via:

1. `?` key (already common in apps)
2. Footer `[?]` button in sidebar
3. Help menu

The `keyboard-shortcuts-help.tsx` file exists in the graph component directory. Check if it's surfaced application-wide.

**KN-07 · Document Table: Keyboard Selection**

The `useDocumentKeyboard` hook exists — verify it implements:

```
Ctrl/Cmd + A  → Select all visible documents
Shift + Click → Range selection
Escape        → Clear selection
Delete        → Delete selected (with confirmation)
```

---

## Focus Visible Audit

### Focus Style Consistency

The codebase uses `focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2` on most interactive elements.

**KN-08 · Missing focus-visible on some elements**

Potential gaps found during grep:
- `folder-sidebar.tsx` `div[role="button"]` elements — replaced by `<button>` (A11Y-04) will fix this
- Any `<a>` tags used as buttons without explicit focus styles

**Check pattern:**
```bash
# Find interactive elements without focus-visible:
grep -r "onClick" src/components --include="*.tsx" | \
  grep -v "focus-visible" | \
  grep -v "button\|Button\|Link\|input"
```

### Focus Indicator Visual Specification

```
Component          Focus Style              Contrast vs. BG
─────────────────────────────────────────────────────────────
Navigation links   ring-2 primary offset-2  ✅ Good
Buttons           ring-2 primary offset-2   ✅ Good  
Table rows        ring-2 primary (no offset) ⚠️ Need offset
Input fields      ring-2 primary/30         ⚠️ Low contrast
Sidebar items     ring-2 primary offset-2   ✅ Good
```

The input focus ring `focus-visible:ring-primary/30` (30% opacity) may not meet 3:1 contrast requirement.

---

## Navigation Flow Testing

### Critical User Flows — Keyboard Only

**Flow 1: Upload a document (keyboard only)**

```
1. Tab to [Upload] button in Documents toolbar
2. Enter → file picker opens (browser native)
3. Select file → Enter
4. Upload begins automatically
5. Tab to table → new row appears
6. Arrow down to new row → status shows
```

**Flow 2: Submit a query (keyboard only)**

```
1. Tab to [Query] nav item
2. Enter → navigate to query page
3. Tab to textarea
4. Type query
5. Enter to submit (NOT Tab then Enter on button)
6. Response renders
7. Tab to copy button on response
8. Enter to copy
```

**Flow 3: Select and delete documents (keyboard only)**

```
1. Navigate to Documents
2. Tab to first row
3. Space → row selected (checkbox)
4. Arrow down → next row
5. Space → range select
6. Tab to batch action bar
7. Tab to Delete button
8. Enter → alert dialog opens
9. Tab should be trapped inside dialog
10. Enter on Cancel OR Tab to Confirm + Enter
```

---

## Positive Keyboard Support Found

```
✅ Skip link implemented
✅ Global keyboard shortcuts hook
✅ aria-current="page" on nav items
✅ useDocumentKeyboard hook for table keyboard navigation
✅ Enter to submit query (Shift+Enter for newline)
✅ Radix UI primitives handle modal focus traps
✅ focus-visible: ring styles on most interactive elements
✅ TooltipProvider with keyboard-triggered tooltips
```

---

## External References

- [Keyboard Navigation Patterns — WCAG 2.1](https://www.w3.org/WAI/WCAG21/Understanding/keyboard)
- [Managing Focus — WAI-ARIA](https://www.w3.org/WAI/ARIA/apg/practices/keyboard-interface/)
- [Data Tables — ARIA Authoring Practices](https://www.w3.org/WAI/ARIA/apg/patterns/table/)
- [Modal Dialog — ARIA Authoring Practices](https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/)
- [Focus Management in SPA — Marcy Sutton](https://www.gatsbyjs.com/blog/2019-07-11-user-testing-accessible-client-routing/)
