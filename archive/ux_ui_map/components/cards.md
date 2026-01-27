# Component: Cards

## Source File

- **Card Component**: [src/components/ui/card.tsx](../../edgequake_webui/src/components/ui/card.tsx)

---

## Card Component

### Base Styles

```
bg-card text-card-foreground flex flex-col gap-6 rounded-xl border py-6 shadow-sm
```

- **Background**: `var(--card)` — `oklch(1 0 0)` light / `oklch(0.205 0 0)` dark
- **Text Color**: `var(--card-foreground)`
- **Border**: 1px solid `var(--border)`
- **Border Radius**: 12px (rounded-xl)
- **Shadow**: shadow-sm
- **Layout**: Flex column
- **Gap**: 24px between children
- **Padding**: 24px vertical

---

## Card Sub-Components

### CardHeader

```
grid auto-rows-min grid-rows-[auto_auto] items-start gap-2 px-6
```

- **Layout**: CSS Grid with auto rows
- **Padding**: 24px horizontal
- **Gap**: 8px between title and description
- **With Action**: 2-column grid layout

### CardTitle

```
leading-none font-semibold
```

- **Typography**: font-semibold, line-height 1
- **Default Size**: Inherits from parent (typically 16-18px)

### CardDescription

```
text-muted-foreground text-sm
```

- **Typography**: 14px (text-sm)
- **Color**: `var(--muted-foreground)`

### CardContent

```
px-6
```

- **Padding**: 24px horizontal
- **No vertical padding**: Relies on parent gap

### CardFooter

```
flex items-center px-6
```

- **Layout**: Flex row
- **Padding**: 24px horizontal
- **With Border**: pt-6 when border-t present

### CardAction

```
col-start-2 row-span-2 row-start-1 self-start justify-self-end
```

- **Position**: Top-right of header
- **Grid**: Spans 2 rows

---

## Card Variants

### Standard Card

Used for content sections throughout the application.

| Property | Value |
|----------|-------|
| Background | `var(--card)` |
| Border | 1px solid `var(--border)` |
| Shadow | shadow-sm |
| Radius | 12px |

### Stats Card (Dashboard)

Special variant with gradient accent.

| Variant | Gradient |
|---------|----------|
| documents | Blue gradient (`from-blue-500/10 to-blue-600/5`) |
| entities | Purple gradient (`from-purple-500/10 to-purple-600/5`) |
| relationships | Emerald gradient (`from-emerald-500/10 to-emerald-600/5`) |
| types | Orange gradient (`from-orange-500/10 to-orange-600/5`) |

**Source**: [src/components/dashboard/stats-card.tsx](../../edgequake_webui/src/components/dashboard/stats-card.tsx)

### Danger Card (Settings)

Used for destructive actions section.

```
border-destructive/30
```

- **Border**: 1px solid `var(--destructive)/30`
- **Title Color**: text-destructive
- **Icon Color**: text-destructive

### Message Card (Query)

Chat message bubbles.

| Type | Alignment | Background | Radius |
|------|-----------|------------|--------|
| User | Right | `var(--primary)` | 16px, 4px top-right |
| Assistant | Left | `var(--card)` + border | 16px, 4px top-left |

---

## Card Layouts

### Single Column (Settings)

```
┌─────────────────────────────────────┐
│ CardHeader                          │
│ ├── Icon + Title                    │
│ └── Description                     │
├─────────────────────────────────────┤
│ CardContent                         │
│ ├── Setting Row 1                   │
│ ├── Separator                       │
│ ├── Setting Row 2                   │
│ └── ...                             │
└─────────────────────────────────────┘
```

### Grid Layout (Dashboard)

```
┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐
│ Stats  │ │ Stats  │ │ Stats  │ │ Stats  │
│ Card 1 │ │ Card 2 │ │ Card 3 │ │ Card 4 │
└────────┘ └────────┘ └────────┘ └────────┘
```

- Desktop: 4 columns (lg:grid-cols-4)
- Tablet: 2 columns (md:grid-cols-2)
- Mobile: 1 column

### Split Layout (Documents)

```
┌─────────────────────────────┬───────────────┐
│ Main Card                   │ Preview Card  │
│ (Documents list)            │ (collapsible) │
└─────────────────────────────┴───────────────┘
```

---

## Usage Locations

| Component | Pages |
|-----------|-------|
| Standard Card | Dashboard, Documents, Query, Settings |
| Stats Card | Dashboard (metrics) |
| Danger Card | Settings (data management) |
| Message Card | Query (chat messages) |
| Panel Card | Graph (entity browser, details), Query (history) |
| Login Card | Login page |

---

## Responsive Behavior

| Breakpoint | Card Padding | Grid Columns |
|------------|--------------|--------------|
| Mobile | 16px | 1 column |
| Tablet | 24px | 2 columns |
| Desktop | 24px | 4 columns (max) |

---

## Accessibility

- Cards use semantic HTML (no role needed for div containers)
- Interactive cards should be `<button>` or have button role
- Focus states on clickable cards
- Adequate color contrast for text on card background

