# Component: Inputs

## Source Files

- **Input**: [src/components/ui/input.tsx](../../edgequake_webui/src/components/ui/input.tsx)
- **Textarea**: [src/components/ui/textarea.tsx](../../edgequake_webui/src/components/ui/textarea.tsx)
- **Select**: [src/components/ui/select.tsx](../../edgequake_webui/src/components/ui/select.tsx)
- **Switch**: [src/components/ui/switch.tsx](../../edgequake_webui/src/components/ui/switch.tsx)
- **Checkbox**: [src/components/ui/checkbox.tsx](../../edgequake_webui/src/components/ui/checkbox.tsx)

---

## Input Component

### Base Styles

```
h-9 w-full min-w-0 rounded-md border bg-transparent px-3 py-1 
text-base shadow-xs transition-[color,box-shadow] outline-none
```

- **Height**: 36px (h-9)
- **Width**: Full width (w-full)
- **Border**: 1px solid `var(--input)`
- **Border Radius**: 6px (rounded-md)
- **Padding**: 12px horizontal, 4px vertical
- **Typography**: 16px base, 14px on md+ screens
- **Shadow**: shadow-xs
- **Background**: Transparent (dark: bg-input/30)

### States

| State | Visual Changes |
|-------|----------------|
| Default | Border `var(--input)` |
| Focus | Border `var(--ring)`, ring 3px with ring-ring/50 |
| Disabled | pointer-events-none, cursor-not-allowed, opacity 50% |
| Invalid | Border `var(--destructive)`, ring destructive/20 |
| Placeholder | text-muted-foreground |

### Variants

| Variant | Description | Usage |
|---------|-------------|-------|
| Text | Standard text input | Username, search fields |
| Password | Masked input | Login password |
| Search | With search icon | Document search, entity search |
| File | File upload | Document upload |

### Usage Locations

- Login: Username, Password inputs
- Documents: Search input
- Query: Search conversations
- Graph: Entity search, node search
- API Explorer: (none)
- Settings: (uses Select instead)

---

## Textarea Component

### Base Styles

```
min-h-[60px] w-full rounded-md border bg-transparent px-3 py-2 
text-base shadow-xs placeholder:text-muted-foreground
```

- **Min Height**: 60px
- **Width**: Full width
- **Border**: 1px solid `var(--input)`
- **Border Radius**: 6px
- **Padding**: 12px horizontal, 8px vertical
- **Typography**: 16px base, 14px on md+ screens

### States

Same as Input component.

### Usage Locations

- Query: Query input (auto-resize)
- API Explorer: Request body JSON input

---

## Select Component (Combobox)

### Trigger Styles

```
flex h-9 w-full items-center justify-between gap-2 rounded-md border 
bg-transparent px-3 py-2 text-sm shadow-xs
```

- **Height**: 36px (h-9)
- **Layout**: Flex row with chevron icon
- **Border**: 1px solid `var(--input)`
- **Border Radius**: 6px
- **Icon**: ChevronDown (right side)

### Content Dropdown

- **Background**: `var(--popover)`
- **Border**: 1px solid `var(--border)`
- **Border Radius**: 8px
- **Shadow**: shadow-lg
- **Max Height**: Constrained with ScrollArea
- **Animation**: Fade in/out with slide

### Item Styles

```
relative flex w-full cursor-pointer items-center gap-2 rounded-sm 
py-1.5 px-2 text-sm outline-none hover:bg-accent
```

- **Hover**: bg-accent
- **Selected**: Check icon visible

### Usage Locations

- Settings: Theme, Language, Node Size, Layout, Query Mode
- Documents: Status filter
- Query: (none - uses button group)

---

## Switch Component

### Base Styles

```
peer inline-flex h-5 w-9 shrink-0 cursor-pointer items-center 
rounded-full border-2 border-transparent shadow-xs
```

- **Dimensions**: 36px × 20px (w-9 h-5)
- **Border Radius**: rounded-full (pill)
- **Thumb**: 16px × 16px circle

### States

| State | Track Color | Thumb Position |
|-------|-------------|----------------|
| Unchecked | `var(--input)` | Left |
| Checked | `var(--primary)` | Right |
| Disabled | Opacity 50%, cursor-not-allowed |
| Focus | Ring 3px |

### Usage Locations

- Settings: Show Node Labels, Show Edge Labels, Enable Streaming

---

## Checkbox Component

### Base Styles

```
peer size-4 shrink-0 rounded-[4px] border border-input 
shadow-xs focus-visible:ring-[3px]
```

- **Dimensions**: 16px × 16px (size-4)
- **Border Radius**: 4px
- **Border**: 1px solid `var(--input)`

### States

| State | Visual Changes |
|-------|----------------|
| Unchecked | Empty box |
| Checked | bg-primary, Check icon |
| Disabled | Opacity 50% |

### Usage Locations

- Documents: Row selection
- Graph: Filter checkboxes (entity types, relationship types)

---

## Form Layouts

### Standard Form Field

```
┌─────────────────────────────────────┐
│ Label (14px, medium)                │
│ ┌─────────────────────────────────┐ │
│ │ Input                           │ │
│ └─────────────────────────────────┘ │
│ Helper text (12px, muted)           │
└─────────────────────────────────────┘
```

### Settings Row Layout

```
┌─────────────────────────────────────────────────────────┐
│ ┌─────────────────────────┐  ┌────────────────────────┐ │
│ │ Label (14px, medium)    │  │ Select/Switch/Input   │ │
│ │ Description (14px,muted)│  │                        │ │
│ └─────────────────────────┘  └────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
```

---

## Validation States

### Error State

- **Border**: `var(--destructive)`
- **Ring**: destructive/20 (light), destructive/40 (dark)
- **Error Message**: text-destructive, 14px

### Success State

- **Border**: `var(--ring)` or green accent
- **Icon**: Check icon (optional)

---

## Accessibility

- All inputs have associated `<label>` elements
- Focus states with visible ring
- ARIA attributes for invalid states (aria-invalid)
- Keyboard navigation support
- Min touch target: 44px height on mobile

