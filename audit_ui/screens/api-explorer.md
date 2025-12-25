# API Explorer Screen Audit

**Route:** `/api-explorer`  
**Viewport(s) Tested:** 320px, 428px, 768px, 1280px, 1536px  
**UI Regions:** Header, Sidebar, Breadcrumb, Request Panel, Response Panel, Endpoint Selector  
**States Captured:** Default, Loading, Response, Error  
**Screenshots:** `screenshots/screens/api-explorer/`  
**Relevant Files:** `src/app/(dashboard)/api-explorer/page.tsx`, `src/components/api-explorer/`

---

## What I Reviewed

### Layout Structure
```
┌─────────────────────────────────────────────────────────────┐
│ Header (fixed, h: 64px)                      │ API │ 🌐 ☀️ 👤│
├────────────┬────────────────────────────────────────────────┤
│ Sidebar    │ Breadcrumb: EdgeQuake > API Explorer           │
│ w: 256px   ├────────────────────────────────────────────────┤
│            │ API Explorer                    Interactive API│
│            │ testing for EdgeQuake                          │
│            ├────────────────────────────────────────────────┤
│            │ ┌─────────────────┬──────────────────────────┐ │
│            │ │ Endpoint        │ Response                 │ │
│            │ │ [GET ▾] [/api/..│                          │ │
│            │ ├─────────────────┤                          │ │
│            │ │ Headers         │                          │ │
│            │ │ Authorization:  │   No response yet        │ │
│            │ │ [Bearer sk-...] │                          │ │
│            │ ├─────────────────┤   Send a request to      │ │
│            │ │ Query Params    │   see the response       │ │
│            │ │ tenant=default  │   here.                  │ │
│            │ ├─────────────────┤                          │ │
│            │ │ Body (JSON)     │                          │ │
│            │ │ ┌─────────────┐ │                          │ │
│            │ │ │{            │ │                          │ │
│            │ │ │  "query": "",│ │                          │ │
│            │ │ │}            │ │                          │ │
│            │ │ └─────────────┘ │                          │ │
│            │ │ [Send Request]  │                          │ │
│            │ └─────────────────┴──────────────────────────┘ │
└────────────┴────────────────────────────────────────────────┘
```

---

## Slickness Score

| Criterion | Score (1–5) | Notes |
|-----------|-------------|-------|
| Visual refinement | 4.0 | Clean two-panel layout |
| Modern styling | 4.2 | Good code editor styling |
| Smooth interactions | 3.8 | Response could animate in |
| Professional polish | 4.0 | Feels like a proper API tool |
| **Overall** | **4.0** | Solid developer tool interface |

---

## Issues

### 🟠 Major

#### Response Panel Empty State
- **Severity:** 🟠 Major
- **Location:** Right response panel
- **Viewport(s) affected:** All
- **Current behavior:** Large empty area with small text
- **Expected behavior:** More prominent empty state with visual indicator

#### No Request History
- **Severity:** 🟠 Major
- **Location:** Missing feature
- **Viewport(s) affected:** All
- **Current behavior:** No history of previous requests
- **Expected behavior:** Request history with replay capability

#### Mobile Layout Breaks
- **Severity:** 🟠 Major
- **Location:** Two-panel layout
- **Viewport(s) affected:** 320px, 428px
- **Current behavior:** Panels may overflow or be too narrow
- **Expected behavior:** Stacked layout on mobile with tabs

---

### 🟡 Minor

#### Method Selector Color Coding
- **Severity:** 🟡 Minor
- **Location:** HTTP method dropdown
- **Viewport(s) affected:** All
- **Current behavior:** Same color for all methods
- **Expected behavior:** GET=green, POST=blue, PUT=orange, DELETE=red

#### JSON Editor Could Have Syntax Highlighting
- **Severity:** 🟡 Minor
- **Location:** Body JSON textarea
- **Viewport(s) affected:** All
- **Current behavior:** Plain textarea
- **Expected behavior:** Monaco editor or syntax highlighting

#### Response Time Not Shown
- **Severity:** 🟡 Minor
- **Location:** Response panel
- **Viewport(s) affected:** All
- **Current behavior:** No timing information
- **Expected behavior:** Show response time in ms

#### No Copy Response Button
- **Severity:** 🟡 Minor
- **Location:** Response panel
- **Viewport(s) affected:** All
- **Current behavior:** Must select and copy manually
- **Expected behavior:** One-click copy button

---

## Recommendations

### 1. Add HTTP Method Color Coding

**Change:** Color-code HTTP methods

**Specifications:**
```tsx
const methodColors = {
  GET: "bg-green-500/10 text-green-600 border-green-500/30",
  POST: "bg-blue-500/10 text-blue-600 border-blue-500/30",
  PUT: "bg-orange-500/10 text-orange-600 border-orange-500/30",
  PATCH: "bg-yellow-500/10 text-yellow-600 border-yellow-500/30",
  DELETE: "bg-red-500/10 text-red-600 border-red-500/30",
};

<Badge className={cn("font-mono", methodColors[method])}>
  {method}
</Badge>
```

**Acceptance Criteria:**
- [ ] Each method has distinct color
- [ ] Colors work in light and dark mode
- [ ] Method badge is prominent

---

### 2. Mobile Responsive Layout

**Change:** Stack panels on mobile with tab switcher

**Specifications:**
```tsx
// Mobile view
<div className="md:hidden">
  <Tabs defaultValue="request">
    <TabsList className="grid w-full grid-cols-2">
      <TabsTrigger value="request">Request</TabsTrigger>
      <TabsTrigger value="response">
        Response
        {response && <Badge variant="outline" className="ml-1">
          {response.status}
        </Badge>}
      </TabsTrigger>
    </TabsList>
    <TabsContent value="request">
      {/* Request form */}
    </TabsContent>
    <TabsContent value="response">
      {/* Response panel */}
    </TabsContent>
  </Tabs>
</div>

// Desktop view
<div className="hidden md:grid md:grid-cols-2 gap-4">
  {/* Side-by-side panels */}
</div>
```

**Acceptance Criteria:**
- [ ] Tabs on mobile
- [ ] Response status visible in tab
- [ ] Full-width panels on mobile

---

### 3. Add Response Metadata

**Change:** Show response time, status, and size

**Specifications:**
```tsx
<div className="flex items-center gap-4 border-b p-3">
  <Badge variant={statusVariant}>
    {response.status} {response.statusText}
  </Badge>
  <span className="text-sm text-muted-foreground">
    {response.time}ms
  </span>
  <span className="text-sm text-muted-foreground">
    {formatBytes(response.size)}
  </span>
  <div className="ml-auto flex gap-2">
    <Button variant="ghost" size="sm" onClick={copyResponse}>
      <Copy className="h-4 w-4" />
    </Button>
    <Button variant="ghost" size="sm" onClick={downloadResponse}>
      <Download className="h-4 w-4" />
    </Button>
  </div>
</div>
```

**Acceptance Criteria:**
- [ ] Status code with color (200=green, 400=orange, 500=red)
- [ ] Response time in ms
- [ ] Response size
- [ ] Copy and download buttons

---

### 4. Add Request History

**Change:** Show recent requests in collapsible sidebar

**Specifications:**
```tsx
<Sheet>
  <SheetTrigger asChild>
    <Button variant="outline" size="sm">
      <History className="h-4 w-4 mr-2" />
      History ({history.length})
    </Button>
  </SheetTrigger>
  <SheetContent side="left">
    <SheetHeader>
      <SheetTitle>Request History</SheetTitle>
    </SheetHeader>
    <div className="space-y-2 mt-4">
      {history.map(req => (
        <button
          key={req.id}
          onClick={() => loadRequest(req)}
          className="w-full text-left p-2 rounded hover:bg-muted"
        >
          <div className="flex items-center gap-2">
            <Badge className={methodColors[req.method]}>
              {req.method}
            </Badge>
            <span className="text-sm truncate">{req.path}</span>
          </div>
          <span className="text-xs text-muted-foreground">
            {formatRelative(req.timestamp)}
          </span>
        </button>
      ))}
    </div>
  </SheetContent>
</Sheet>
```

**Acceptance Criteria:**
- [ ] Shows last 20 requests
- [ ] Can click to replay
- [ ] Shows method, path, timestamp
- [ ] Persists in localStorage

---

### 5. Enhance JSON Editor

**Change:** Add syntax highlighting and validation

**Specifications:**
```tsx
import { JsonEditor } from '@/components/json-editor';

<JsonEditor
  value={body}
  onChange={setBody}
  height="200px"
  validate={true}
  onError={(errors) => setJsonErrors(errors)}
/>
```

Alternatively, use Monaco Editor:
```tsx
import Editor from '@monaco-editor/react';

<Editor
  height="200px"
  language="json"
  theme={theme === 'dark' ? 'vs-dark' : 'light'}
  value={body}
  onChange={setBody}
  options={{
    minimap: { enabled: false },
    lineNumbers: 'off',
    fontSize: 13,
  }}
/>
```

**Acceptance Criteria:**
- [ ] JSON syntax highlighting
- [ ] Error indicators for invalid JSON
- [ ] Bracket matching
- [ ] Auto-indent

---

## Measurements

| Element | Current | Recommended |
|---------|---------|-------------|
| Request panel width | 50% | 40-45% |
| Response panel width | 50% | 55-60% |
| JSON editor height | ~200px | 200px min, expandable |
| Response area height | Flexible | Full remaining height |
| Method badge width | Auto | ~60px consistent |

---

## Responsive Behavior

### Mobile (320-428px)
- ⚠️ Must use tabbed layout
- ⚠️ JSON editor needs full width
- ⚠️ Response should be scrollable
- ⚠️ Headers section should collapse

### Tablet (768px)
- ⚠️ Could use split or tabbed
- ✅ Both panels can fit
- ⚠️ May need narrower request panel

### Desktop (1280px+)
- ✅ Side-by-side works well
- ✅ Plenty of space for code
- ⚠️ Consider request history sidebar

---

## Accessibility

| Check | Status | Notes |
|-------|--------|-------|
| Form labels | ✅ Good | All inputs labeled |
| Method selector | ⚠️ Check | Needs aria-label |
| Response area | ⚠️ Check | Needs role="log" |
| Loading state | ⚠️ Check | Needs aria-busy |
| Error messages | ⚠️ Check | Needs aria-live |
| Keyboard shortcuts | ⚠️ Missing | Add Cmd+Enter to send |

### Recommended Keyboard Shortcuts
| Shortcut | Action |
|----------|--------|
| `Cmd+Enter` | Send request |
| `Cmd+L` | Focus URL input |
| `Cmd+K` | Clear response |
| `Cmd+H` | Toggle history |

---

## API Endpoints to Support

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/health` | GET | Health check |
| `/api/query` | POST | Query knowledge graph |
| `/api/documents` | GET | List documents |
| `/api/documents` | POST | Upload document |
| `/api/documents/:id` | DELETE | Delete document |
| `/api/graph` | GET | Get graph data |
| `/api/entities` | GET | List entities |
| `/api/settings` | GET | Get settings |
| `/api/settings` | PUT | Update settings |

---

## Example Request Templates

Consider adding quick-fill templates:
```tsx
const templates = [
  {
    name: "Query Knowledge Graph",
    method: "POST",
    path: "/api/query",
    body: {
      query: "What is EdgeQuake?",
      mode: "hybrid",
      tenant_id: "default"
    }
  },
  {
    name: "List Documents",
    method: "GET",
    path: "/api/documents",
    params: { tenant_id: "default" }
  },
  // ...
];
```

---

## Screenshots Reference

| State | Breakpoint | File |
|-------|------------|------|
| Default | Desktop 1280px | `06-api-explorer-desktop.png` |
| Default | Desktop L 1536px | `06-api-explorer-desktop-l.png` |
| Default | Tablet 768px | `06-api-explorer-tablet.png` |
| Default | Mobile L 428px | `06-api-explorer-mobile-l.png` |
| Loading | Desktop | `06-api-explorer-loading.png` |
| Response | Desktop | `06-api-explorer-response.png` |
| Error | Desktop | `06-api-explorer-error.png` |

---

*Last updated: December 25, 2025*
