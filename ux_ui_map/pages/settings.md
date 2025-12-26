# Page: Settings

## Overview

- **Route**: `/settings`
- **Title**: "Settings"
- **Layout**: Single-column centered layout with max-width 1024px (max-w-4xl)
- **Source File**: [src/app/(dashboard)/settings/page.tsx](../../edgequake_webui/src/app/(dashboard)/settings/page.tsx)

## Layout Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│ ┌───────────────┬─────────────────────────────────────────────────┐ │
│ │               │ Header (64px)                                   │ │
│ │               │ [Mobile Menu]           [API Status] [Theme] [User] │
│ │               ├─────────────────────────────────────────────────┤ │
│ │   Sidebar     │ Breadcrumb: EdgeQuake > Settings                │ │
│ │   (64px)      ├─────────────────────────────────────────────────┤ │
│ │               │                                                 │ │
│ │   Nav Icons   │ Page Header                                     │ │
│ │               │ "Settings" + "Customize your EdgeQuake..."      │ │
│ │               │                                                 │ │
│ │               │ ┌─────────────────────────────────────────────┐ │ │
│ │               │ │ Appearance Card                             │ │ │
│ │               │ │ - Theme selector                            │ │ │
│ │               │ │ - Language selector                         │ │ │
│ │               │ └─────────────────────────────────────────────┘ │ │
│ │               │                                                 │ │
│ │               │ ┌─────────────────────────────────────────────┐ │ │
│ │               │ │ Graph Visualization Card                    │ │ │
│ │               │ │ - Show Node Labels toggle                   │ │ │
│ │               │ │ - Show Edge Labels toggle                   │ │ │
│ │               │ │ - Node Size selector                        │ │ │
│ │               │ │ - Default Layout selector                   │ │ │
│ │               │ └─────────────────────────────────────────────┘ │ │
│ │               │                                                 │ │
│ │               │ ┌─────────────────────────────────────────────┐ │ │
│ │               │ │ Query Defaults Card                         │ │ │
│ │               │ │ - Default Query Mode selector               │ │ │
│ │               │ │ - Enable Streaming toggle                   │ │ │
│ │               │ └─────────────────────────────────────────────┘ │ │
│ │               │                                                 │ │
│ │               │ ┌─────────────────────────────────────────────┐ │ │
│ │               │ │ Data Management Card (danger zone)          │ │ │
│ │               │ │ - Settings Backup (Export/Import)           │ │ │
│ │               │ │ - Clear History                             │ │ │
│ │               │ │ - Reset All Settings                        │ │ │
│ │               │ └─────────────────────────────────────────────┘ │ │
│ └───────────────┴─────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────┘
```

## Screenshots

| Viewport | Screenshot |
|----------|------------|
| Desktop (1440px) | [settings-desktop.png](../screenshots/settings/settings-desktop.png) |
| Tablet (768px) | [settings-tablet.png](../screenshots/settings/settings-tablet.png) |
| Mobile (375px) | [settings-mobile.png](../screenshots/settings/settings-mobile.png) |

---

## Region: Page Header

- **Position**: Top of main content
- **Spacing**: 8px bottom margin
- **Content**:
  - H1: "Settings" (30px, bold, tracking-tight)
  - Subtitle: "Customize your EdgeQuake experience" (16px, muted-foreground)

---

## Container: Appearance Card

- **Type**: Card component
- **Icon**: Palette icon (20px, text-primary)
- **Border**: 1px solid `var(--border)`
- **Border Radius**: 12px
- **Padding**: CardHeader 16px, CardContent 24px

### Section: Theme

- **Layout**: Flex row, space-between
- **Label**: "Theme" (14px, medium)
- **Description**: "Select your preferred color scheme" (14px, muted-foreground)

#### Component: Theme Selector

- **Type**: Select dropdown
- **Width**: 150px
- **Options**:
  - Light (Sun icon)
  - Dark (Moon icon)
  - System (Monitor icon)

### Section: Language

- **Layout**: Flex row, space-between
- **Label**: "Language" (14px, medium)
- **Description**: "Select your preferred language"

#### Component: Language Selector

- **Type**: Select dropdown
- **Width**: 150px
- **Options**: English, 中文, 日本語, 한국어

---

## Container: Graph Visualization Card

- **Type**: Card component
- **Icon**: Globe icon (20px)
- **Header**: "Graph Visualization"
- **Description**: "Configure how the knowledge graph is displayed"

### Section: Show Node Labels

- **Layout**: Flex row, space-between
- **Label**: "Show Node Labels" (14px, medium)
- **Description**: "Display labels on graph nodes" (12px, muted-foreground)

#### Component: Switch Toggle

- **Type**: Switch component
- **Default State**: Checked (true)
- **Source**: [src/components/ui/switch.tsx](../../edgequake_webui/src/components/ui/switch.tsx)

### Section: Show Edge Labels

- **Layout**: Flex row, space-between
- **Label**: "Show Edge Labels"
- **Description**: "Display relationship types on edges"

#### Component: Switch Toggle

- **Type**: Switch component
- **Default State**: Unchecked (false)

### Section: Node Size

- **Layout**: Flex row, space-between
- **Label**: "Node Size"
- **Description**: "Size of nodes in the graph"

#### Component: Node Size Selector

- **Type**: Select dropdown
- **Width**: 120px
- **Options**: Small, Medium (default), Large

### Section: Default Layout

- **Layout**: Flex row, space-between
- **Label**: "Default Layout"
- **Description**: "Initial graph layout algorithm"

#### Component: Layout Selector

- **Type**: Select dropdown
- **Width**: 150px
- **Options**: Force-Directed (default), Circular, Random

---

## Container: Query Defaults Card

- **Type**: Card component
- **Icon**: Database icon (20px)
- **Header**: "Query Defaults"
- **Description**: "Default settings for knowledge graph queries"

### Section: Default Query Mode

- **Layout**: Flex row, space-between
- **Label**: "Default Query Mode"
- **Description**: "Default retrieval mode for queries"

#### Component: Mode Selector

- **Type**: Select dropdown
- **Width**: 120px
- **Options**: Local, Global, Hybrid (default), Naive

### Section: Enable Streaming

- **Layout**: Flex row, space-between
- **Label**: "Enable Streaming"
- **Description**: "Show responses as they are generated"

#### Component: Switch Toggle

- **Type**: Switch component
- **Default State**: Checked (true)

---

## Container: Data Management Card

- **Type**: Card component with danger styling
- **Icon**: Database icon (20px, text-destructive)
- **Header**: "Data Management" (text-destructive)
- **Description**: "Manage local data, import/export settings, and reset..."
- **Border**: 1px solid `var(--destructive)/30`

### Section: Settings Backup

- **Layout**: Flex row, space-between
- **Label**: "Settings Backup" (14px, medium)
- **Description**: "Export or import your settings as JSON"

#### Component: Export Button

- **Type**: Button, outline variant, sm size
- **Icon**: Download icon (16px)
- **Text**: "Export"
- **Function**: Downloads settings as JSON file

#### Component: Import Button

- **Type**: Button (as label), outline variant, sm size
- **Icon**: Upload icon (16px)
- **Text**: "Import"
- **Function**: Opens file picker for .json import

### Section: Query History

- **Layout**: Flex row, space-between
- **Label**: "Query History"
- **Description**: "Clear all saved query history and conversations"

#### Component: Clear History Button

- **Type**: AlertDialog trigger button
- **Variant**: Outline with destructive styling
- **Text**: "Clear History"
- **Border**: 1px solid `var(--destructive)/50`
- **Text Color**: `var(--destructive)`

#### Component: Clear History Dialog

- **Type**: AlertDialog
- **Title**: "Clear query history?"
- **Description**: Explains permanent deletion
- **Actions**: Cancel (secondary), Clear (destructive)
- **Source**: [src/components/ui/alert-dialog.tsx](../../edgequake_webui/src/components/ui/alert-dialog.tsx)

### Section: Reset All Settings

- **Layout**: Flex row, space-between
- **Background**: `var(--destructive)/5` with destructive border
- **Border Radius**: 8px
- **Padding**: 16px
- **Label**: "Reset All Settings" (text-destructive)
- **Description**: "Reset all settings to their default values..."

#### Component: Reset Settings Button

- **Type**: AlertDialog trigger button
- **Variant**: Destructive
- **Size**: sm
- **Text**: "Reset Settings"

#### Component: Reset Settings Dialog

- **Type**: AlertDialog
- **Title**: "Reset all settings?"
- **Description**: Explains reset behavior
- **Actions**: Cancel, Reset (destructive)

---

## State Management

- **Settings Store**: [src/stores/use-settings-store.ts](../../edgequake_webui/src/stores/use-settings-store.ts)
  - Persists: language, graphSettings, querySettings
  - Methods: setLanguage, setGraphSettings, setQuerySettings, resetSettings, exportSettings, importSettings
- **Query Store**: [src/stores/use-query-store.ts](../../edgequake_webui/src/stores/use-query-store.ts)
  - Method: clearHistory

---

## Responsive Behavior

| Breakpoint | Layout | Card Width |
|------------|--------|------------|
| Mobile (<768px) | Full width, stacked | 100% with padding |
| Tablet (768-1024px) | Centered, max-w-4xl | ~90% of viewport |
| Desktop (>1024px) | Centered, max-w-4xl | 1024px max |

---

## Component Cross-References

- [Card](../components/cards.md) — Settings sections container
- [Button](../components/buttons.md) — Export, Import, Clear History, Reset
- [Select](../components/inputs.md) — Theme, Language, Node Size, Layout, Mode
- [Switch](../components/inputs.md) — Toggle settings
- [AlertDialog](../components/dialogs.md) — Confirmation dialogs
- [Separator](../components/navigation.md) — Section dividers

