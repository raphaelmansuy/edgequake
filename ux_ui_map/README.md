# UI Map: EdgeQuake WebUI

## Overview

- **Mapped On**: December 26, 2025
- **Framework**: Next.js 16.1.0 + React 19.2.3
- **UI Library**: Radix UI + Tailwind CSS v4
- **Total Pages**: 7
- **Total Components Cataloged**: 30+
- **Last Updated**: December 26, 2025

## Application Description

EdgeQuake WebUI is the client interface for the EdgeQuake Knowledge Graph RAG Platform. It provides a comprehensive dashboard for managing documents, exploring knowledge graphs, querying the RAG system, and configuring settings.

## Design System

- **Typography**: Inter font family
- **Color Scheme**: Light/Dark theme support using CSS custom properties
- **Spacing Scale**: 4px base unit with semantic tokens
- **Border Radius**: 0.625rem (10px) base radius
- **Icon Library**: Lucide React

## Page Index

| Page         | Route           | Regions                                   | Documentation                           | Screenshot                                                  |
| ------------ | --------------- | ----------------------------------------- | --------------------------------------- | ----------------------------------------------------------- |
| Dashboard    | `/`             | Header, Sidebar, Breadcrumb, Main         | [View](./pages/dashboard.md)            | [Desktop](./screenshots/dashboard/desktop.png)              |
| Documents    | `/documents`    | Header, Sidebar, Breadcrumb, Main, Panel  | [View](./pages/documents.md)            | [Desktop](./screenshots/documents/documents-desktop.png)    |
| Query        | `/query`        | Header, Sidebar, Main, History Panel      | [View](./pages/query.md)                | [Desktop](./screenshots/query/query-desktop.png)            |
| Graph        | `/graph`        | Header, Sidebar, Entity Browser, Canvas   | [View](./pages/graph.md)                | [Desktop](./screenshots/graph/graph-desktop.png)            |
| Settings     | `/settings`     | Header, Sidebar, Breadcrumb, Main         | [View](./pages/settings.md)             | [Desktop](./screenshots/settings/settings-desktop.png)      |
| API Explorer | `/api-explorer` | Header, Sidebar, Endpoint List, Request   | [View](./pages/api-explorer.md)         | [Desktop](./screenshots/api-explorer/api-explorer-desktop.png) |
| Login        | `/login`        | Full-screen centered card                 | [View](./pages/login.md)                | [Desktop](./screenshots/login/login-desktop.png)            |

## Component Library

| Component | Variants | Documentation |
|-----------|----------|---------------|
| Buttons   | 6 variants (default, destructive, outline, secondary, ghost, link) | [View](./components/buttons.md) |
| Inputs    | Text, Password, Textarea, Select, Switch, Checkbox | [View](./components/inputs.md) |
| Cards     | Standard, Stats, Danger, Message | [View](./components/cards.md) |
| Dialogs   | Dialog, AlertDialog, Sheet, Popover, Dropdown, Tooltip | [View](./components/dialogs.md) |
| Tables    | Data tables with selection, sorting, actions | [View](./components/tables.md) |
| Navigation | Sidebar, Header, Breadcrumb, Tabs, ScrollArea | [View](./components/navigation.md) |

## Layout Structure

The application uses a consistent dashboard layout for authenticated routes:

```
┌─────────────────────────────────────────────────────────────┐
│ ┌───────────┬─────────────────────────────────────────────┐ │
│ │           │ Header (64px, fixed)                        │ │
│ │           ├─────────────────────────────────────────────┤ │
│ │ Sidebar   │ Breadcrumb (48px)                           │ │
│ │ (256px    ├─────────────────────────────────────────────┤ │
│ │ expanded  │                                             │ │
│ │ 64px      │ Main Content (fluid)                        │ │
│ │ collapsed)│                                             │ │
│ │           │                                             │ │
│ └───────────┴─────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

## Theme Tokens

### Light Theme

| Token          | Value              | Usage             |
| -------------- | ------------------ | ----------------- |
| `--background` | `oklch(1 0 0)`     | Page background   |
| `--foreground` | `oklch(0.145 0 0)` | Primary text      |
| `--card`       | `oklch(1 0 0)`     | Card backgrounds  |
| `--primary`    | `oklch(0.205 0 0)` | Primary actions   |
| `--muted`      | `oklch(0.97 0 0)`  | Muted backgrounds |
| `--border`     | `oklch(0.922 0 0)` | Borders           |

### Dark Theme

| Token          | Value                | Usage             |
| -------------- | -------------------- | ----------------- |
| `--background` | `oklch(0.145 0 0)`   | Page background   |
| `--foreground` | `oklch(0.985 0 0)`   | Primary text      |
| `--card`       | `oklch(0.205 0 0)`   | Card backgrounds  |
| `--primary`    | `oklch(0.922 0 0)`   | Primary actions   |
| `--muted`      | `oklch(0.269 0 0)`   | Muted backgrounds |
| `--border`     | `oklch(1 0 0 / 10%)` | Borders           |

## Breakpoints

| Name    | Width   | Description                   |
| ------- | ------- | ----------------------------- |
| Mobile  | 375px   | Single column, sidebar hidden |
| Tablet  | 768px   | Sidebar as sheet overlay      |
| Desktop | 1024px+ | Full sidebar visible          |

## Accessibility

- WCAG 2.1 AA compliance targets
- Touch targets minimum 44px × 44px
- Keyboard navigation support (Tab, Arrow keys, Escape)
- ARIA labels on interactive elements
- Screen reader support via semantic HTML
- Focus visible states on all interactive elements

## Source Code References

| Component        | Source File                                                                               |
| ---------------- | ----------------------------------------------------------------------------------------- |
| App Layout       | [src/app/layout.tsx](../edgequake_webui/src/app/layout.tsx)                               |
| Dashboard Layout | [src/app/(dashboard)/layout.tsx](<../edgequake_webui/src/app/(dashboard)/layout.tsx>)     |
| Sidebar          | [src/components/layout/sidebar.tsx](../edgequake_webui/src/components/layout/sidebar.tsx) |
| Header           | [src/components/layout/header.tsx](../edgequake_webui/src/components/layout/header.tsx)   |
| Button           | [src/components/ui/button.tsx](../edgequake_webui/src/components/ui/button.tsx)           |
| Card             | [src/components/ui/card.tsx](../edgequake_webui/src/components/ui/card.tsx)               |
| Input            | [src/components/ui/input.tsx](../edgequake_webui/src/components/ui/input.tsx)             |
| Dialog           | [src/components/ui/dialog.tsx](../edgequake_webui/src/components/ui/dialog.tsx)           |
| Table            | [src/components/ui/table.tsx](../edgequake_webui/src/components/ui/table.tsx)             |
| Design Tokens    | [src/app/design-tokens.css](../edgequake_webui/src/app/design-tokens.css)                 |
| Global Styles    | [src/app/globals.css](../edgequake_webui/src/app/globals.css)                             |
