# Page: API Explorer

## Overview

- **Route**: `/api-explorer`
- **Title**: "API Explorer"
- **Layout**: Two-panel split layout with endpoint list (left) and request/response area (right)
- **Source File**: [src/app/(dashboard)/api-explorer/page.tsx](../../edgequake_webui/src/app/(dashboard)/api-explorer/page.tsx)
- **Main Component**: [src/components/shared/api-explorer.tsx](../../edgequake_webui/src/components/shared/api-explorer.tsx)

## Layout Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│ ┌───────────────┬─────────────────────────────────────────────────┐ │
│ │               │ Header (64px)                                   │ │
│ │               ├─────────────────────────────────────────────────┤ │
│ │   Sidebar     │ Breadcrumb: EdgeQuake > API Explorer            │ │
│ │   (64px)      ├──────────────────────┬──────────────────────────┤ │
│ │               │                      │                          │ │
│ │   Nav Icons   │ API Endpoints        │ Request/Response Area    │ │
│ │               │ (320px)              │ (flexible)               │ │
│ │               │                      │                          │ │
│ │               │ ┌──────────────────┐ │ ┌──────────────────────┐ │ │
│ │               │ │ Health ▼     (1) │ │ │ Empty State:         │ │ │
│ │               │ │   GET /health    │ │ │ "Select an Endpoint" │ │ │
│ │               │ ├──────────────────┤ │ │                      │ │ │
│ │               │ │ Auth ▼       (2) │ │ │ Or when selected:    │ │ │
│ │               │ │   POST /auth/... │ │ │ ┌────────────────┐   │ │ │
│ │               │ │   GET /auth/me   │ │ │ │ Endpoint Header│   │ │ │
│ │               │ ├──────────────────┤ │ │ ├────────────────┤   │ │ │
│ │               │ │ Documents ▼  (4) │ │ │ │ Request Body   │   │ │ │
│ │               │ │   GET /documents │ │ │ │ (textarea)     │   │ │ │
│ │               │ │   POST /documents│ │ │ ├────────────────┤   │ │ │
│ │               │ │   ...            │ │ │ │ Response       │   │ │ │
│ │               │ ├──────────────────┤ │ │ │ (scroll area)  │   │ │ │
│ │               │ │ Query ▼      (1) │ │ │ └────────────────┘   │ │ │
│ │               │ │ Graph ▼      (3) │ │ │                      │ │ │
│ │               │ │ Entities ▼   (5) │ │ │                      │ │ │
│ │               │ │ Relationships (2)│ │ │                      │ │ │
│ │               │ │ Pipeline ▼   (1) │ │ │                      │ │ │
│ │               │ └──────────────────┘ │ └──────────────────────┘ │ │
│ └───────────────┴──────────────────────┴──────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────┘
```

## Screenshots

| Viewport | Screenshot |
|----------|------------|
| Desktop (1440px) | [api-explorer-desktop.png](../screenshots/api-explorer/api-explorer-desktop.png) |
| Desktop with endpoint | [api-explorer-desktop-with-endpoint.png](../screenshots/api-explorer/api-explorer-desktop-with-endpoint.png) |
| Tablet (768px) | [api-explorer-tablet.png](../screenshots/api-explorer/api-explorer-tablet.png) |
| Mobile (375px) | [api-explorer-mobile.png](../screenshots/api-explorer/api-explorer-mobile.png) |

---

## Region: Endpoint List Panel (Left)

- **Position**: Left side
- **Dimensions**: 320px width (w-80)
- **Background**: `var(--card)`
- **Border**: 1px solid border on right
- **Overflow**: auto (scrollable)

### Container: Panel Header

- **Padding**: 16px
- **Content**: H2 "API Endpoints" (18px, semibold)
- **Margin Bottom**: 16px

### Container: Category Groups

- **Type**: Collapsible sections
- **Spacing**: 8px gap between categories

#### Component: Category Header

- **Type**: Collapsible trigger button
- **Layout**: Flex row with chevron, name, and count badge
- **States**:
  - Expanded: ChevronDown icon
  - Collapsed: ChevronRight icon
- **Typography**: 14px, medium
- **Hover**: bg-muted
- **Border Radius**: 6px

#### Component: Endpoint Count Badge

- **Type**: Badge, secondary variant
- **Position**: Right side of category header
- **Content**: Number of endpoints in category

### Container: Endpoint Items

- **Type**: List of clickable buttons
- **Indent**: 16px left margin (ml-4)
- **Spacing**: 4px gap

#### Component: Endpoint Button

- **Type**: Button-like clickable element
- **Layout**: Flex row with method badge + path
- **Typography**: Path in monospace, 12px
- **States**:
  - Default: Transparent background
  - Hover: bg-muted
  - Selected: bg-muted (active state)
- **Border Radius**: 6px
- **Padding**: 8px

#### Component: HTTP Method Badge

- **Type**: Badge with method-specific colors
- **Typography**: 10px, semibold, uppercase
- **Padding**: 6px horizontal
- **Colors**:
  - GET: Green (bg-green-500/10, text-green-600)
  - POST: Blue (bg-blue-500/10, text-blue-600)
  - PUT: Yellow (bg-yellow-500/10, text-yellow-600)
  - PATCH: Orange (bg-orange-500/10, text-orange-600)
  - DELETE: Red (bg-red-500/10, text-red-600)

---

## Region: Request/Response Area (Right)

- **Position**: Right side, flexible width
- **Layout**: Flex column

### Container: Empty State

- **Visibility**: When no endpoint selected
- **Layout**: Centered content
- **Content**:
  - H2: "Select an Endpoint" (18px, medium)
  - Description: "Choose an API endpoint from the list to test it" (14px, muted-foreground)

---

### Container: Endpoint Header (When Selected)

- **Position**: Top of panel
- **Layout**: Flex row with gap
- **Border**: 1px solid border on bottom
- **Padding**: 12px horizontal, 12px vertical

#### Component: Method Badge

- **Type**: HTTP Method badge (same styling as list)
- **Position**: Left

#### Component: Path Display

- **Type**: Code element
- **Typography**: Monospace, 14px

#### Component: Description

- **Type**: Text span
- **Typography**: 14px, muted-foreground
- **Flex**: 1 (fills remaining space)

#### Component: Execute Button

- **Type**: Button, default variant
- **Icon**: Play icon (left)
- **Text**: "Execute"
- **States**:
  - Default: Primary styling
  - Loading: Loader2 icon (animated spin) + "Execute"
  - Disabled: During loading

---

### Container: Request Body (When Applicable)

- **Visibility**: Only for POST, PUT, PATCH endpoints
- **Border**: 1px solid border on bottom
- **Padding**: 16px

#### Component: Section Title

- **Type**: H3 heading
- **Typography**: 14px, medium
- **Margin Bottom**: 8px

#### Component: Request Textarea

- **Type**: Textarea
- **Height**: 150px minimum (min-h-[150px])
- **Typography**: Monospace, 14px
- **Placeholder**: "Enter JSON request body..."
- **Content**: Pre-populated with example JSON for supported endpoints
- **Source**: [src/components/ui/textarea.tsx](../../edgequake_webui/src/components/ui/textarea.tsx)

---

### Container: Response Section

- **Position**: Bottom, flexible height
- **Layout**: Flex column

#### Container: Response Header

- **Layout**: Flex row, space-between
- **Border**: 1px solid border on bottom
- **Padding**: 8px horizontal, 8px vertical

##### Component: Section Title

- **Type**: H3 heading
- **Typography**: 14px, medium

##### Component: Copy Button

- **Type**: Button, ghost variant, sm size
- **Visibility**: Only when response exists
- **Icon**: Copy icon (default), Check icon (after copy)
- **Text**: "Copy"
- **Function**: Copies response to clipboard

#### Container: Response Content

- **Type**: ScrollArea
- **Padding**: 16px

##### Component: Response Empty State

- **Type**: Paragraph
- **Visibility**: Before execution
- **Content**: "Click Execute to see the response"
- **Typography**: 14px, muted-foreground

##### Component: Response Output

- **Type**: Pre element
- **Visibility**: After execution
- **Typography**: Monospace, 14px, whitespace pre-wrap
- **Content**: JSON.stringify(result, null, 2)

---

## API Categories and Endpoints

| Category | Endpoints | Description |
|----------|-----------|-------------|
| Health | 1 | API health check |
| Auth | 2 | Authentication (login, current user) |
| Documents | 4 | Document CRUD operations |
| Query | 1 | Knowledge graph queries |
| Graph | 3 | Graph data and statistics |
| Entities | 5 | Entity CRUD and merge |
| Relationships | 2 | Relationship operations |
| Pipeline | 1 | Pipeline status |

---

## Responsive Behavior

| Breakpoint | Endpoint List | Request/Response |
|------------|---------------|------------------|
| Mobile (<768px) | Full width, endpoint list only visible | Hidden until endpoint selected |
| Tablet (768-1024px) | 280px width | Remaining width |
| Desktop (>1024px) | 320px width | Remaining width |

---

## Component Cross-References

- [Button](../components/buttons.md) — Execute button, endpoint items
- [Badge](../components/buttons.md) — HTTP method badges, count badges
- [Textarea](../components/inputs.md) — Request body input
- [Collapsible](../components/dialogs.md) — Category expandable sections
- [ScrollArea](../components/navigation.md) — Response scrolling

