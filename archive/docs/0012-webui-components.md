# EdgeQuake WebUI Components

> Catalog of reusable UI components, design patterns, and feature modules.

**Version**: 1.0.0 | **Last Updated**: 2026-01-09

---

## 1. Component Strategy

EdgeQuake WebUI divides components into four distinct tiers based on **Atomic Design** principles:

1.  **Primitives (`components/ui`)**: Low-level, stylable atoms (Inputs, Buttons).
2.  **Shared Patterns (`components/shared`)**: Reusable UI patterns agnostic to domain logic.
3.  **Layouts (`components/layout`)**: Structural frames defining application zones.
4.  **Feature Modules (`components/{feature}`)**: Domain-rich logic bound to specific use cases.

---

## 2. UI Primitives (`components/ui/`)

We utilize **shadcn/ui** as our component library foundation. These components are copy-pasteable, accessible, and built on **Radix UI** primitives.

### key Primitives

| Component | Underlying Primitive | Purpose |
|-----------|----------------------|---------|
| **Button** | `button` | Standard interactive triggers. Supports variants: `default`, `destructive`, `outline`, `secondary`, `ghost`. |
| **Dialog** | `@radix-ui/react-dialog` | Modal windows for confirmation or complex inputs. |
| **Sheet** | `@radix-ui/react-dialog` | Slide-over panels used for metadata inspection or mobile menus. |
| **Card** | `div` | Content container with consistent border, padding, and shadow. |
| **Table** | `table` | Responsive data grid base. |
| **Tooltip** | `@radix-ui/react-tooltip` | Contextual help text on hover. |

> **Dev Note**: Do not modify files in `components/ui/` heavily. If you need a variant, add it via `cva` (Class Variance Authority) config within the component file rather than changing core logic.

---

## 3. Shared Molecules (`components/shared/`)

These components glue primitives together for common UX patterns across the application.

### `EmptyState`
Displays a friendly illustration and action button when a list or view has no data.

```tsx
<EmptyState
  title="No Documents Found"
  description="Upload a document to get started."
  icon={<FileIcon />}
  action={<UploadButton />}
/>
```

### `ResponsiveTable`
A wrapper around the standard `Table` that handles:
- Horizontal scrolling on mobile.
- Loading skeletons.
- Sorting headers.

### `WebsocketStatus`
A pill component indicating real-time connection health.
- **Green**: Connected.
- **Yellow**: Reconnecting.
- **Red**: Disconnected (with retry).

---

## 4. Graph Architecture (`components/graph/`)

The graph module is the most complex UI region. It strictly separates **rendering** (Sigma.js) from **logic** (Zustand).

### Core Components

-   **`GraphViewer`**: The main canvas container. Initializes the Sigma instance and handles resize events.
-   **`GraphRenderer`**: Handles the actual WebGL rendering loop and visual settings.
-   **`GraphControls`**: The floating HUD for zoom, layout toggling, and search.
-   **`NodeDetails`**: A slide-over panel showing properties and neighbors of a selected node.

### Interaction Flow

1.  User clicks a node in `GraphViewer`.
2.  `GraphEvents` captures the click.
3.  Event handler updates `useGraphStore.selectedNode`.
4.  `NodeDetails` (listening to the store) slides in.

---

## 5. Document Management

The system separates the **manager view** (list of all docs) from the **document view** (single doc analysis).

### Manager (`components/documents/`)

-   **`DocumentManager`**: The "Smart Container" fetching the list of documents.
-   **`DocumentFilters`**: Search bar, status dropdowns, and date pickers controlling the list query.
-   **`IngestionProgressPanel`**: A real-time updating dashboard showing batch processing status via WebSockets.

### Single Document (`components/document/`)

-   **`ChunkExplorer`**: Visualizes the document split into RAG chunks.
-   **`MetadataSidebar`**: Shows extracted entities, creation info, and processing costs.
-   **`LineageTree`**: A recursive tree view showing how source text became chunks, then graph nodes.

---

## 6. Query Interface (`components/query/`)

Components handling the Chat-with-Graph experience.

-   **`ChatInterface`**: Main container managing the message history stream.
-   **`MessageBubble`**: Renders User or AI messages. Supports Markdown and syntax highlighting.
-   **`SourceCitations`**: Renders footnotes and link-backs to specific graph nodes or document chunks referenced in the answer.

---

## 7. Layout & Shell (`components/layout/`)

Defines the application frame.

-   **`AppSidebar`**: The primary navigation rail. Collapsible.
-   **`AppHeader`**: Top bar containing `Breadcrumbs`, `TenantSelector`, and `UserNav`.
-   **`MobileNav`**: Drawer navigation for small screens.

**Structure**:
```tsx
<div className="flex h-screen overflow-hidden">
  <AppSidebar />
  <div className="flex-1 flex flex-col">
    <AppHeader />
    <main className="flex-1 overflow-auto p-4">
      {children}
    </main>
  </div>
</div>
```

---

## 8. Best Practices

### 8.1 Accessibility (a11y)
-   All interactive elements **must** have focus states.
-   Dialogs must trap focus.
-   Use `Radix UI` primitives wherever possible to ensure screen reader compliance.

### 8.2 Performance
-   **Lazy load** heavy components (like `GraphViewer`) using `next/dynamic`.
-   Use `React.memo` on components receiving high-frequency updates (e.g., progress bars, timer ticks).

### 8.3 Styling
-   Use **Tailwind CSS** for all styling.
-   Use `cn()` utility to merge classes conditionally.
-   Adhere to the design system tokens defined in `tailwind.config.ts`.
