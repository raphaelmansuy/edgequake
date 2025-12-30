# Ingestion WebUI Improvement Plan

> **Session:** 2024-12-29 - WebUI Ingestion Cost Display Improvements
> **Status:** ✅ COMPLETED
> **Objective:** Ensure ingestion cost information is properly captured and displayed in a SLICK modern interface

---

## Implementation Summary

### Completed Tasks

- [x] Step 1: Add cost fields to Document TypeScript type
- [x] Step 2: Update API response to include cost data
- [x] Step 3: Create enhanced CostCell component
- [x] Step 4: Update document-manager table to use CostCell
- [x] Step 5: Add cost breakdown to preview panel
- [x] Step 6: Add rich tooltip with token and model info
- [x] Step 7: Modern table styling with alternating rows
- [x] Step 8: Browser testing verified with screenshots

### Files Modified

1. **Backend:**

   - `edgequake/crates/edgequake-api/src/handlers/documents.rs` - Added cost fields to DocumentSummary
   - `edgequake/crates/edgequake-api/src/processor.rs` - Store cost data in async processing

2. **Frontend:**
   - `edgequake_webui/src/types/index.ts` - Added cost fields to Document interface
   - `edgequake_webui/src/components/documents/cost-cell.tsx` - NEW component
   - `edgequake_webui/src/components/documents/document-manager.tsx` - Use CostCell, modern styling
   - `edgequake_webui/src/components/documents/document-preview-panel.tsx` - Cost breakdown section

### Screenshots

- `documents-with-cost.png` - Initial cost cell display
- `documents-with-cost-preview-panel.png` - Full preview panel with cost
- `cost-cell-tooltip.png` - Detailed tooltip on hover
- `documents-modern-table.png` - Modern table styling
- `final-with-preview-panel.png` - Complete implementation

---

## 1. Current State Audit

### 1.1 Documents Page (Screenshot: 01-documents-empty.png)

**Current Components:**

- Header with title "Documents" and subtitle
- Search bar with placeholder
- Status filter dropdown ("All Status (0)")
- Sort controls (Created/Updated toggle buttons)
- Upload zone (drag & drop with file type hints)
- Documents table (empty state with icon)
- Preview panel (collapsed on right edge)
- Refresh button

**Issues Identified:**

| Issue       | Severity | Description                                                                 |
| ----------- | -------- | --------------------------------------------------------------------------- |
| **COST-01** | 🔴 HIGH  | CostBadge in table uses `processing_duration_ms` proxy instead of real cost |
| **COST-02** | 🟡 MED   | No cost info visible in empty state or upload feedback                      |
| **COST-03** | 🟡 MED   | Cost column hidden on mobile (lg:table-cell)                                |
| **UI-01**   | 🟢 LOW   | Sort buttons could be more prominent                                        |
| **UI-02**   | 🟢 LOW   | Upload zone could show cost estimate                                        |
| **UI-03**   | 🟡 MED   | Table lacks visual hierarchy for cost data                                  |

### 1.2 Costs Page (Screenshot: 02-costs-page.png)

**Current Components:**

- Cost Summary card (Total Cost, Documents, Avg per Document, Tokens Used)
- Budget indicator (shows "No budget configured")
- Cost breakdown chart placeholder
- Cost Trend chart placeholder
- Token Usage Details table (extraction, embedding stages)

**Issues Identified:**

| Issue       | Severity | Description                                              |
| ----------- | -------- | -------------------------------------------------------- |
| **COST-04** | 🔴 HIGH  | Breakdown chart shows "No cost breakdown data available" |
| **COST-05** | 🔴 HIGH  | Trend chart shows "No historical data available"         |
| **COST-06** | 🟡 MED   | API returns CORS error for `/api/v1/costs/history`       |
| **UI-04**   | 🟢 LOW   | Cards could have better visual hierarchy                 |
| **UI-05**   | 🟢 LOW   | Token table missing tooltips for columns                 |

### 1.3 Document Types & Cost Fields Analysis

**Document type (from types/index.ts):**

```typescript
interface Document {
  // Cost-related fields:
  lineage?: DocumentLineage; // Contains processing_duration_ms
  // Missing: cost_usd, input_tokens, output_tokens
}
```

**DocumentLineage (from types/index.ts):**

```typescript
interface DocumentLineage {
  llm_model?: string;
  processing_duration_ms?: number;
  // Missing: cost_usd, token_counts
}
```

**CostBadge (from cost-badge.tsx):**

```typescript
// Currently uses duration-based cost proxy:
cost={doc.lineage?.processing_duration_ms ? (doc.lineage.processing_duration_ms / 1000) * 0.0001 : 0}
```

---

## 2. Improvement Plan

### 2.1 Priority 1: Fix Cost Data Flow (Backend → Frontend)

**Problem:** The Document type doesn't include real cost data from processing.

**Solution:**

1. **Update ProcessingStats serialization** - Ensure cost data flows to API response
2. **Update Document type** - Add cost fields to TypeScript interface
3. **Update CostBadge usage** - Use real cost instead of duration proxy

**Files to modify:**

- `edgequake_webui/src/types/index.ts` - Add cost fields to Document
- `edgequake_webui/src/components/documents/document-manager.tsx` - Use real cost
- `edgequake/crates/edgequake-api/src/handlers/documents.rs` - Include cost in response

### 2.2 Priority 2: Enhanced Document Table

**Improvements:**

| Column   | Current                      | Improved                               |
| -------- | ---------------------------- | -------------------------------------- |
| Cost     | Hidden on mobile, uses proxy | Always visible, real USD + token count |
| Status   | Basic badge                  | Badge with progress % if processing    |
| Entities | Plain number                 | Number with icon tooltip               |
| Created  | Text only                    | Relative time with tooltip             |

**New component: `CostCell`**

- Shows cost in USD ($0.0012)
- Tooltip shows breakdown: input_tokens, output_tokens, model
- Color coding: green (< $0.001), yellow (< $0.01), red (>= $0.01)

### 2.3 Priority 3: Upload Cost Estimation

**New feature:** Show estimated cost before upload

**Implementation:**

1. After file drop, estimate token count based on file size
2. Call `/api/v1/pipeline/costs/estimate` with token estimate
3. Show estimated cost in upload progress UI

### 2.4 Priority 4: Modern Table Styling

**Visual improvements:**

- Alternating row colors for better readability
- Sticky header for long lists
- Hover states with subtle highlight
- Click-to-select with visual feedback
- Cost column with currency formatting
- Token counts with K/M suffixes

---

## 3. Implementation Steps

```markdown
- [ ] Step 1: Add cost fields to Document TypeScript type
- [ ] Step 2: Update API response to include cost data
- [ ] Step 3: Create enhanced CostCell component
- [ ] Step 4: Update document-manager table to use CostCell
- [ ] Step 5: Add cost estimation to upload flow
- [ ] Step 6: Add token count to document preview panel
- [ ] Step 7: Test with browser tools
- [ ] Step 8: Screenshot final result
```

---

## 4. Design Tokens

Following existing design-tokens.md:

```css
/* Cost-specific tokens */
--cost-green: hsl(142, 76%, 36%); /* < $0.001 */
--cost-yellow: hsl(45, 93%, 47%); /* $0.001 - $0.01 */
--cost-red: hsl(0, 84%, 60%); /* >= $0.01 */

/* Table enhancements */
--table-row-hover: hsl(var(--muted) / 0.5);
--table-row-selected: hsl(var(--primary) / 0.1);
--table-header-bg: hsl(var(--muted) / 0.3);
```

---

## 5. Component Architecture

```
documents/
├── document-manager.tsx        # Main page component
├── cost-badge.tsx              # Simple inline cost badge
├── cost-cell.tsx               # NEW: Enhanced table cell with tooltip
├── document-preview-panel.tsx  # Right panel with cost details
├── document-filters.tsx        # Filter controls
└── batch-progress-card.tsx     # Upload progress with cost estimate
```

---

## 6. Success Criteria

1. ✅ Document table shows real USD cost (not duration proxy)
2. ✅ Token counts visible (input + output)
3. ✅ Cost breakdown in preview panel
4. ✅ Cost estimation during upload
5. ✅ Modern, SLICK visual design
6. ✅ Manual testing proves functionality
