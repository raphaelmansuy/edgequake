# Component Parity Matrix

## Legend

- ✅ Full parity
- ⚠️ Partial implementation
- ❌ Not implemented
- 🔄 Different approach (functionally equivalent)
- ⬆️ Target exceeds source
- ➖ Not applicable

---

## Matrix by Category

### LAYOUT: Layout & Shell Components

| ID    | Component               | Source | Target | Status | Gap ID | Priority | Notes              |
| ----- | ----------------------- | ------ | ------ | ------ | ------ | -------- | ------------------ |
| C-001 | App Shell / Root Layout | ✅     | ✅     | ✅     | -      | -        | Next.js layout.tsx |
| C-002 | Site Header             | ✅     | ✅     | ✅     | -      | -        | -                  |
| C-003 | Sidebar Navigation      | ✅     | ✅     | ✅     | -      | -        | -                  |
| C-004 | Footer                  | ⚠️     | ⚠️     | ✅     | -      | -        | Minimal in both    |
| C-005 | Breadcrumbs             | ❌     | ✅     | ⬆️     | -      | -        | New in target      |

---

### NAV: Navigation Components

| ID    | Component       | Source | Target | Status | Gap ID | Priority | Notes            |
| ----- | --------------- | ------ | ------ | ------ | ------ | -------- | ---------------- |
| C-006 | Main Navigation | ✅     | ✅     | ✅     | -      | -        | App Router based |
| C-007 | Tab Navigation  | ✅     | ✅     | ✅     | -      | -        | Within pages     |
| C-008 | Mobile Menu     | ⚠️     | ✅     | ⬆️     | -      | -        | Better mobile UX |

---

### GRAPH: Knowledge Graph Components

| ID    | Component              | Source | Target | Status | Gap ID | Priority | Notes                      |
| ----- | ---------------------- | ------ | ------ | ------ | ------ | -------- | -------------------------- |
| C-010 | GraphViewer            | ✅     | ✅     | ✅     | -      | -        | react-force-graph          |
| C-011 | GraphControls          | ✅     | ✅     | ✅     | -      | -        | -                          |
| C-012 | ZoomControl            | ✅     | ✅     | ✅     | -      | -        | -                          |
| C-013 | LayoutsControl         | ✅     | ✅     | ✅     | -      | -        | Force/Circular/Random      |
| C-014 | Legend                 | ✅     | ✅     | ✅     | -      | -        | -                          |
| C-015 | LegendButton           | ✅     | ✅     | ✅     | -      | -        | -                          |
| C-016 | GraphLabels            | ✅     | ✅     | ✅     | -      | -        | Entity type filter         |
| C-017 | GraphSearch            | ✅     | ✅     | ✅     | -      | -        | Label search               |
| C-018 | FocusOnNode            | ✅     | ✅     | ✅     | -      | -        | -                          |
| C-019 | PropertiesView         | ✅     | ✅     | ✅     | -      | -        | Node details panel         |
| C-020 | PropertyEditDialog     | ✅     | ✅     | ✅     | -      | -        | Integrated in node-details |
| C-021 | PropertyRowComponents  | ✅     | ✅     | ✅     | -      | -        | -                          |
| C-022 | EditablePropertyRow    | ✅     | ✅     | ✅     | -      | -        | -                          |
| C-023 | MergeDialog            | ✅     | ✅     | ✅     | -      | -        | In entity-edit-dialog      |
| C-024 | Settings (Graph)       | ✅     | ✅     | ✅     | -      | -        | -                          |
| C-025 | SettingsDisplay        | ✅     | ✅     | ✅     | -      | -        | -                          |
| C-026 | FullScreenControl      | ✅     | ✅     | ✅     | -      | -        | -                          |
| C-027 | GraphContextMenu       | ⚠️     | ✅     | ⬆️     | -      | -        | Enhanced                   |
| C-028 | NodeContextMenu        | ⚠️     | ✅     | ⬆️     | -      | -        | Enhanced                   |
| C-029 | GraphExport            | ✅     | ✅     | ✅     | -      | -        | PNG/SVG                    |
| C-030 | GraphFilters           | ✅     | ✅     | ✅     | -      | -        | -                          |
| C-031 | GraphRenderer          | ✅     | ✅     | ✅     | -      | -        | Core rendering             |
| C-032 | GraphEvents            | ✅     | ✅     | ✅     | -      | -        | Click/hover handlers       |
| C-033 | EntityEditDialog       | ✅     | ✅     | ✅     | -      | -        | Edit/rename/merge flow     |
| C-034 | RelationshipEditDialog | ✅     | ✅     | ✅     | -      | -        | Type/weight/description    |

---

### DOCS: Document Management Components

| ID    | Component             | Source | Target | Status | Gap ID | Priority | Notes         |
| ----- | --------------------- | ------ | ------ | ------ | ------ | -------- | ------------- |
| C-040 | DocumentManager       | ✅     | ✅     | ✅     | -      | -        | Main page     |
| C-041 | DocumentList          | ✅     | ✅     | ✅     | -      | -        | Table view    |
| C-042 | DocumentFilters       | ✅     | ✅     | ✅     | -      | -        | Status filter |
| C-043 | PaginationControls    | ✅     | ✅     | ✅     | -      | -        | -             |
| C-044 | UploadDocumentsDialog | ✅     | ✅     | ✅     | -      | -        | -             |
| C-045 | DeleteDocumentsDialog | ✅     | ✅     | ✅     | -      | -        | -             |
| C-046 | ClearDocumentsDialog  | ✅     | ✅     | ✅     | -      | -        | Implemented   |
| C-047 | PipelineStatusDialog  | ✅     | ✅     | ✅     | -      | -        | With history  |
| C-048 | BatchProgressCard     | ✅     | ✅     | ✅     | -      | -        | -             |
| C-049 | DocumentDetailDialog  | ✅     | ✅     | ✅     | -      | -        | -             |
| C-050 | ScanDocumentsButton   | ✅     | ✅     | ✅     | -      | -        | Implemented   |
| C-051 | ReprocessFailedButton | ✅     | ✅     | ✅     | -      | -        | Implemented   |
| C-052 | ResetStatusButton     | ✅     | ✅     | ✅     | -      | -        | Implemented   |
| C-053 | ScanProgressIndicator | ✅     | ✅     | ✅     | -      | -        | Via polling   |

---

### QUERY: Query Interface Components

| ID    | Component          | Source | Target | Status | Gap ID | Priority | Notes            |
| ----- | ------------------ | ------ | ------ | ------ | ------ | -------- | ---------------- |
| C-060 | QueryInterface     | ✅     | ✅     | ✅     | -      | -        | Main chat UI     |
| C-061 | QueryModeSelector  | ✅     | ✅     | ✅     | -      | -        | -                |
| C-062 | QuerySettings      | ✅     | ✅     | ✅     | -      | -        | Advanced options |
| C-063 | ChatMessage        | ✅     | ✅     | ✅     | -      | -        | -                |
| C-064 | MarkdownRenderer   | ✅     | ✅     | ✅     | -      | -        | -                |
| C-065 | ThinkingDisplay    | ✅     | ✅     | ✅     | -      | -        | Thinking time    |
| C-066 | SourceCitations    | ✅     | ✅     | ✅     | -      | -        | References       |
| C-067 | QueryHistory       | ✅     | ✅     | ✅     | -      | -        | Conversation     |
| C-068 | ClearHistoryButton | ✅     | ✅     | ✅     | -      | -        | -                |

---

### AUTH: Authentication Components

| ID    | Component    | Source | Target | Status | Gap ID | Priority | Notes            |
| ----- | ------------ | ------ | ------ | ------ | ------ | -------- | ---------------- |
| C-070 | LoginPage    | ✅     | ✅     | ✅     | -      | -        | -                |
| C-071 | LoginForm    | ✅     | ✅     | ✅     | -      | -        | -                |
| C-072 | AuthGuard    | ✅     | ✅     | ✅     | -      | -        | Route protection |
| C-073 | ApiKeyAlert  | ✅     | ✅     | ✅     | -      | -        | API key prompt   |
| C-074 | LogoutButton | ✅     | ✅     | ✅     | -      | -        | -                |

---

### TENANT: Multi-Tenancy Components

| ID    | Component           | Source | Target | Status | Gap ID | Priority | Notes                 |
| ----- | ------------------- | ------ | ------ | ------ | ------ | -------- | --------------------- |
| C-080 | TenantSelector      | ✅     | ✅     | ✅     | -      | -        | Header dropdown       |
| C-081 | TenantSelectionPage | ✅     | ✅     | ✅     | -      | -        | Full page             |
| C-082 | KnowledgeBaseList   | ✅     | ✅     | ✅     | -      | -        | Now "Workspaces"      |
| C-083 | CreateTenantDialog  | ✅     | ✅     | ✅     | -      | -        | -                     |
| C-084 | CreateKBDialog      | ✅     | ✅     | ✅     | -      | -        | Now "CreateWorkspace" |
| C-085 | TenantSearchInput   | ✅     | ✅     | ✅     | -      | -        | -                     |

---

### SETTINGS: Settings Components

| ID    | Component          | Source | Target | Status | Gap ID | Priority | Notes              |
| ----- | ------------------ | ------ | ------ | ------ | ------ | -------- | ------------------ |
| C-090 | AppSettings        | ✅     | ✅     | ✅     | -      | -        | Main settings page |
| C-091 | ThemeToggle        | ✅     | ✅     | ✅     | -      | -        | -                  |
| C-092 | LanguageToggle     | ✅     | ✅     | ✅     | -      | -        | -                  |
| C-093 | ApiKeyInput        | ✅     | ✅     | ✅     | -      | -        | -                  |
| C-094 | BackendUrlInput    | ✅     | ✅     | ✅     | -      | -        | -                  |
| C-095 | GraphSettingsPanel | ✅     | ✅     | ✅     | -      | -        | -                  |
| C-096 | QuerySettingsPanel | ✅     | ✅     | ✅     | -      | -        | -                  |
| C-097 | ClearCacheButton   | ✅     | ✅     | ✅     | -      | -        | Implemented        |

---

### UI: Base UI Components (shadcn/ui)

| ID    | Component  | Source | Target | Status | Gap ID | Priority | Notes     |
| ----- | ---------- | ------ | ------ | ------ | ------ | -------- | --------- |
| C-100 | Button     | ✅     | ✅     | ✅     | -      | -        | shadcn/ui |
| C-101 | Input      | ✅     | ✅     | ✅     | -      | -        | shadcn/ui |
| C-102 | Dialog     | ✅     | ✅     | ✅     | -      | -        | shadcn/ui |
| C-103 | Card       | ✅     | ✅     | ✅     | -      | -        | shadcn/ui |
| C-104 | Table      | ✅     | ✅     | ✅     | -      | -        | shadcn/ui |
| C-105 | Tabs       | ✅     | ✅     | ✅     | -      | -        | shadcn/ui |
| C-106 | Select     | ✅     | ✅     | ✅     | -      | -        | shadcn/ui |
| C-107 | Dropdown   | ✅     | ✅     | ✅     | -      | -        | shadcn/ui |
| C-108 | Toast      | ✅     | ✅     | ✅     | -      | -        | shadcn/ui |
| C-109 | Tooltip    | ✅     | ✅     | ✅     | -      | -        | shadcn/ui |
| C-110 | Badge      | ✅     | ✅     | ✅     | -      | -        | shadcn/ui |
| C-111 | Skeleton   | ✅     | ✅     | ✅     | -      | -        | shadcn/ui |
| C-112 | Progress   | ✅     | ✅     | ✅     | -      | -        | shadcn/ui |
| C-113 | Slider     | ✅     | ✅     | ✅     | -      | -        | shadcn/ui |
| C-114 | Switch     | ✅     | ✅     | ✅     | -      | -        | shadcn/ui |
| C-115 | Checkbox   | ✅     | ✅     | ✅     | -      | -        | shadcn/ui |
| C-116 | Alert      | ✅     | ✅     | ✅     | -      | -        | shadcn/ui |
| C-117 | Separator  | ✅     | ✅     | ✅     | -      | -        | shadcn/ui |
| C-118 | ScrollArea | ✅     | ✅     | ✅     | -      | -        | shadcn/ui |

---

### STATUS: Status & Feedback Components

| ID    | Component        | Source | Target | Status | Gap ID | Priority | Notes            |
| ----- | ---------------- | ------ | ------ | ------ | ------ | -------- | ---------------- |
| C-120 | ServerStatus     | ✅     | ✅     | ✅     | -      | -        | Health indicator |
| C-121 | ConnectionStatus | ✅     | ✅     | ✅     | -      | -        | -                |
| C-122 | LoadingSpinner   | ✅     | ✅     | ✅     | -      | -        | -                |
| C-123 | ErrorBoundary    | ✅     | ✅     | ✅     | -      | -        | error.tsx        |
| C-124 | LoadingState     | ✅     | ✅     | ✅     | -      | -        | loading.tsx      |
| C-125 | EmptyState       | ✅     | ✅     | ✅     | -      | -        | -                |

---

## Summary by Category

| Category  | Total   | ✅     | ⚠️    | ❌    | 🔄    | ⬆️    | Parity % |
| --------- | ------- | ------ | ----- | ----- | ----- | ----- | -------- |
| LAYOUT    | 5       | 4      | 0     | 0     | 0     | 1     | 100%     |
| NAV       | 3       | 2      | 0     | 0     | 0     | 1     | 100%     |
| GRAPH     | 25      | 24     | 0     | 0     | 0     | 1     | 100%     |
| DOCS      | 14      | 14     | 0     | 0     | 0     | 0     | 100%     |
| QUERY     | 9       | 9      | 0     | 0     | 0     | 0     | 100%     |
| AUTH      | 5       | 5      | 0     | 0     | 0     | 0     | 100%     |
| TENANT    | 6       | 6      | 0     | 0     | 0     | 0     | 100%     |
| SETTINGS  | 8       | 8      | 0     | 0     | 0     | 0     | 100%     |
| UI        | 19      | 19     | 0     | 0     | 0     | 0     | 100%     |
| STATUS    | 6       | 6      | 0     | 0     | 0     | 0     | 100%     |
| **Total** | **100** | **97** | **0** | **0** | **0** | **3** | **100%** |

---

## Critical Path to Full Parity

### P1 Gaps (Must Fix)

1. **C-050: Scan Documents Button**

   - Location: Document Manager toolbar
   - API: POST /api/v1/documents/scan
   - Effort: 0.5 days

2. **C-051: Reprocess Failed Button**

   - Location: Document Manager (when failed_count > 0)
   - API: POST /api/v1/documents/reprocess
   - Effort: 0.5 days

3. **C-020: Property Edit Dialog - Rename Flow**
   - Location: Graph node context menu
   - Issue: Missing allow_rename and merge handling
   - Effort: 1 day

### P2 Gaps (Should Fix)

4. **C-046: Clear Documents Dialog**

   - Location: Document Manager toolbar
   - API: DELETE /api/v1/documents
   - Effort: 0.5 days

5. **C-047: Pipeline Status - History Messages**

   - Location: Pipeline status dialog
   - Issue: Missing history_messages display
   - Effort: 0.5 days

6. **C-052: Reset Status Button**

   - Location: Document detail or bulk actions
   - API: Needs implementation
   - Effort: 1 day (including API)

7. **C-053: Scan Progress Indicator**
   - Location: Document Manager
   - API: Use task polling
   - Effort: 0.5 days

### P3 Gaps (Nice to Have)

8. **C-097: Clear Cache Button**
   - Location: Settings
   - API: Not implemented in Rust backend
   - Effort: 0.5 days (if API added)

---

## Quick Wins

| Component                 | Effort   | Impact | Notes                    |
| ------------------------- | -------- | ------ | ------------------------ |
| Scan Documents Button     | 0.5 days | High   | Simple button + API call |
| Reprocess Failed Button   | 0.5 days | High   | Simple button + API call |
| Clear Documents Dialog    | 0.5 days | Medium | Confirmation dialog      |
| Pipeline History Messages | 0.5 days | Medium | Display existing data    |

---

## Target Advantages (Features Exceeding Source)

| Feature           | Description                  | Benefit                   |
| ----------------- | ---------------------------- | ------------------------- |
| Breadcrumbs       | Navigation breadcrumbs       | Better navigation UX      |
| Mobile Menu       | Enhanced mobile navigation   | Better mobile experience  |
| Context Menus     | Enhanced graph context menus | More discoverable actions |
| Error Boundaries  | File-based error.tsx         | Better error handling     |
| Loading States    | File-based loading.tsx       | Automatic loading UI      |
| Server Components | React Server Components      | Faster initial load       |
| SEO Metadata      | Built-in Next.js metadata    | Better SEO                |
