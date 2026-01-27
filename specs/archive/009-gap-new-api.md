# Cross-Language Implementation Gap Analysis: Web UI Migration

## Context

| Role | Language | Stack | Location |
|------|----------|-------|----------|
| Source (Reference) | React/JavaScript | Current Web UI | `./edgequake_webui` |
| Target (Implementation) | Next.js 16 | React 19, Server Components, App Router | `./edgequake_webui` |
| API Layer | REST API | New REST API endpoints | `./edgequake` |
| Documentation | Markdown | - | `./gap_analysis_api` |

**Source Implementation:** Existing Web UI consuming legacy API endpoints with established component library and user interactions.

**Target Implementation:** Next.js 16 application leveraging React Server Components, the App Router, and consuming a new REST API with potentially different endpoint structures, authentication patterns, and response schemas.

**Analysis Goal:** Ensure the Next.js 16 UI provides complete feature parity with the existing Web UI while properly adapting to the new REST API contract.

---

## Gap Analysis Scratchpad Template

```markdown
# Gap Analysis - Working Notes

## Last Updated: [timestamp]
## Current Phase: [inventory|mapping|analysis|roadmap|review]
## Current File: [path]

### Progress
- Source UI components analyzed: [X/Y]
- Target UI components analyzed: [X/Y]
- API endpoints mapped: [X/Y]
- Features mapped: [X/Y]

### Component Registry
| ID | Component/Feature | Source Status | Target Status | Gap Type |
|----|-------------------|---------------|---------------|----------|
| C001 | [name] | ✅ complete | ⚠️ partial | [type] |

### API Mapping Registry
| ID | Old Endpoint | New Endpoint | Breaking Changes |
|----|--------------|--------------|------------------|
| A001 | GET /api/v1/users | GET /api/v2/users | Schema changed |

### Completed
- [file]: [status]

### Findings
#### Parity Achieved
- [component]: [source file] ↔ [target file]

#### Gaps Identified
- [GAP-XXX]: [feature] - [severity] - [description]

#### API Breaking Changes
- [API-XXX]: [endpoint] - [change description]

#### Target Exceeds Source
- [feature]: [description of enhancement]

#### Ambiguous/Needs Clarification
- [item]: [question]

### Pending Actions
- [ ] [action item]
```

---

## UI Component Migration Template

```markdown
## Component: [C-XXX] [Component Name]

**Category:** [LAYOUT|NAVIGATION|FORM|DATA-DISPLAY|FEEDBACK|OVERLAY|UTILITY]
**Priority:** [P0|P1|P2|P3|P4]

### Source Implementation

**Location:** `[file path]`
**Type:** [Client Component | Static | SSR-Ready]

#### Props Interface
```typescript
interface SourceProps {
  prop1: type;
  prop2: type;
}
```

#### API Dependencies
| Endpoint | Method | Purpose |
|----------|--------|---------|
| /api/v1/resource | GET | Fetch data |

#### State Management
- Local state: [description]
- Global state: [Redux/Context/Zustand]
- Server state: [React Query/SWR/custom]

#### User Interactions
- [interaction 1]: [behavior]
- [interaction 2]: [behavior]

### Target Implementation (Next.js 16)

**Location:** `[file path]` | `NOT IMPLEMENTED`
**Type:** [Server Component | Client Component | Hybrid]

#### Architectural Decisions
- [ ] Server Component (default, no interactivity)
- [ ] Client Component ('use client' directive needed)
- [ ] Hybrid (Server wrapper with Client islands)

#### Props Interface
```typescript
interface TargetProps {
  prop1: type;
  prop2: type;
}
```

#### New API Integration
| New Endpoint | Method | Migration Notes |
|--------------|--------|-----------------|
| /api/v2/resource | GET | [changes] |

#### Data Fetching Pattern
```typescript
// Server Component pattern
async function Component() {
  const data = await fetch('/api/v2/resource');
  return <ClientComponent data={data} />;
}
```

### Gap Analysis

**Status:** [✅ Parity | ⚠️ Partial | ❌ Missing | 🔄 Divergent]
**Gap Type:** [MISSING|PARTIAL|DIVERGENT|ARCH|API-BREAKING]
**Severity:** [P0|P1|P2|P3|P4]

#### Migration Considerations
- [ ] Needs 'use client' directive
- [ ] Requires API response transformation
- [ ] Authentication pattern change
- [ ] Error boundary updates needed
- [ ] Loading state handling changed
- [ ] SEO/metadata updates required

#### Breaking API Changes Impact
[How API changes affect this component]

### Remediation

#### Implementation Steps
1. [step 1]
2. [step 2]

#### API Adapter Pattern (if needed)
```typescript
// Transform old API response to new format
function adaptResponse(newApiResponse: NewType): OldType {
  return {
    // mapping
  };
}
```

#### Effort Estimate
- **Complexity:** [Low|Medium|High]
- **Estimated Time:** [hours/days]
```

---

## API Migration Mapping Template

```markdown
## API Endpoint: [A-XXX] [Endpoint Purpose]

### Endpoint Mapping

| Aspect | Old API | New API |
|--------|---------|---------|
| URL | `/api/v1/resource` | `/api/v2/resources` |
| Method | GET | GET |
| Auth | Bearer Token | OAuth2 + API Key |

### Request Schema Comparison

**Old Request:**
```typescript
interface OldRequest {
  userId: number;
  filters: string;
}
```

**New Request:**
```typescript
interface NewRequest {
  user_id: string;  // Changed: number → string
  filters: {        // Changed: string → object
    status: string[];
    dateRange: { from: string; to: string };
  };
}
```

### Response Schema Comparison

**Old Response:**
```typescript
interface OldResponse {
  data: Item[];
  total: number;
}
```

**New Response:**
```typescript
interface NewResponse {
  items: Item[];      // Renamed: data → items
  pagination: {
    total: number;
    page: number;
    pageSize: number;
  };
}
```

### Breaking Changes

| Change | Type | Impact | Adapter Required |
|--------|------|--------|------------------|
| `data` → `items` | Rename | All consumers | Yes |
| Pagination structure | Restructure | List components | Yes |
| userId type | Type change | Auth flows | Yes |

### Adapter Implementation

```typescript
// api/adapters/resource.adapter.ts

interface LegacyResponse {
  data: Item[];
  total: number;
}

export function adaptResourceResponse(
  newResponse: NewResponse
): LegacyResponse {
  return {
    data: newResponse.items,
    total: newResponse.pagination.total,
  };
}

export function adaptResourceRequest(
  oldRequest: OldRequest
): NewRequest {
  return {
    user_id: String(oldRequest.userId),
    filters: JSON.parse(oldRequest.filters),
  };
}
```

### Affected Components
- [C-001]: [component name]
- [C-015]: [component name]
```

---

## Next.js 16 Specific Migration Patterns

### Server Component Data Fetching

```markdown
## Pattern: Server-Side Data Fetching

### Source Pattern (Client-Side)
```tsx
// Old: Client component with useEffect
'use client';
import { useState, useEffect } from 'react';

function DataList() {
  const [data, setData] = useState([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    fetch('/api/v1/items')
      .then(res => res.json())
      .then(setData)
      .finally(() => setLoading(false));
  }, []);

  if (loading) return <Spinner />;
  return <List items={data} />;
}
```

### Target Pattern (Server Component)
```tsx
// New: Server component with async/await
import { adaptItemsResponse } from '@/adapters/items';

async function DataList() {
  const response = await fetch('https://api.new.com/v2/items', {
    headers: { 'Authorization': `Bearer ${getToken()}` },
    next: { revalidate: 60 } // ISR pattern
  });
  
  const newData = await response.json();
  const data = adaptItemsResponse(newData); // API adapter
  
  return <List items={data} />;
}

// Loading state handled by loading.tsx
// Error state handled by error.tsx
```

### Migration Checklist
- [ ] Move data fetching from useEffect to component body
- [ ] Add API response adapter
- [ ] Create loading.tsx for Suspense boundary
- [ ] Create error.tsx for error boundary
- [ ] Update authentication header pattern
- [ ] Configure caching/revalidation strategy
```

### Client Component Interactivity

```markdown
## Pattern: Interactive Components

### When to Use 'use client'
- Event handlers (onClick, onChange, onSubmit)
- useState, useEffect, useReducer
- Browser-only APIs (localStorage, window)
- Third-party client libraries

### Hybrid Pattern
```tsx
// Server Component wrapper
async function ProductPage({ id }: { id: string }) {
  const product = await fetchProduct(id); // Server fetch
  
  return (
    <div>
      <ProductDetails product={product} />    {/* Server */}
      <AddToCartButton productId={id} />      {/* Client */}
      <ProductReviews productId={id} />       {/* Server */}
    </div>
  );
}

// Client Component island
'use client';
function AddToCartButton({ productId }: { productId: string }) {
  const [adding, setAdding] = useState(false);
  
  const handleAdd = async () => {
    setAdding(true);
    await addToCart(productId);
    setAdding(false);
  };
  
  return <button onClick={handleAdd} disabled={adding}>Add to Cart</button>;
}
```
```

---

## Feature Classification for UI Migration

| Category | Code | Description | Examples |
|----------|------|-------------|----------|
| Layout | `LAYOUT` | Page structure and composition | Headers, sidebars, grids |
| Navigation | `NAV` | Routing and navigation | Links, breadcrumbs, tabs |
| Forms | `FORM` | User input and validation | Inputs, selects, file uploads |
| Data Display | `DATA` | Presenting information | Tables, cards, lists |
| Feedback | `FEED` | User feedback mechanisms | Toasts, alerts, progress |
| Authentication | `AUTH` | Login, sessions, permissions | Login forms, auth guards |
| State | `STATE` | Global state management | Stores, contexts, caches |
| API Integration | `API` | Backend communication | Fetching, mutations, caching |

---

## Gap Severity for UI Migration

| Severity | Code | Definition |
|----------|------|------------|
| Critical | `P0` | Core functionality broken; users cannot complete primary tasks |
| High | `P1` | Important feature missing; significant UX degradation |
| Medium | `P2` | Feature works but with limitations or workarounds |
| Low | `P3` | Minor issues, cosmetic differences |
| Enhancement | `P4` | Opportunity to improve beyond source |

---

## Output Documents

### 1. UI Gap Analysis Report (`ui-gap-analysis.md`)

### 2. API Migration Guide (`api-migration-guide.md`)

### 3. Component Parity Matrix (`component-parity-matrix.md`)

### 4. Migration Roadmap (`migration-roadmap.md`)

---

## Quick Start Checklist

```markdown
## Pre-Migration Analysis
- [ ] Document all existing UI routes/pages
- [ ] Inventory all components with their API dependencies
- [ ] Map old API endpoints to new API endpoints
- [ ] Identify schema changes between APIs
- [ ] Document authentication/authorization changes

## Component Analysis
- [ ] Classify each component: Server vs Client
- [ ] Identify components requiring 'use client'
- [ ] Map data fetching patterns to new patterns
- [ ] Document state management dependencies

## API Adaptation
- [ ] Create adapter functions for breaking changes
- [ ] Define TypeScript interfaces for old and new schemas
- [ ] Implement request transformers
- [ ] Implement response transformers

## Next.js 16 Specific
- [ ] Design app router folder structure
- [ ] Plan loading.tsx placement for Suspense
- [ ] Plan error.tsx placement for error boundaries
- [ ] Define metadata strategy for SEO
- [ ] Plan server action usage for mutations
```

---

This adapted prompt focuses specifically on the challenges of migrating  Next.js 16 while simultaneously adapting to a new REST API, addressing the dual concerns of frontend architectural changes and backend contract modifications.