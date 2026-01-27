# Component: Buttons

## Source Files

- **Button Component**: [src/components/ui/button.tsx](../../edgequake_webui/src/components/ui/button.tsx)
- **Badge Component**: [src/components/ui/badge.tsx](../../edgequake_webui/src/components/ui/badge.tsx)

---

## Button Component

### Base Styles

- **Display**: inline-flex, items-center, justify-center
- **Typography**: 14px (text-sm), font-medium
- **Border Radius**: 6px (rounded-md)
- **Gap**: 8px (gap-2) between icon and text
- **Transition**: All properties
- **Focus**: Ring 3px with ring-ring/50
- **Disabled**: pointer-events-none, opacity-50

---

## Button Variants

### Default (Primary)

```
bg-primary text-primary-foreground hover:bg-primary/90
```

- **Background**: `var(--primary)` — `oklch(0.205 0 0)` light / `oklch(0.922 0 0)` dark
- **Text Color**: `var(--primary-foreground)` — white in light, dark in dark mode
- **Hover**: 90% opacity background
- **Usage**: Primary actions (Sign In, Save, Upload, Execute)

### Destructive

```
bg-destructive text-white hover:bg-destructive/90
```

- **Background**: `var(--destructive)` — Red tone
- **Text Color**: White
- **Hover**: 90% opacity
- **Focus Ring**: Destructive color ring
- **Dark Mode**: 60% opacity background
- **Usage**: Delete, Reset, Clear actions

### Outline

```
border bg-background shadow-xs hover:bg-accent hover:text-accent-foreground
```

- **Background**: `var(--background)` — Transparent
- **Border**: 1px solid `var(--input)`
- **Shadow**: shadow-xs
- **Hover**: `var(--accent)` background
- **Dark Mode**: bg-input/30, border-input
- **Usage**: Secondary actions (Cancel, Export, Refresh)

### Secondary

```
bg-secondary text-secondary-foreground hover:bg-secondary/80
```

- **Background**: `var(--secondary)`
- **Text Color**: `var(--secondary-foreground)`
- **Hover**: 80% opacity
- **Usage**: Alternative secondary actions

### Ghost

```
hover:bg-accent hover:text-accent-foreground
```

- **Background**: Transparent
- **Hover**: `var(--accent)` background
- **Dark Mode Hover**: 50% opacity accent
- **Usage**: Toolbar buttons, icon buttons, subtle actions

### Link

```
text-primary underline-offset-4 hover:underline
```

- **Text Color**: `var(--primary)`
- **Underline**: On hover, 4px offset
- **Usage**: Inline links, navigation links

---

## Button Sizes

| Size | Height | Padding | Icon Size | Border Radius |
|------|--------|---------|-----------|---------------|
| `sm` | 32px (h-8) | 12px horizontal | Has icon: 10px | rounded-md |
| `default` | 36px (h-9) | 16px horizontal | Has icon: 12px | rounded-md |
| `lg` | 40px (h-10) | 24px horizontal | Has icon: 16px | rounded-md |
| `icon` | 36px × 36px | - | - | rounded-md |
| `icon-sm` | 32px × 32px | - | - | rounded-md |
| `icon-lg` | 40px × 40px | - | - | rounded-md |

---

## Button States

| State | Visual Changes |
|-------|----------------|
| Default | Base variant styling |
| Hover | Background color change (per variant) |
| Focus | Ring 3px with ring-ring/50, border-ring |
| Active/Pressed | Darker background |
| Disabled | pointer-events-none, opacity 50% |
| Loading | Loader2 icon with spin animation |

---

## Usage Examples

### Primary Actions
- Sign In (Login page)
- Save (Settings page)
- Upload Documents (Documents page)
- Execute (API Explorer)
- Send (Query page)

### Outline Actions
- Export / Import (Settings)
- Refresh (Documents)
- Browse Files (Documents)

### Destructive Actions
- Reset Settings (Settings)
- Clear History (Settings)
- Delete Document (Documents)

### Ghost Actions
- Toolbar icon buttons
- Collapse/Expand toggles
- Navigation items

---

## Badge Component

### Source File

[src/components/ui/badge.tsx](../../edgequake_webui/src/components/ui/badge.tsx)

### Base Styles

- **Display**: inline-flex, items-center
- **Border Radius**: rounded-full (pill shape)
- **Typography**: 12px (text-xs), font-semibold
- **Padding**: 4px vertical, 10px horizontal
- **Border**: 1px solid transparent

### Badge Variants

| Variant | Background | Text Color | Border |
|---------|------------|------------|--------|
| `default` | `var(--primary)` | `var(--primary-foreground)` | None |
| `secondary` | `var(--secondary)` | `var(--secondary-foreground)` | None |
| `destructive` | `var(--destructive)` | White | None |
| `outline` | Transparent | `var(--foreground)` | `var(--border)` |

### HTTP Method Badges (API Explorer)

| Method | Background | Text Color | Border |
|--------|------------|------------|--------|
| GET | `bg-green-500/10` | `text-green-600` | `border-green-500/30` |
| POST | `bg-blue-500/10` | `text-blue-600` | `border-blue-500/30` |
| PUT | `bg-yellow-500/10` | `text-yellow-600` | `border-yellow-500/30` |
| PATCH | `bg-orange-500/10` | `text-orange-600` | `border-orange-500/30` |
| DELETE | `bg-red-500/10` | `text-red-600` | `border-red-500/30` |

### Status Badges (Documents)

| Status | Background | Icon | Text Color |
|--------|------------|------|------------|
| Pending | Yellow/amber | Clock | text-yellow-600 |
| Processing | Blue | Loader2 (spinning) | text-blue-600 |
| Completed | Green | CheckCircle | text-green-600 |
| Indexed | Green | CheckCircle | text-green-600 |
| Failed | Red | XCircle | text-red-600 |

---

## Usage Locations

| Component | Pages |
|-----------|-------|
| Button (default) | Dashboard, Documents, Query, Settings, Login |
| Button (outline) | Documents, Settings, Query |
| Button (destructive) | Settings |
| Button (ghost) | All pages (toolbar, navigation) |
| Badge (default) | Graph (entity types) |
| Badge (secondary) | API Explorer (counts) |
| Status Badges | Documents (document status) |
| HTTP Badges | API Explorer |

