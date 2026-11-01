# Lens — Design System

Parent: [README](../README.md) · Spec: [11](../11-design-system.md) · [09](../09-ux-ui-spec.md)

## The borrow, in one picture

```text
  gr8r-studio (plain JS + CSS)                EdgeQuake (React + Tailwind + shadcn)
  ─────────────────────────────               ────────────────────────────────────
  tokens.css  ────── decisions ──────►        app/globals.css (already aligned)
  components.css  ─── shapes ────────►        components/ui/* + 4 new primitives
  panels.css  ────── rhythm ────────►         Tailwind spacing scale
  settings.css  ──── settings layout ►        components/settings/shell/* (already)
  pages/members.js ─ density ────────►        components/members/* (density only)
```

Nothing is copied. There is no new stylesheet, no new dependency, no new icon set.

## Why the two systems are already close

| gr8r-studio | EdgeQuake today | Verdict |
|-------------|-----------------|---------|
| warm paper `#fbfaf8` | `--background: oklch(.988 .002 85)` | already aligned — this is why the screens will feel native |
| hairline `#e9e7e2` | `--border: oklch(.915 .004 85)` | aligned |
| `--text-2 #57544e` | `--muted-foreground: oklch(.48 0 0)` darkened for AA | aligned, EdgeQuake is stricter |
| `--fs 13.5px` | `text-[13.5px]` used in the sidebar | aligned |
| `--r 8px` | `--radius: .5rem` | identical |
| `--side-w 252px` | sidebar ≈ 252px | make exact |
| `--topbar-h 46px` | header `h-12` (48px) | keep 48; the 2px is not worth a churn |
| `--dur 140ms` | Tailwind default 150ms | use `duration-150` |
| `--gutter 24px` | `eq-gutter` utility | aligned |

So the borrow is mostly **discipline**, not color. The value is in the primitives and the rules.

## The four primitives, specified

### `SettingRow`

```text
  ┌────────────────────────────────────────────────────────────────────┐
  │ PDF parser                                  [ Edit ]               │  label 13.5/500
  │ Inherited from Acme · vision                                       │  desc 12.5/400 muted
  └────────────────────────────────────────────────────────────────────┘
   ↑ padding: 10px 0 · border-bottom: 1px solid --border (last: none)
```

```tsx
export function SettingRow({ label, description, control, tone }: {
  label: ReactNode;
  description?: ReactNode;
  control?: ReactNode;
  tone?: 'default' | 'warning' | 'danger';
}) {
  return (
    <div className="flex items-start justify-between gap-6 border-b border-border
                    py-2.5 last:border-b-0">
      <div className="min-w-0">
        <div className="text-[13.5px] font-medium text-foreground">{label}</div>
        {description && (
          <div className={cn('mt-0.5 text-[12.5px] text-muted-foreground',
                             tone === 'warning' && 'text-amber-600 dark:text-amber-400',
                             tone === 'danger'  && 'text-destructive')}>
            {description}
          </div>
        )}
      </div>
      {control && <div className="flex shrink-0 items-center gap-2">{control}</div>}
    </div>
  );
}
```

Two-line, hairline-separated, control right-aligned. That single component renders the Advanced
drawer, the grant editor, the delete receipt and the organization settings — four screens, one
shape, which is the entire reason to borrow it.

### `SegmentedControl`

```text
  ┌──────────┬──────────┬──────────┐
  │  Active  │ Pending  │ Archived │    active: bg-card + ring-1 --border + font-medium
  └──────────┴──────────┴──────────┘    inactive: text-muted-foreground, hover --muted
```

Height 26px, radius 6px, 12.5px type, 10px horizontal padding. Used for status and scope
filters. Not used for the retrieval choice (radio cards carry consequence text).

### `MeterRow`

```text
  Workspaces                                          3 of 10
  ████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
  ↑ h-1.5 rounded-full bg-muted, fill bg-primary | amber ≥80% | destructive =100%
```

Numbers as text (AA), bar `aria-hidden` (decorative). At 100% the numbers themselves turn amber,
so the information survives a monochrome screenshot.

### `CapabilityTable`

```text
  CAPABILITY                       OWNER   ADMIN   MEMBER   VIEWER
  ─────────────────────────────── ─────── ─────── ──────── ────────
  Organization
    View the organization             ✓       ✓        ✓        ✓
    Manage settings                   ✓       ✓        —        —
    Create workspaces                 ✓       ✓        —        —
    Grant the owner role              ✓       —        —        —
  Workspace
    Write content                     ✓       ✓        ✓        —
    Manage settings                   ✓       ✓        —        —
```

Group rows are `bg-muted/20` bands at 11px uppercase. ✓ is `text-success`, — is
`text-muted-foreground/40`. Row height 28px (tighter than `SettingRow`, because a matrix is
scanned, not read).

## Token decision table (what a new component may use)

| Need | Use | Never |
|------|-----|-------|
| page background | `bg-background` | `bg-white`, `bg-[#fbfaf8]` |
| card | `bg-card` + `ring-1 ring-border` | `shadow-lg` |
| hairline | `border-border` | `border-gray-200` |
| body text | `text-foreground` | `text-black` |
| secondary text | `text-muted-foreground` | `text-gray-500` |
| primary action | `bg-primary text-primary-foreground` | `bg-blue-600` |
| accent tint | `bg-primary/10` | `bg-blue-50` |
| success | `text-success` / `bg-success/10` | `text-green-600` |
| warning | `text-amber-600 dark:text-amber-400` | `text-yellow-500` |
| danger | `text-destructive` | `text-red-600` |
| radius | `rounded-md` (6) / `rounded-lg` (8) / `rounded-full` | `rounded-xl` for a 30px row |
| motion | `duration-150 ease-out` | `duration-300` |
| number | `tabular-nums` | — |

A lint script greps for the "Never" column in `components/admin/**` and fails.

## Typography scale, applied

| Role | Size | Weight | Tracking | Color |
|------|------|--------|----------|-------|
| Page title | 19px | 600 | −0.015em | `foreground` |
| Dialog title | 16px | 600 | −0.01em | `foreground` |
| Dialog lead | 13.5px | 400 | 0 | `muted-foreground` |
| Section heading (in-dialog) | 13.5px | 600 | 0 | `foreground` |
| Row label | 13.5px | 500 | 0 | `foreground` |
| Row description | 12.5px | 400 | 0 | `muted-foreground` |
| Group heading | 11px | 600 | 0.06em upper | `muted-foreground` |
| Table header | 12px | 500 | 0 | `muted-foreground` |
| Table cell | 13.5px | 400 | 0 | `foreground` |
| Badge | 11px | 500 | 0 | contextual |
| Mono (slug, id) | 12.5px | 400 | 0 | `muted-foreground` |

Six sizes. Adding a seventh requires deleting one.

## Spacing, applied

| Context | Value |
|---------|-------|
| Dialog padding | 20px (`px-5 py-4`) |
| Field vertical gap | 16px |
| Section gap inside a dialog | 20px |
| Drawer row padding | 10px vertical |
| Table row | 30px desktop / 44px touch hit area |
| Card padding | 16px |
| Page gutter | 24px |
| Between page header and content | 24px |

## Dark mode, verified

| Element | Light | Dark | Check |
|---------|-------|------|-------|
| Dialog surface | `#fff` | one step above page | not pure black |
| Hairline | darker than surface | lighter than surface | both ≥ 1.5:1 against their surface |
| Primary button | brand | brand, lightened | white text ≥ 4.5:1 both |
| Danger text | `--destructive` | lightened | ≥ 4.5:1 |
| Meter fill | brand | brand | ≥ 3:1 against track |
| Focus ring | `--ring` | `--ring` | ≥ 3:1 |
| Disabled control | 50% opacity | 50% opacity | still ≥ 3:1 for the border |

The tenant brand color is the only variable: `resolvePrimaryPalette` already returns a
contrast-checked pair per mode, and every new component reads `--primary`, so a branded tenant
gets a coherent dialog without a second code path.

## Motion contract

| Action | Animation | Duration |
|--------|-----------|----------|
| Dialog open | opacity 0→1, scale .98→1 | 150ms ease-out |
| Dialog close | reverse | 120ms ease-in |
| Drawer open | height 0→auto + opacity | 150ms ease-out |
| Drawer row hover | background only | 100ms |
| Switcher open | opacity + 4px translate | 120ms |
| Quota change | width transition on the bar | 200ms |
| `prefers-reduced-motion` | all of the above become instant | — |

No bounce, no spring, no parallax. gr8r-studio's `--spring` curve is deliberately **not** taken:
a permissions dialog is not a moment for delight.

## What makes it feel like gr8r-studio without looking copied

1. **The hairline cadence.** Every list is rows separated by one 1px line, never boxes.
2. **The muted control.** Secondary actions are ghost buttons with `muted-foreground` text, not
   outlined boxes.
3. **The quiet header.** Title, one lead sentence, one primary action. No icon soup.
4. **The dense numbers.** 30px rows, 13.5px type, tabular numerals.
5. **The single accent.** One brand-colored element per screen.

Those five are the whole aesthetic, and all five are implementable with EdgeQuake's existing
tokens.

## Review gate

```bash
# structural
rg -n "bg-white|text-gray-|text-black|border-gray-|bg-blue-|text-red-" \
   edgequake_webui/src/components/admin || echo "clean"

# screenshots
cd edgequake_webui && SPEC181_ROUND=review \
  npx playwright test e2e/spec181 --project=mock-api

# a11y
npx playwright test e2e/spec181/a11y.spec.ts --project=mock-api
```

A reviewer opens the `review/` folder at 1440, 768 and 390, in both themes, and checks
[11 §7](../11-design-system.md#7--what-a-reviewer-checks). That checklist is the definition of
"design done".
