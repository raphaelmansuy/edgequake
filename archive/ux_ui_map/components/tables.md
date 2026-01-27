# Component: Tables

## Source File

- **Table Component**: [src/components/ui/table.tsx](../../edgequake_webui/src/components/ui/table.tsx)

---

## Table Component

### Base Styles

```
w-full caption-bottom text-sm
```

- **Width**: Full width
- **Typography**: 14px (text-sm)
- **Caption**: Bottom position

---

## Table Sub-Components

### TableHeader

```
[&_tr]:border-b
```

- **Border**: Bottom border on rows
- **Background**: Transparent (inherits from parent)

### TableBody

```
[&_tr:last-child]:border-0
```

- **Border**: Bottom border on all rows except last

### TableFooter

```
bg-muted/50 font-medium [&>tr]:last:border-b-0
```

- **Background**: `var(--muted)` at 50%
- **Typography**: font-medium

### TableRow

```
border-b transition-colors hover:bg-muted/50 data-[state=selected]:bg-muted
```

- **Border**: 1px bottom border
- **Hover**: bg-muted at 50%
- **Selected**: bg-muted (full)
- **Transition**: Color transition

### TableHead

```
h-10 px-2 text-left align-middle font-medium text-muted-foreground
```

- **Height**: 40px (h-10)
- **Padding**: 8px horizontal
- **Typography**: font-medium, text-muted-foreground
- **Alignment**: Left

### TableCell

```
p-2 align-middle
```

- **Padding**: 8px
- **Alignment**: Middle vertical

### TableCaption

```
mt-4 text-sm text-muted-foreground
```

- **Margin**: 16px top
- **Typography**: 14px, muted

---

## Table Variants

### Documents Table

| Column | Width | Content |
|--------|-------|---------|
| Checkbox | 40px | Selection checkbox |
| Name | flex | Filename with icon + truncated path |
| Status | 100px | Status badge |
| Size | 80px | File size formatted |
| Created | 120px | Relative time |
| Actions | 40px | More options dropdown |

### Responsive Behavior

- **Desktop**: Full table with all columns
- **Tablet**: Horizontal scroll
- **Mobile**: Card-based layout or simplified columns

---

## Interactive Features

### Row Selection

- **Checkbox**: First column
- **Selected State**: bg-muted background
- **Multi-select**: Shift+click for range

### Sortable Headers

- **Indicator**: Sort icon (ascending/descending)
- **Click**: Toggle sort direction
- **Active**: Text highlighted

### Row Actions

- **Trigger**: MoreVertical icon button
- **Menu**: Dropdown with options
- **Options**: View, Edit, Delete, etc.

---

## Empty State

When table has no data:

```
┌─────────────────────────────────────┐
│                                     │
│         [Large Icon]                │
│                                     │
│    "No documents yet"               │
│    Upload documents to build...     │
│                                     │
│        [Primary Action]             │
│                                     │
└─────────────────────────────────────┘
```

---

## Usage Locations

| Table | Page | Columns |
|-------|------|---------|
| Documents | Documents | Name, Status, Size, Created, Actions |
| Entities | Graph | (Entity list in panel) |
| Conversations | Query | (History list in panel) |

---

## Accessibility

- Proper `<table>`, `<thead>`, `<tbody>` semantics
- `scope="col"` on header cells
- Keyboard navigation (Tab between interactive elements)
- Screen reader announcements for sort changes
- Row selection announced

