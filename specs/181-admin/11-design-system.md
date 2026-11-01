# 11 — Design system (borrowed from gr8r-studio)

Parent: [README](README.md) · Prev: [10-frontend-architecture](10-frontend-architecture.md) ·
Next: [12-edge-cases](12-edge-cases.md)

Source of inspiration: `/Users/raphaelmansuy/Github/03-working/gr8r-studio`, specifically
`src/styles/tokens.css`, `src/styles/components.css`, `src/styles/panels.css`,
`src/styles/pages/settings.css` and `src/pages/{members,settings}.js`.

gr8r-studio is plain ES modules plus hand-written CSS with a custom-property token layer.
EdgeQuake is React + Next + Tailwind v4 + shadcn. We therefore take the **decisions**, not the
stylesheets. Nothing here adds a dependency or a component library.

---

## 1 — What we take, and what we leave

| gr8r-studio | Take? | Why |
|-------------|-------|-----|
| Warm paper background (`#fbfaf8`) instead of pure white | **yes** | already in EdgeQuake's `globals.css` (`--background: oklch(0.988 0.002 85)`) — keep it, never drift to `#fff` |
| Hairline borders (`#e9e7e2`), not grey blocks | **yes** | maps to `--border` |
| 13.5px body / 12.5px secondary type scale | **yes** | density that reads as "tool", not "marketing site" |
| 4pt spacing scale (`--s-1`…`--s-12`) | **yes** | Tailwind's 4px step already equals it |
| `--gutter: 24px` shared content edge | **yes** | already implemented as `eq-gutter` in the header |
| Sidebar `252px`, row `30px`, topbar `46px` | **yes (252 / 30)** | EdgeQuake's rail is already 252-ish; make it exact |
| `srow` = label + description + control on one row, hairline between | **yes** | the right primitive for grant rows, quota rows, inherit rows |
| `sblock` = a titled group of rows | **yes** | |
| Segmented control (`seg`) for 2–4 exclusive options | **yes** | replaces radio groups in the drawer |
| Pill buttons (`pillbtn bordered`) for inline enum change | **yes** | the role chip in the members table |
| `permsTable()` — capability × role with ✓ / — | **yes** | exactly the matrix [SPEC-169](../../edgequake_webui/src/components/members/roles-tab.tsx) already builds; borrow the density |
| Tabs `Members | Teams | Roles & permissions` | **yes** | People page shape |
| Theme cards with mini previews | no | EdgeQuake has its own appearance section |
| Billing / plan cards / invoices | no | not in scope |
| 2FA / sessions / recovery codes | no | out of scope |
| Plain-JS render loop, `data-keep` DOM preservation | **no** | React + TanStack Query already solves it |
| `lucide` icon-subset Vite plugin | no | Next handles it |
| Accent swatches (`data-accent`) | **partially** | EdgeQuake's tenant brand color already does this; keep one mechanism |

---

## 2 — Token mapping

The gr8r-studio token names are the *semantics*. The EdgeQuake CSS variables are the
*implementation*. There is no new token file; this is a mapping table so a reviewer can check
that a new component uses the right one.

| gr8r token | gr8r value (light) | EdgeQuake variable | Notes |
|-----------|--------------------|--------------------|-------|
| `--bg` | `#fbfaf8` | `--background` | already |
| `--surface` | `#ffffff` | `--card` | |
| `--surface-2` | `#f6f5f2` | `--muted` | |
| `--sunken` | `#f7f6f3` | `--muted/40` | inset panels |
| `--bg-side` | `#f4f3f0` | `--sidebar` | |
| `--border` | `#e9e7e2` | `--border` | hairline |
| `--border-strong` | `#dad7d0` | `--input` | interactive edge, 3:1 |
| `--text` | `#1d1c1a` | `--foreground` | |
| `--text-2` | `#57544e` | `--muted-foreground` | AA at 12.5px |
| `--accent` | `#4b5bd6` | `--primary` **or** tenant brand | tenant brand wins when set |
| `--accent-soft` | 10% accent | `bg-primary/10` | |
| `--green` / `--red` / `--amber` | semantic | `--success` / `--destructive` / `--warning` | |
| `--r: 8px` | | `--radius` | |
| `--r-full` | 999px | `rounded-full` | badges, chips |
| `--fs-2xs: 11px` | | `text-[11px]` | uppercase group headings |
| `--fs-xs: 12px` | | `text-xs` | secondary |
| `--fs-sm: 12.5px` | | `text-[12.5px]` | row descriptions |
| `--fs: 13.5px` | | `text-[13.5px]` | body, table cells |
| `--fs-xl: 19px` | | `text-xl` | page title |
| `--dur: 140ms` | | `duration-150` | snappier than Tailwind's 200 |
| `--ease: cubic-bezier(.23,1,.32,1)` | | `ease-out` | close enough; no custom curve |

**One rule.** A new component in this pack may use only these variables. No literal hex, no
`text-gray-500`, no `bg-white`. The dark theme then costs nothing.

---

## 3 — The four primitives we add

Everything in [09](09-ux-ui-spec.md) is built from four new components plus existing shadcn
primitives. That is the whole design-system contribution.

### 3.1 `SettingRow` (gr8r `srow`)

```text
+--------------------------------------------------------------------------+
| Label                                        [ control ]                |
| Description line in --text-2 at 12.5px                                   |
+--------------------------------------------------------------------------+
```

```tsx
<SettingRow
  label="PDF parser"
  description="Inherited from Acme · vision"
  control={<Button variant="ghost" size="sm">Edit</Button>}
/>
```

Used by: the Advanced drawer (6 rows), the grant editor (n grants), the delete receipt
(9 planes), the organization settings page.

### 3.2 `SegmentedControl` (gr8r `seg`)

```text
  ┌────────┬────────┐
  │ Graph  │ Normal │      active: card bg + hairline + weight 500
  └────────┴────────┘
```

For 2–4 exclusive options where the descriptions fit below. The retrieval choice in
[W-03](09-ux-ui-spec.md#w-03-create-workspace--the-three-decision-dialog) uses radio **cards**
instead, because each option carries two lines of consequence text; the segmented control is
for terse enums (status filters, scope filters).

### 3.3 `MeterRow` (gr8r `.meter`)

```text
  Workspaces                                    3 of 10
  ███░░░░░░░
```

Amber at ≥ 80%, red at 100%. Used in [W-01](09-ux-ui-spec.md#w-01-header-switcher),
[W-03](09-ux-ui-spec.md#w-03-create-workspace--the-three-decision-dialog),
[W-06](09-ux-ui-spec.md#w-06-organization-page-organization-owner),
[W-07](09-ux-ui-spec.md#w-07-platform-admin--deployment--organizations).

Accessibility: the numbers are text, the bar is `aria-hidden`. A screen reader hears
"3 of 10 workspaces", not "progressbar 30 percent".

### 3.4 `CapabilityTable` (gr8r `permsTable`)

Already exists as
[`roles-tab.tsx::CapabilityMatrix`](../../edgequake_webui/src/components/members/roles-tab.tsx).
The change is data, not markup: it renders `GET /roles` filtered by `axis`, so the workspace
rows appear under workspace columns. Density borrowed from gr8r: 12.5px rows, group separators
as `bg-muted/20` bands, ✓ in `--success`, `—` in `--muted-foreground/40`.

---

## 4 — Composition rules (the "calm" part)

These are what make gr8r-studio feel quiet. They are rules, not suggestions.

| Rule | Detail |
|------|--------|
| **One accent per screen** | Only the primary action and one active state use `--primary`. Everything else is ink, hairline or `--muted`. |
| **Hairline over box** | Separate with a 1px border, not a shadow or a grey fill. Cards get `--shadow-card` (1px ring), not elevation. |
| **No nested cards** | A card inside a card is a layout bug. The drawer inside a dialog is a bordered panel, not another card. |
| **Type does the hierarchy** | Weight 600 for the title, 500 for a row label, 400 for description. Size steps are 11 / 12 / 12.5 / 13.5 / 19. No size jump without a weight change. |
| **Numbers are tabular** | `tabular-nums` on every count, quota and date column. |
| **Rows are 30px, not 44px** | Density. Touch targets on mobile get a transparent 44px hit area via padding, not by growing the row. |
| **Motion ≤ 140ms, ease-out** | Dialogs fade+scale 0.98→1. The drawer slides 8px. Nothing bounces. Respect `prefers-reduced-motion`. |
| **Never shift layout** | The quota strip reserves its height whether or not it has content; the switcher reserves the hint line. CLS budget 0. |
| **Empty is drawn** | Every list has a designed empty state with one action ([LAW-181-12](01-first-principles.md)). |
| **One warning color per dialog** | Amber for "you can fix this" (quota), red for "this destroys data" (delete). Never both. |

---

## 5 — Screen-by-screen delta from today

| Screen | Today | After | Design tokens involved |
|--------|-------|-------|------------------------|
| Header switcher | two `CommandGroup`s, flat list, create rows ungated | grouped by axis, role chip per workspace, quota footer, gated create | `--muted` heading band, `--r-sm` rows, `MeterRow` |
| Create tenant | 8-step dialog, `max-w-4xl`, stepper bar | 3-decision dialog, `max-w-lg`, no stepper | `--fs-xl` title, `SettingRow`, `SegmentedControl` |
| Create workspace | 8-step dialog, `max-w-4xl` | 3-decision dialog, `max-w-lg` | same |
| Members | table, single `Role` column | table with `SCOPE` column, grant rows, `+n more` | `--row-h`, pill chips, `tabluar-nums` |
| Roles & permissions | org matrix only | axis-aware matrix | ✓/— density from gr8r `permsTable` |
| Settings nav | `You / Workspace / Organization / Deployment` | same, plus `Organizations` and `Workspaces` sections | `--fs-2xs` uppercase headings |
| Organization page | **does not exist** | [W-06](09-ux-ui-spec.md#w-06-organization-page-organization-owner) | `StatStrip`, list rows |
| Deployment > Organizations | a small card in Observability | [W-07](09-ux-ui-spec.md#w-07-platform-admin--deployment--organizations) table with usage meters | `CapabilityTable` density |
| Tenant guard empty states | one card, create button | 5 role-aware states | `--surface-2` illustration plate, one primary action |

---

## 6 — Dark theme

gr8r-studio redefines tokens for dark rather than inverting. EdgeQuake already does the same
(`.dark` block in `globals.css`). The rules that matter for the new screens:

| Aspect | Light | Dark |
|--------|-------|------|
| Page | warm paper `oklch(.988 .002 85)` | `#1a1a19`-equivalent |
| Card | pure white | one step up from page, never black |
| Hairline | darker than the card | lighter than the card |
| Accent | tenant brand, contrast-checked | auto-lightened (existing `resolvePrimaryPalette`) |
| Danger | `oklch(.53 .235 27)` | lightened to keep 4.5:1 |
| Meter fill | brand at 100%, amber/red overrides | same hue, lower chroma |

The brand-color picker already computes WCAG AA for both modes
([`tenant-primary.ts`](../../edgequake_webui/src/lib/theme/tenant-primary.ts)); the new screens
inherit that and must not introduce a literal color that bypasses it.

---

## 7 — What a reviewer checks

```text
[ ] No literal color outside the token table (§2)
[ ] Every list has an empty state with exactly one action
[ ] Every inheritable value prints its layer and value
[ ] Every quota display shows used / limit as text
[ ] Primary action is the only --primary element on screen
[ ] Row height 30px desktop; 44px hit area on mobile via padding
[ ] Numbers are tabular
[ ] Focus ring visible on every interactive element (3:1)
[ ] Dialog traps focus; Esc with edits asks to discard
[ ] prefers-reduced-motion honoured
[ ] CLS 0 on the switcher and both dialogs
```

These become the [14 §visual gates](14-e2e-test-matrix.md#visual-gates) screenshot checklist.
