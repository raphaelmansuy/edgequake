# Graph Visualization, Settings & API Explorer - Combined Audit

## Overview
This document covers audits for three supporting screens:
- Graph Visualization (`/graph`)
- Settings (`/settings`)
- API Explorer (`/api-explorer`)

---

# Graph Visualization Screen (`/graph`)

**Screenshot Reference:** [`04-graph-full.png`](../audit_ui/screenshots/04-graph-full.png)

**Component Files:**
- Page: [`src/app/(dashboard)/graph/page.tsx`](../edgequake_webui/src/app/(dashboard)/graph/page.tsx)
- Components: [`src/components/graph/`](../edgequake_webui/src/components/graph/)

## What I Reviewed
- Graph canvas (Sigma.js renderer) - Takes most of screen (~1344px × 926px)
- Right panel - Node details and filters (detected in components)
- Controls (zoom, layout, export) - Should be visible but count was 0 in tests
- Search/filter functionality
- Legend for node types

## Issues

### 🔴 Critical

**C1. No Left Panel for Graph Navigation**
- **Issue:** Unlike other pages, no way to browse entity list or document list
- **Impact:** Must use search or randomly click nodes to explore
- **Recommendation:** Add left sidebar with:
  - Entity list (searchable)
  - Document list
  - Community clusters
  - Recent queries/paths

**C2. No Minimap for Large Graphs**
- **Issue:** When graph has 100+ nodes, hard to navigate
- **Recommendation:** Add minimap in corner (Sigma.js supports this via @react-sigma/minimap)

**C3. No Onboarding/Empty State**
- **Issue:** When no documents indexed, graph is empty with no explanation
- **Recommendation:** Show empty state with:
  - "No entities yet" message
  - Link to upload documents
  - Example of what graph will look like

### 🟡 Major

**M1. Controls Not Visible**
- **Issue:** Test shows 0 controls detected
- **Evidence:** Components exist but may not be rendered or positioned correctly
- **Expected:** Zoom controls (+, -, fit), layout selector, export button

**M2. Node Details Panel Hidden/Collapsed**
- **Issue:** Right panel should show when node selected, but may be collapsed
- **Recommendation:** 
  - Auto-expand when node clicked
  - Show: Entity name, type, properties, connected entities, source documents

**M3. No Filter UI Detected**
- **Issue:** Graph filters component exists but not visible in screenshot
- **Recommendation:** Add filter panel/drawer:
  - Filter by entity type
  - Filter by relationship type
  - Filter by document source
  - Date range filter

**M4. Legend Missing or Not Prominent**
- **Issue:** No legend detected in test
- **Recommendation:** Add color-coded legend for:
  - Entity types (Person, Organization, Location, etc.)
  - Relationship strengths (line thickness)
  - Community colors (if using Louvain clustering)

**M5. No Relationship Labels on Hover**
- **Issue:** Edge labels not showing or not visible
- **Recommendation:** Show relationship type on edge hover

### 🟢 Minor

**m1. No Keyboard Navigation**
- **Recommendation:** 
  - Arrow keys to pan
  - +/- to zoom
  - Tab to cycle through nodes
  - Enter to select focused node

**m2. No Path Finding**
- **Recommendation:** Add "Find Path" feature:
  - Select two nodes
  - Highlight shortest path
  - Show relationship chain

**m3. Export Options Limited**
- **Issue:** Export component exists but formats unknown
- **Recommendation:** Support PNG, SVG, JSON, CSV exports

**m4. No Time-based Visualization**
- **Recommendation:** If entities have timestamps, add timeline scrubber

**m5. Performance with Large Graphs**
- **Concern:** Sigma.js can struggle with 1000+ nodes
- **Recommendation:** Implement level-of-detail (LOD) rendering, node clustering

## Recommendations

### R1. Three-Panel Layout
```
┌──────────┬─────────────────────────┬───────────────┐
│  Entity  │    Graph Canvas         │  Node Details │
│  Browser │                         │               │
│          │   [Graph Rendering]     │   Name        │
│  Search  │                         │   Type        │
│  [____]  │                         │   Properties  │
│          │   Minimap               │               │
│  Filter: │   ┌────┐                │   Connected:  │
│  □ Person│   │    │                │   • Entity A  │
│  □ Org   │   └────┘                │   • Entity B  │
│  □ Loc   │                         │               │
│          │                         │   Sources:    │
│  Sort:   │   [Zoom] [Layout]       │   • Doc 1     │
│  • Name  │   [Export]              │   • Doc 2     │
│  • Degree│                         │               │
└──────────┴─────────────────────────┴───────────────┘
   280px           ~1280px                 360px
```

### R2. Comprehensive Controls
```tsx
<div className="absolute top-4 right-4 flex flex-col gap-2">
  {/* Zoom Controls */}
  <Card className="p-2">
    <Button size="sm" variant="ghost" onClick={handleZoomIn}>
      <Plus className="h-4 w-4" />
    </Button>
    <Button size="sm" variant="ghost" onClick={handleZoomOut}>
      <Minus className="h-4 w-4" />
    </Button>
    <Button size="sm" variant="ghost" onClick={handleFitView}>
      <Maximize className="h-4 w-4" />
    </Button>
  </Card>
  
  {/* Layout Selector */}
  <DropdownMenu>
    <DropdownMenuTrigger asChild>
      <Button size="sm" variant="outline">
        <Layout className="h-4 w-4 mr-2" />
        {layoutType}
      </Button>
    </DropdownMenuTrigger>
    <DropdownMenuContent>
      <DropdownMenuItem onClick={() => setLayout('force')}>
        Force Atlas
      </DropdownMenuItem>
      <DropdownMenuItem onClick={() => setLayout('circular')}>
        Circular
      </DropdownMenuItem>
      <DropdownMenuItem onClick={() => setLayout('random')}>
        Random
      </DropdownMenuItem>
    </DropdownMenuContent>
  </DropdownMenu>
  
  {/* Export */}
  <Button size="sm" variant="outline" onClick={handleExport}>
    <Download className="h-4 w-4 mr-2" />
    Export
  </Button>
</div>
```

### R3. Search with Autocomplete
```tsx
<div className="p-4 border-b">
  <div className="relative">
    <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4" />
    <Input
      placeholder="Search entities..."
      className="pl-9"
      value={searchQuery}
      onChange={handleSearch}
    />
  </div>
  
  {/* Autocomplete results */}
  {searchResults.length > 0 && (
    <ScrollArea className="max-h-[300px] mt-2">
      <div className="space-y-1">
        {searchResults.map(entity => (
          <button
            key={entity.id}
            onClick={() => focusNode(entity.id)}
            className="w-full text-left px-3 py-2 hover:bg-muted rounded-lg"
          >
            <p className="text-sm font-medium">{entity.name}</p>
            <p className="text-xs text-muted-foreground">{entity.type}</p>
          </button>
        ))}
      </div>
    </ScrollArea>
  )}
</div>
```

### R4. Interactive Legend
```tsx
<Card className="absolute bottom-4 left-4 p-3 w-64">
  <CardHeader className="p-0 pb-2">
    <CardTitle className="text-sm">Entity Types</CardTitle>
  </CardHeader>
  <CardContent className="p-0 space-y-1">
    {entityTypes.map(type => (
      <label key={type.name} className="flex items-center gap-2 cursor-pointer">
        <Checkbox
          checked={visibleTypes.has(type.name)}
          onCheckedChange={() => toggleType(type.name)}
        />
        <div 
          className="w-3 h-3 rounded-full" 
          style={{ backgroundColor: type.color }}
        />
        <span className="text-sm">{type.name}</span>
        <Badge variant="secondary" className="ml-auto text-xs">
          {type.count}
        </Badge>
      </label>
    ))}
  </CardContent>
</Card>
```

---

# Settings Screen (`/settings`)

**Screenshot Reference:** [`05-settings-full.png`](../audit_ui/screenshots/05-settings-full.png)

**Component Files:**
- Page: [`src/app/(dashboard)/settings/page.tsx`](../edgequake_webui/src/app/(dashboard)/settings/page.tsx)

## What I Reviewed
- Settings form with 1 input detected (very minimal)
- Expected: Tabs for different setting categories
- Expected: Multiple form sections

## Issues

### 🔴 Critical

**C1. Settings Appear Very Minimal**
- **Issue:** Only 1 input detected - likely a placeholder
- **Expected:** Comprehensive settings for:
  - User profile
  - LLM configuration
  - Storage/database settings
  - API keys
  - Query defaults
  - UI preferences
  - Advanced options

**C2. No Tabs for Organization**
- **Issue:** All settings in one long page is overwhelming
- **Recommendation:** Use tabs:
  - General
  - LLM & AI
  - Storage
  - API Keys
  - Appearance
  - Advanced

### 🟡 Major

**M1. No Right Panel for Help/Documentation**
- **Issue:** Complex settings need contextual help
- **Recommendation:** Right panel shows:
  - Help text for selected setting
  - Default values
  - Impact of changes
  - Related documentation links

**M2. No Save/Reset Indicators**
- **Issue:** User doesn't know if changes are saved
- **Recommendation:**
  - Unsaved changes badge
  - Save/Cancel buttons appear when modified
  - Toast notification on save
  - Reset to defaults button

**M3. No Validation Feedback**
- **Issue:** Invalid values not caught until save
- **Recommendation:** Real-time validation with error messages

## Recommendations

### R1. Tab-Based Settings Layout
```
┌──────────┬────────────────────────────────────────┬──────────────┐
│ Sidebar  │ Settings                               │ Help         │
│          ├────────────────────────────────────────┤              │
│          │ [General][LLM][Storage][API][Appearance]│              │
│          ├────────────────────────────────────────┤              │
│          │                                        │  ℹ️ Setting   │
│          │  Profile Settings                      │  Help        │
│          │  ┌────────────────────────────────┐   │              │
│          │  │ Name: [____________]           │   │  This        │
│          │  │ Email: [___________]           │   │  controls... │
│          │  │ Language: [English ▾]          │   │              │
│          │  └────────────────────────────────┘   │  Default:    │
│          │                                        │  English     │
│          │  Query Defaults                        │              │
│          │  ┌────────────────────────────────┐   │  Impact:     │
│          │  │ Mode: [Understand ▾]           │   │  Affects all │
│          │  │ Temperature: ─────●──── 0.7    │   │  queries     │
│          │  └────────────────────────────────┘   │              │
│          │                                        │              │
│          │  [Reset to Defaults] [Save Changes]   │              │
└──────────┴────────────────────────────────────────┴──────────────┘
```

### R2. Setting Categories
```tsx
const settingsTabs = [
  {
    id: 'general',
    label: 'General',
    icon: Settings,
    sections: [
      {
        title: 'Profile',
        settings: [
          { name: 'display_name', type: 'text', label: 'Display Name' },
          { name: 'email', type: 'email', label: 'Email' },
          { name: 'language', type: 'select', label: 'Language', options: [...] },
        ],
      },
      {
        title: 'Preferences',
        settings: [
          { name: 'theme', type: 'select', label: 'Theme', options: ['light', 'dark', 'system'] },
          { name: 'timezone', type: 'select', label: 'Timezone' },
        ],
      },
    ],
  },
  {
    id: 'llm',
    label: 'LLM & AI',
    icon: Brain,
    sections: [
      {
        title: 'LLM Provider',
        settings: [
          { name: 'provider', type: 'select', options: ['openai', 'anthropic', 'ollama'] },
          { name: 'api_key', type: 'password', label: 'API Key' },
          { name: 'model', type: 'select', label: 'Model' },
        ],
      },
      {
        title: 'Query Defaults',
        settings: [
          { name: 'temperature', type: 'slider', min: 0, max: 1, step: 0.1 },
          { name: 'max_tokens', type: 'number' },
          { name: 'top_p', type: 'slider', min: 0, max: 1 },
        ],
      },
    ],
  },
  // ... more tabs
];
```

---

# API Explorer Screen (`/api-explorer`)

**Screenshot Reference:** [`06-api-explorer-full.png`](../audit_ui/screenshots/06-api-explorer-full.png)

**Component Files:**
- Page: [`src/app/(dashboard)/api-explorer/page.tsx`](../edgequake_webui/src/app/(dashboard)/api-explorer/page.tsx)

## What I Reviewed
- Endpoint list (3 endpoints detected)
- Request/response interface
- API documentation

## Issues

### 🔴 Critical

**C1. No Interactive Request Builder**
- **Issue:** Users can't easily test API endpoints
- **Expected:** 
  - Dropdown to select endpoint
  - Form to input parameters
  - Execute button
  - Response display

**C2. No Authentication UI**
- **Issue:** API requires auth but no way to set tokens
- **Recommendation:** 
  - Header input for Authorization token
  - Save credentials option
  - Test connection button

### 🟡 Major

**M1. Limited to 3 Endpoints**
- **Issue:** EdgeQuake API has many more endpoints
- **Recommendation:** Generate from OpenAPI spec if available

**M2. No Code Examples**
- **Issue:** Developers need code snippets
- **Recommendation:** Show examples in:
  - cURL
  - JavaScript/TypeScript
  - Python
  - Rust

**M3. No Response Schema**
- **Issue:** Users don't know what to expect
- **Recommendation:** Show JSON schema for request/response

## Recommendations

### R1. Postman-Style Interface
```
┌──────────┬────────────────────────────────────────┬──────────────┐
│ Sidebar  │ API Explorer                           │ Docs         │
│          ├────────────────────────────────────────┤              │
│  📁 Auth │ GET /documents                         │  GET         │
│    POST  │ ┌────────────────────────────────────┐ │  /documents  │
│           │ │ Headers:                           │ │              │
│  📁 Docs  │ │ Authorization: [Bearer ...]        │ │  Retrieves   │
│    GET    │ │                                    │ │  list of     │
│    POST   │ │ Query Params:                      │ │  documents   │
│    DELETE │ │ page: [1]                          │ │              │
│           │ │ page_size: [20]                    │ │  Parameters: │
│  📁 Query │ │ status: [all ▾]                    │ │  • page      │
│    POST   │ └────────────────────────────────────┘ │  • page_size │
│           │                                        │  • status    │
│  📁 Graph │ [Send Request]                         │              │
│    GET    │                                        │  Response:   │
│           │ Response: 200 OK (234ms)               │  200 OK      │
│           │ ┌────────────────────────────────────┐ │  DocumentList│
│           │ │ {                                  │ │              │
│           │ │   "items": [...],                  │ │  Example:    │
│           │ │   "total": 42,                     │ │  [JSON...]   │
│           │ │   "page": 1                        │ │              │
│           │ │ }                                  │ │              │
│           │ └────────────────────────────────────┘ │              │
│           │                                        │              │
│           │ [Copy as cURL] [Python] [JavaScript]  │              │
└──────────┴────────────────────────────────────────┴──────────────┘
```

### R2. Code Generation
```tsx
const generateCurlCommand = (endpoint, params, headers) => {
  return `curl -X ${endpoint.method} '${endpoint.url}' \\
  -H 'Authorization: Bearer ${headers.auth}' \\
  -H 'Content-Type: application/json' \\
  ${params ? `-d '${JSON.stringify(params)}'` : ''}`;
};

const generatePythonCode = (endpoint, params) => {
  return `import requests

response = requests.${endpoint.method.toLowerCase()}(
    '${endpoint.url}',
    headers={'Authorization': 'Bearer YOUR_TOKEN'},
    json=${JSON.stringify(params, null, 2)}
)

print(response.json())`;
};
```

---

# Responsive Layouts Audit

## Tablet (768x1024)

**Screenshot References:**
- [`07-tablet-home.png`](../audit_ui/screenshots/07-tablet-home.png)
- [`07-tablet-query.png`](../audit_ui/screenshots/07-tablet-query.png)

### Issues
**M1. Sidebar Doesn't Auto-Collapse**
- **Issue:** Sidebar takes too much space on tablet
- **Recommendation:** Auto-collapse to icon-only at < 1024px

**M2. Right Panels Not Responsive**
- **Issue:** Right panels (sources, previews) remain fixed width
- **Recommendation:** Reduce to 280px or convert to bottom sheet

**M3. Stats Cards Grid**
- **Issue:** 4-column grid too cramped
- **Recommendation:** Use 2x2 grid on tablet

## Mobile (375x667)

**Screenshot References:**
- [`08-mobile-home.png`](../audit_ui/screenshots/08-mobile-home.png)
- [`08-mobile-query.png`](../audit_ui/screenshots/08-mobile-query.png)

### Issues
**M1. Mobile Menu Works**
- **Observation:** Menu button detected (good!)
- **Verify:** Ensure drawer opens smoothly

**M2. Stats Cards Stack**
- **Recommendation:** Single column on mobile

**M3. Query Input Too Small**
- **Issue:** Input area might be too cramped on mobile
- **Recommendation:** Make input bottom sheet on mobile

**M4. Right Panels Must Be Bottom Sheets**
- **Critical:** No room for side panels on mobile
- **Recommendation:** Sources become bottom sheet (swipe up)

---

# Accessibility Audit Summary

**Screenshot Reference:** Test ran on Dashboard

## Findings

### ✅ Good
- Skip link detected
- 6 aria-labeled elements
- Proper heading hierarchy (H1: 2, H2: 0, H3: 0)
- Keyboard navigation works (Tab key)

### ⚠️ Needs Improvement

**A1. Low Aria-Label Count**
- Only 6 elements have aria-labels
- **Recommendation:** Add to all interactive elements (buttons, inputs, links)

**A2. Button Color Contrast**
- Detected colors: `color: lab(2.75381 0 0)` on transparent background
- **Concern:** May fail WCAG AA contrast ratio (4.5:1 for normal text)
- **Action:** Verify contrast ratios across all themes

**A3. No Focus Indicators Visible**
- **Issue:** Focus states may not be prominent enough
- **Recommendation:** Add visible focus rings (2px outline, high contrast)

**A4. No Reduced Motion Support**
- **Issue:** Animations always on
- **Recommendation:** Respect `prefers-reduced-motion` media query

**A5. No Screen Reader Announcements**
- **Issue:** Dynamic content updates not announced
- **Recommendation:** Use `aria-live` regions for:
  - Loading states
  - Error messages
  - Success notifications
  - New messages in chat

---

# Priority Summary (All Screens)

## 🔥 Critical - Fix Immediately
1. Add collapsible left/right panels (Dashboard, Documents, Query, Graph)
2. Rich empty states (Query, Graph, Documents)
3. Auto-expanding textarea (Query)
4. Comprehensive settings implementation
5. Right panel for document preview
6. Color contrast fixes (Accessibility)

## 📌 Important - Next Sprint
7. Conversation management (Query)
8. Bulk selection (Documents)
9. Graph controls and filters
10. API Explorer interactive builder
11. Mobile-responsive panels (bottom sheets)
12. Search functionality (Documents, Graph)
13. Keyboard shortcuts across all pages

## 💡 Nice to Have - Future
14. Export features (conversations, graph)
15. Advanced visualizations
16. Query suggestions/autocomplete
17. Time-based graph navigation
18. Screen reader announcements
19. Path finding in graph
20. Code generation in API Explorer
