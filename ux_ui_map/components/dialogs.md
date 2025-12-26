# Component: Dialogs

## Source Files

- **Dialog**: [src/components/ui/dialog.tsx](../../edgequake_webui/src/components/ui/dialog.tsx)
- **AlertDialog**: [src/components/ui/alert-dialog.tsx](../../edgequake_webui/src/components/ui/alert-dialog.tsx)
- **Sheet**: [src/components/ui/sheet.tsx](../../edgequake_webui/src/components/ui/sheet.tsx)
- **Popover**: [src/components/ui/popover.tsx](../../edgequake_webui/src/components/ui/popover.tsx)
- **Dropdown Menu**: [src/components/ui/dropdown-menu.tsx](../../edgequake_webui/src/components/ui/dropdown-menu.tsx)
- **Context Menu**: [src/components/ui/context-menu.tsx](../../edgequake_webui/src/components/ui/context-menu.tsx)
- **Tooltip**: [src/components/ui/tooltip.tsx](../../edgequake_webui/src/components/ui/tooltip.tsx)
- **Command**: [src/components/ui/command.tsx](../../edgequake_webui/src/components/ui/command.tsx)
- **Collapsible**: [src/components/ui/collapsible.tsx](../../edgequake_webui/src/components/ui/collapsible.tsx)

---

## Dialog Component

### Overlay

```
fixed inset-0 z-50 bg-black/50 backdrop-blur-[2px]
```

- **Background**: Black at 50% opacity
- **Backdrop**: 2px blur
- **Z-Index**: 50
- **Animation**: Fade in/out

### Content

```
fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-50 
grid w-full max-w-lg gap-4 rounded-lg border bg-background p-6 shadow-lg
```

- **Position**: Centered
- **Max Width**: 512px (max-w-lg)
- **Background**: `var(--background)`
- **Border**: 1px solid `var(--border)`
- **Border Radius**: 8px (rounded-lg)
- **Padding**: 24px
- **Shadow**: shadow-lg

### Header

- **Gap**: 8px between title and description
- **Alignment**: Left

### Footer

- **Layout**: Flex row with gap
- **Alignment**: Right (justify-end)
- **Gap**: 8px

### Usage Locations

- Documents: Pipeline status dialog
- Query: Settings sheet
- Graph: Search command

---

## AlertDialog Component

Confirmation dialogs for destructive actions.

### Structure

Same as Dialog but with specific semantics:
- **Title**: AlertDialogTitle
- **Description**: AlertDialogDescription
- **Actions**: Cancel (secondary), Confirm (primary/destructive)

### Action Buttons

| Button | Variant | Position |
|--------|---------|----------|
| Cancel | Outline/secondary | Left |
| Confirm | Primary or Destructive | Right |

### Usage Locations

- Settings: Clear History, Reset Settings
- Documents: Delete document confirmation
- Graph: Delete entity/relationship

---

## Sheet Component

Side panel overlay (mobile navigation, settings panel).

### Overlay

Same as Dialog overlay.

### Content

```
fixed z-50 bg-background shadow-lg
```

- **Position**: Based on `side` prop
- **Sides**: top, right, bottom, left
- **Animation**: Slide in from side

### Side Dimensions

| Side | Dimensions |
|------|------------|
| `right` | w-3/4 max-w-sm (right edge) |
| `left` | w-3/4 max-w-sm (left edge) |
| `top` | h-auto (top edge) |
| `bottom` | h-auto (bottom edge) |

### Usage Locations

- Mobile navigation: Sidebar as sheet from left
- Query: Conversation history panel (mobile)
- Query: Advanced settings sheet (right)

---

## Popover Component

### Trigger

Any clickable element.

### Content

```
z-50 w-72 rounded-md border bg-popover p-4 text-popover-foreground shadow-md
```

- **Width**: 288px (w-72)
- **Background**: `var(--popover)`
- **Border**: 1px solid `var(--border)`
- **Border Radius**: 6px
- **Padding**: 16px
- **Shadow**: shadow-md

### Usage Locations

- Graph: Node details popover
- Header: Theme toggle, language selector

---

## Dropdown Menu Component

### Trigger

Button with chevron or icon.

### Content

```
z-50 min-w-[8rem] overflow-hidden rounded-md border bg-popover p-1 
text-popover-foreground shadow-md
```

- **Min Width**: 128px
- **Background**: `var(--popover)`
- **Border Radius**: 6px
- **Padding**: 4px

### Item

```
relative flex cursor-pointer items-center gap-2 rounded-sm px-2 py-1.5 
text-sm outline-none hover:bg-accent
```

- **Height**: Auto (py-1.5)
- **Hover**: bg-accent
- **Gap**: 8px (icon + text)

### Usage Locations

- Header: User menu, theme toggle, language selector
- Documents: Row actions menu
- Graph: Layout control, export options

---

## Context Menu Component

Right-click menu.

### Structure

Same as Dropdown Menu but triggered by right-click.

### Usage Locations

- Graph: Node context menu (View Details, Expand, Query)

---

## Tooltip Component

### Trigger

Any hoverable element.

### Content

```
z-50 overflow-hidden rounded-md bg-primary px-3 py-1.5 text-xs 
text-primary-foreground shadow-md
```

- **Background**: `var(--primary)`
- **Text**: `var(--primary-foreground)`
- **Typography**: 12px (text-xs)
- **Border Radius**: 6px
- **Padding**: 6px vertical, 12px horizontal
- **Animation**: Fade in with slight slide

### Usage Locations

- Sidebar: Navigation item tooltips (collapsed state)
- Graph: Button tooltips (zoom controls, toolbar)
- API Explorer: Endpoint action tooltips

---

## Command Component (⌘K)

Command palette / search interface.

### Dialog Wrapper

Opens in Dialog overlay.

### Input

```
flex h-10 w-full rounded-md bg-transparent py-3 text-sm outline-none 
placeholder:text-muted-foreground
```

- **Height**: 40px
- **Icon**: Search icon (left)
- **Placeholder**: "Type a command or search..."

### List

```
max-h-[300px] overflow-y-auto
```

- **Max Height**: 300px
- **Overflow**: Scrollable

### Item

```
relative flex cursor-pointer items-center gap-2 rounded-sm px-2 py-3 
text-sm hover:bg-accent
```

### Usage Locations

- Graph: Node search (⌘K shortcut)

---

## Collapsible Component

Expandable/collapsible sections.

### Trigger

```
flex items-center gap-2 w-full p-2 hover:bg-muted rounded text-sm font-medium
```

- **Icon**: ChevronDown (expanded) / ChevronRight (collapsed)
- **Hover**: bg-muted

### Content

- **Animation**: Height transition
- **Hidden**: When collapsed

### Usage Locations

- API Explorer: Category groups
- Sidebar: Navigation sections

---

## Avatar Component

### Source File

[src/components/ui/avatar.tsx](../../edgequake_webui/src/components/ui/avatar.tsx)

### Sizes

| Size | Dimensions |
|------|------------|
| Default | 40px × 40px |
| sm | 32px × 32px |
| lg | 48px × 48px |

### States

- **Image**: Shows user avatar image
- **Fallback**: Initials or icon

### Usage Locations

- Header: User menu avatar
- Query: Message avatars (user, assistant)

---

## Animation Patterns

| Component | Enter Animation | Exit Animation |
|-----------|-----------------|----------------|
| Dialog | Fade + scale from 95% | Fade + scale to 95% |
| Sheet | Slide from side | Slide to side |
| Dropdown | Fade + scale from 95% | Fade out |
| Tooltip | Fade + slide | Fade out |
| Collapsible | Height transition | Height transition |

