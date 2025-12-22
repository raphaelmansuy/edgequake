# EdgeQuake WebUI - Component Mapping

> LightRAG → EdgeQuake component migration guide with file-level mapping.

**Parent Document**: [00-master-plan.md](./00-master-plan.md)

---

## Table of Contents

1. [Overview](#overview)
2. [Core Structure Mapping](#core-structure-mapping)
3. [Feature Components](#feature-components)
4. [UI Components](#ui-components)
5. [Store Migration](#store-migration)
6. [Key Changes](#key-changes)

---

## Overview

This document maps every component from `lightrag_webui/src/` to its equivalent in `edgequake_webui/src/`. Components are either:

- **Direct Port**: Minimal changes, same structure
- **Refactored**: Significant changes for Next.js patterns
- **New**: Created specifically for EdgeQuake
- **Removed**: Not needed in new architecture

---

## Core Structure Mapping

### Entry Points

| LightRAG        | EdgeQuake                    | Status      | Notes                            |
| --------------- | ---------------------------- | ----------- | -------------------------------- |
| `main.tsx`      | `app/layout.tsx`             | Refactored  | Root layout replaces React entry |
| `App.tsx`       | `app/(dashboard)/layout.tsx` | Refactored  | Dashboard layout                 |
| `AppRouter.tsx` | Next.js App Router           | Replaced    | File-based routing               |
| `index.css`     | `app/globals.css`            | Direct Port | Global styles                    |

### Root Files

| LightRAG Path                                        | EdgeQuake Path               | Notes                      |
| ---------------------------------------------------- | ---------------------------- | -------------------------- |
| [main.tsx](../lightrag_webui/src/main.tsx)           | `app/layout.tsx`             | Root layout with providers |
| [App.tsx](../lightrag_webui/src/App.tsx)             | `app/(dashboard)/layout.tsx` | Main app structure         |
| [AppRouter.tsx](../lightrag_webui/src/AppRouter.tsx) | N/A                          | Replaced by file routing   |
| [i18n.ts](../lightrag_webui/src/i18n.ts)             | `lib/i18n.ts`                | i18next → next-intl        |

---

## Feature Components

### Features Directory

| LightRAG Path                                                                              | EdgeQuake Path                                                         | Status      |
| ------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------- | ----------- |
| [features/GraphViewer.tsx](../lightrag_webui/src/features/GraphViewer.tsx)                 | `app/(dashboard)/graph/page.tsx` + `components/graph/graph-viewer.tsx` | Refactored  |
| [features/DocumentManager.tsx](../lightrag_webui/src/features/DocumentManager.tsx)         | `app/(dashboard)/documents/page.tsx` + `components/documents/`         | Refactored  |
| [features/RetrievalTesting.tsx](../lightrag_webui/src/features/RetrievalTesting.tsx)       | `app/(dashboard)/query/page.tsx` + `components/query/`                 | Refactored  |
| [features/LoginPage.tsx](../lightrag_webui/src/features/LoginPage.tsx)                     | `app/(auth)/login/page.tsx`                                            | Refactored  |
| [features/SiteHeader.tsx](../lightrag_webui/src/features/SiteHeader.tsx)                   | `components/layout/header.tsx`                                         | Direct Port |
| [features/TenantSelectionPage.tsx](../lightrag_webui/src/features/TenantSelectionPage.tsx) | `app/(auth)/select-tenant/page.tsx`                                    | Refactored  |
| [features/ApiSite.tsx](../lightrag_webui/src/features/ApiSite.tsx)                         | `app/(dashboard)/api-explorer/page.tsx`                                | Direct Port |

---

## UI Components

### Components from `components/ui/`

All shadcn/ui components will be regenerated using the CLI:

```bash
bunx shadcn@latest add button card dialog input select table tabs textarea tooltip
```

| LightRAG Component                                                     | EdgeQuake Component              | Source    |
| ---------------------------------------------------------------------- | -------------------------------- | --------- |
| [Alert.tsx](../lightrag_webui/src/components/ui/Alert.tsx)             | `components/ui/alert.tsx`        | shadcn/ui |
| [AlertDialog.tsx](../lightrag_webui/src/components/ui/AlertDialog.tsx) | `components/ui/alert-dialog.tsx` | shadcn/ui |
| [Button.tsx](../lightrag_webui/src/components/ui/Button.tsx)           | `components/ui/button.tsx`       | shadcn/ui |
| [Card.tsx](../lightrag_webui/src/components/ui/Card.tsx)               | `components/ui/card.tsx`         | shadcn/ui |
| [Checkbox.tsx](../lightrag_webui/src/components/ui/Checkbox.tsx)       | `components/ui/checkbox.tsx`     | shadcn/ui |
| [Dialog.tsx](../lightrag_webui/src/components/ui/Dialog.tsx)           | `components/ui/dialog.tsx`       | shadcn/ui |
| [Input.tsx](../lightrag_webui/src/components/ui/Input.tsx)             | `components/ui/input.tsx`        | shadcn/ui |
| [Select.tsx](../lightrag_webui/src/components/ui/Select.tsx)           | `components/ui/select.tsx`       | shadcn/ui |
| [Table.tsx](../lightrag_webui/src/components/ui/Table.tsx)             | `components/ui/table.tsx`        | shadcn/ui |
| [Tabs.tsx](../lightrag_webui/src/components/ui/Tabs.tsx)               | `components/ui/tabs.tsx`         | shadcn/ui |
| [Textarea.tsx](../lightrag_webui/src/components/ui/Textarea.tsx)       | `components/ui/textarea.tsx`     | shadcn/ui |
| [Tooltip.tsx](../lightrag_webui/src/components/ui/Tooltip.tsx)         | `components/ui/tooltip.tsx`      | shadcn/ui |

### Graph Components

| LightRAG Path                                                                                          | EdgeQuake Path                            | Status      |
| ------------------------------------------------------------------------------------------------------ | ----------------------------------------- | ----------- |
| [components/graph/FocusOnNode.tsx](../lightrag_webui/src/components/graph/FocusOnNode.tsx)             | `components/graph/focus-on-node.tsx`      | Direct Port |
| [components/graph/FullScreenControl.tsx](../lightrag_webui/src/components/graph/FullScreenControl.tsx) | `components/graph/fullscreen-control.tsx` | Direct Port |
| [components/graph/GraphControl.tsx](../lightrag_webui/src/components/graph/GraphControl.tsx)           | `components/graph/graph-control.tsx`      | Direct Port |
| [components/graph/GraphLabels.tsx](../lightrag_webui/src/components/graph/GraphLabels.tsx)             | `components/graph/graph-labels.tsx`       | Direct Port |
| [components/graph/GraphSearch.tsx](../lightrag_webui/src/components/graph/GraphSearch.tsx)             | `components/graph/graph-search.tsx`       | Direct Port |
| [components/graph/LayoutsControl.tsx](../lightrag_webui/src/components/graph/LayoutsControl.tsx)       | `components/graph/layouts-control.tsx`    | Direct Port |
| [components/graph/Legend.tsx](../lightrag_webui/src/components/graph/Legend.tsx)                       | `components/graph/legend.tsx`             | Direct Port |
| [components/graph/LegendButton.tsx](../lightrag_webui/src/components/graph/LegendButton.tsx)           | `components/graph/legend-button.tsx`      | Direct Port |
| [components/graph/MergeDialog.tsx](../lightrag_webui/src/components/graph/MergeDialog.tsx)             | `components/graph/merge-dialog.tsx`       | Direct Port |
| [components/graph/PropertiesView.tsx](../lightrag_webui/src/components/graph/PropertiesView.tsx)       | `components/graph/properties-view.tsx`    | Direct Port |
| [components/graph/Settings.tsx](../lightrag_webui/src/components/graph/Settings.tsx)                   | `components/graph/settings.tsx`           | Direct Port |
| [components/graph/ZoomControl.tsx](../lightrag_webui/src/components/graph/ZoomControl.tsx)             | `components/graph/zoom-control.tsx`       | Direct Port |

### Document Components

| LightRAG Path                                                                                                          | EdgeQuake Path                             | Status      |
| ---------------------------------------------------------------------------------------------------------------------- | ------------------------------------------ | ----------- |
| [components/documents/UploadDocumentsDialog.tsx](../lightrag_webui/src/components/documents/UploadDocumentsDialog.tsx) | `components/documents/upload-dialog.tsx`   | Direct Port |
| [components/documents/ClearDocumentsDialog.tsx](../lightrag_webui/src/components/documents/ClearDocumentsDialog.tsx)   | `components/documents/clear-dialog.tsx`    | Direct Port |
| [components/documents/DeleteDocumentsDialog.tsx](../lightrag_webui/src/components/documents/DeleteDocumentsDialog.tsx) | `components/documents/delete-dialog.tsx`   | Direct Port |
| [components/documents/PipelineStatusDialog.tsx](../lightrag_webui/src/components/documents/PipelineStatusDialog.tsx)   | `components/documents/pipeline-status.tsx` | Direct Port |

### Retrieval Components

| LightRAG Path                                                                                          | EdgeQuake Path                        | Status      |
| ------------------------------------------------------------------------------------------------------ | ------------------------------------- | ----------- |
| [components/retrieval/ChatMessage.tsx](../lightrag_webui/src/components/retrieval/ChatMessage.tsx)     | `components/query/chat-message.tsx`   | Direct Port |
| [components/retrieval/QuerySettings.tsx](../lightrag_webui/src/components/retrieval/QuerySettings.tsx) | `components/query/query-settings.tsx` | Direct Port |

### Shared Components

| LightRAG Path                                                                                        | EdgeQuake Path                           | Status                   |
| ---------------------------------------------------------------------------------------------------- | ---------------------------------------- | ------------------------ |
| [components/ThemeToggle.tsx](../lightrag_webui/src/components/ThemeToggle.tsx)                       | `components/shared/theme-toggle.tsx`     | Refactored (next-themes) |
| [components/ThemeProvider.tsx](../lightrag_webui/src/components/ThemeProvider.tsx)                   | `providers/theme-provider.tsx`           | Refactored (next-themes) |
| [components/LanguageToggle.tsx](../lightrag_webui/src/components/LanguageToggle.tsx)                 | `components/shared/language-toggle.tsx`  | Direct Port              |
| [components/TenantSelector.tsx](../lightrag_webui/src/components/TenantSelector.tsx)                 | `components/shared/tenant-selector.tsx`  | Direct Port              |
| [components/ApiKeyAlert.tsx](../lightrag_webui/src/components/ApiKeyAlert.tsx)                       | `components/shared/api-key-alert.tsx`    | Direct Port              |
| [components/AppSettings.tsx](../lightrag_webui/src/components/AppSettings.tsx)                       | `components/shared/app-settings.tsx`     | Direct Port              |
| [components/status/StatusIndicator.tsx](../lightrag_webui/src/components/status/StatusIndicator.tsx) | `components/shared/status-indicator.tsx` | Direct Port              |

---

## Store Migration

### Zustand Stores

| LightRAG Path                                                  | EdgeQuake Path                                             | Changes                    |
| -------------------------------------------------------------- | ---------------------------------------------------------- | -------------------------- |
| [stores/state.ts](../lightrag_webui/src/stores/state.ts)       | `stores/use-backend-store.ts` + `stores/use-auth-store.ts` | Split into separate stores |
| [stores/settings.ts](../lightrag_webui/src/stores/settings.ts) | `stores/use-settings-store.ts`                             | Renamed, same pattern      |
| [stores/tenant.ts](../lightrag_webui/src/stores/tenant.ts)     | `stores/use-tenant-store.ts`                               | Renamed, same pattern      |
| [stores/graph.ts](../lightrag_webui/src/stores/graph.ts)       | `stores/use-graph-store.ts`                                | Renamed, same pattern      |

### Store Pattern Changes

**Before (LightRAG):**

```typescript
// Uses custom createSelectors pattern
export const useSettingsStore = createSelectors(useSettingsStoreBase);
// Usage: useSettingsStore.use.theme()
```

**After (EdgeQuake):**

```typescript
// Standard Zustand pattern with TypeScript
export const useSettingsStore = create<SettingsState>()(
  persist((set) => ({ ... }), { name: 'edgequake-settings' })
)
// Usage: useSettingsStore((s) => s.theme)
```

---

## API Layer Migration

| LightRAG Path                                            | EdgeQuake Path         | Changes          |
| -------------------------------------------------------- | ---------------------- | ---------------- |
| [api/client.ts](../lightrag_webui/src/api/client.ts)     | `lib/api/client.ts`    | Axios → fetch    |
| [api/lightrag.ts](../lightrag_webui/src/api/lightrag.ts) | `lib/api/edgequake.ts` | Endpoint mapping |
| [api/tenant.ts](../lightrag_webui/src/api/tenant.ts)     | `lib/api/tenant.ts`    | Direct Port      |

---

## Hooks Migration

| LightRAG Path                                                                  | EdgeQuake Path             | Changes                   |
| ------------------------------------------------------------------------------ | -------------------------- | ------------------------- |
| [hooks/useDebounce.tsx](../lightrag_webui/src/hooks/useDebounce.tsx)           | `hooks/use-debounce.ts`    | Direct Port               |
| [hooks/useLightragGraph.tsx](../lightrag_webui/src/hooks/useLightragGraph.tsx) | `hooks/use-graph.ts`       | Renamed, refactored       |
| [hooks/useRandomGraph.tsx](../lightrag_webui/src/hooks/useRandomGraph.tsx)     | Removed                    | Dev only                  |
| [hooks/useRouteState.ts](../lightrag_webui/src/hooks/useRouteState.ts)         | `hooks/use-route-state.ts` | Refactored for App Router |
| [hooks/useTenantContext.ts](../lightrag_webui/src/hooks/useTenantContext.ts)   | `hooks/use-tenant.ts`      | Direct Port               |
| [hooks/useTheme.tsx](../lightrag_webui/src/hooks/useTheme.tsx)                 | N/A                        | next-themes built-in      |

---

## Key Changes

### 1. Client Component Directive

All interactive components need `'use client'` at the top:

```tsx
"use client";

import { useState } from "react";
// ...
```

### 2. File Naming Convention

- LightRAG: `PascalCase.tsx` (e.g., `GraphViewer.tsx`)
- EdgeQuake: `kebab-case.tsx` (e.g., `graph-viewer.tsx`)

### 3. Import Alias

- Both use `@/` alias but configured differently:
  - LightRAG: `vite.config.ts` resolve alias
  - EdgeQuake: `tsconfig.json` paths

### 4. Theme Provider

**LightRAG:**

```tsx
import ThemeProvider from "@/components/ThemeProvider";
```

**EdgeQuake:**

```tsx
import { ThemeProvider } from "next-themes";
```

### 5. Router Navigation

**LightRAG:**

```tsx
import { useNavigate } from "react-router-dom";
const navigate = useNavigate();
navigate("/login");
```

**EdgeQuake:**

```tsx
import { useRouter } from "next/navigation";
const router = useRouter();
router.push("/login");
```

### 6. API Client

**LightRAG:**

```tsx
import { axiosInstance } from "./client";
const response = await axiosInstance.get("/health");
```

**EdgeQuake:**

```tsx
import { apiClient } from "@/lib/api/client";
const response = await apiClient<HealthResponse>("/health");
```

---

## Related Documents

- **Previous**: [02-api-integration.md](./02-api-integration.md) - API integration
- **Next**: [04-ui-ux-improvements.md](./04-ui-ux-improvements.md) - UX enhancements
