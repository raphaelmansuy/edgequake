# EdgeQuake Source Citations - Comprehensive Test Plan

**Date**: 2025-12-31  
**Status**: Backend Complete, Frontend Testing Pending  
**Related Issues**: Line-based navigation, highlighting, overflow, graph explorer

## Test Environment Setup

### 1. Start PostgreSQL Backend

```bash
# Terminal 1: Database
docker run --name edgequake-pg \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=edgequake \
  -p 5432:5432 \
  -d postgres:15

# Wait for DB to start
sleep 5

# Terminal 2: Backend with PostgreSQL
export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/edgequake"
export OPENAI_API_KEY="sk-..."  # Real API key for entity extraction
cd edgequake/edgequake
cargo run --bin edgequake

# Terminal 3: Frontend
cd edgequake_webui
bun run dev
```

### 2. Prepare Test Data

Create test document with clear line structure and multiple entities:

```bash
curl -X POST "http://localhost:8080/api/v1/workspaces/default/documents" \
  -H "Content-Type: application/json" \
  -d '{
    "content": "# Software Development Team\n\n## Leadership\n\nLine 4: Sarah Chen is the technical lead at TechCorp.\nLine 5: She specializes in distributed systems and Rust programming.\nLine 6: Sarah reports to the CTO and manages a team of 8 engineers.\n\n## Quality Assurance\n\nLine 9: Mike Johnson leads the QA department.\nLine 10: He implemented comprehensive testing strategies including unit tests.\nLine 11: Mike previously worked at Google for 5 years.\n\n## Documentation\n\nLine 14: Emily Rodriguez manages the documentation team.\nLine 15: She maintains API docs, ADRs, and user guides.\nLine 16: Emily uses Swagger and Markdown extensively.\n\n## Infrastructure\n\nLine 19: David Kim handles monitoring and observability.\nLine 20: He set up APM, error tracking, and dashboards.\nLine 21: David is an expert in Prometheus and Grafana.",
    "title": "Team Structure Document",
    "async_processing": false,
    "metadata": {
      "source": "hr_database",
      "department": "engineering"
    }
  }'
```

Expected: Document ID returned, processing completes within 10-15 seconds.

## Test Suite

### Test 1: Line Numbers in API Response

**Objective**: Verify backend sends line numbers in query response

**Steps**:

```bash
# Submit query
curl -X POST "http://localhost:8080/api/v1/workspaces/default/query" \
  -H "Content-Type: application/json" \
  -d '{
    "query": "Who is Sarah Chen and what does she do?",
    "mode": "hybrid",
    "options": {
      "top_k": 3
    }
  }' | jq '.sources[].chunks[] | {id, start_line, end_line, chunk_index}'
```

**Expected Output**:

```json
{
  "id": "doc_xyz_chunk_0",
  "start_line": 4,
  "end_line": 6,
  "chunk_index": 0
}
```

**Pass Criteria**:

- [ ] Response includes non-null `start_line`
- [ ] Response includes non-null `end_line`
- [ ] Response includes non-null `chunk_index`
- [ ] Line numbers match actual chunk position in document

---

### Test 2: Frontend URL Parameter Generation

**Objective**: Verify clicking source citation generates correct URL with line parameters

**Steps**:

1. Navigate to http://localhost:3000/query
2. Submit query: "What is Sarah Chen's role?"
3. Wait for response with source citations
4. Click on first source citation chunk
5. Inspect browser URL

**Expected URL**:

```
http://localhost:3000/documents/{document_id}?start_line=4&end_line=6&chunk_index=0&highlight=chunk
```

**Pass Criteria**:

- [ ] URL contains `start_line` parameter
- [ ] URL contains `end_line` parameter
- [ ] URL contains `chunk_index` parameter
- [ ] URL contains `highlight=chunk` parameter
- [ ] Parameters match values from API response

**Debug Commands** (if fails):

```javascript
// In browser console
const citationLink = document.querySelector(
  '[data-testid="source-citation-chunk"]'
);
console.log("Citation href:", citationLink?.href);
console.log("Citation data:", citationLink?.dataset);
```

---

### Test 3: Document View Highlighting

**Objective**: Verify yellow highlight appears on correct lines in document view

**Steps**:

1. Follow Test 2 to navigate to document with line parameters
2. Wait for document to load
3. Inspect DOM for highlight elements
4. Verify scroll behavior

**Expected DOM**:

```html
<mark class="highlight-citation" data-line-start="4" data-line-end="6">
  Line 4: Sarah Chen is the technical lead at TechCorp. Line 5: She specializes
  in distributed systems and Rust programming. Line 6: Sarah reports to the CTO
  and manages a team of 8 engineers.
</mark>
```

**Expected CSS**:

```css
.highlight-citation {
  background-color: rgba(255, 255, 0, 0.3); /* Yellow with 30% opacity */
  padding: 2px 0;
  border-radius: 2px;
  transition: background-color 0.3s ease;
}
```

**Pass Criteria**:

- [ ] `mark.highlight-citation` element exists
- [ ] Element contains correct text from lines 4-6
- [ ] Yellow background is visible
- [ ] Page scrolls to highlighted section automatically
- [ ] Highlight persists for at least 3 seconds

**Debug Commands**:

```javascript
// Check if highlights exist
const highlights = document.querySelectorAll(".highlight-citation");
console.log("Highlight count:", highlights.length);
console.log("Highlight text:", highlights[0]?.textContent.substring(0, 100));
console.log("Computed style:", window.getComputedStyle(highlights[0]));

// Check ContentRenderer state
const renderer = document.querySelector('[data-component="ContentRenderer"]');
console.log("Renderer props:", renderer?.dataset);
```

---

### Test 4: Document Container Overflow

**Objective**: Verify source citations container doesn't overflow horizontally

**Steps**:

1. Submit query: "Tell me about the entire team structure"
2. Wait for response with many source citations
3. Inspect source citations panel
4. Check for horizontal scrollbar
5. Test with very long entity names

**Expected Behavior**:

- Container width: max 100% of parent
- Text wrapping: enabled (break-words, overflow-wrap)
- Horizontal scroll: none
- Long words: wrapped to next line

**Pass Criteria**:

- [ ] No horizontal scrollbar in source citations panel
- [ ] Long entity names wrap to next line
- [ ] Document IDs wrap if too long
- [ ] All content visible without horizontal scroll

**Debug Commands**:

```javascript
const citationsPanel = document.querySelector(
  '[data-testid="source-citations"]'
);
console.log("Panel width:", citationsPanel?.clientWidth);
console.log("Panel scrollWidth:", citationsPanel?.scrollWidth);
console.log(
  "Overflow:",
  citationsPanel?.scrollWidth > citationsPanel?.clientWidth
);

// Check CSS
console.log("CSS:", window.getComputedStyle(citationsPanel));
```

**CSS to Verify**:

```css
.source-citations-container {
  max-width: 100%;
  overflow-x: hidden;
  overflow-wrap: break-word;
  word-break: break-word;
}
```

---

### Test 5: Open Graph Explorer Navigation

**Objective**: Verify clicking "Open Graph Explorer" navigates to filtered graph view

**Steps**:

1. Submit query: "Who works on testing?"
2. Wait for response with entities (Mike Johnson, QA, Testing, etc.)
3. Click "Open Graph Explorer" button
4. Inspect URL parameters
5. Verify graph view shows filtered entities

**Expected URL**:

```
http://localhost:3000/graph?entities=MIKE_JOHNSON,QA,TESTING,GOOGLE
```

**Expected Behavior**:

- Graph view opens
- Only entities from URL parameter are visible
- Graph layout centers on filtered entities
- Related relationships are shown

**Pass Criteria**:

- [ ] Navigation to /graph occurs
- [ ] URL contains `entities` parameter
- [ ] Entity names are normalized (UPPERCASE, underscores)
- [ ] Graph view filters correctly
- [ ] At least 2 entities visible
- [ ] Relationships between entities shown

**Debug Commands**:

```javascript
// Check button click handler
const graphButton = document.querySelector(
  '[data-testid="open-graph-explorer"]'
);
console.log("Button exists:", !!graphButton);
console.log("Button onclick:", graphButton?.onclick);

// Check entity extraction
const entities = document.querySelectorAll("[data-entity-name]");
console.log(
  "Entities:",
  Array.from(entities).map((e) => e.dataset.entityName)
);
```

---

### Test 6: Multi-Chunk Highlighting

**Objective**: Verify highlighting works when clicking different chunks from same query

**Steps**:

1. Submit query with multiple source chunks: "What are all the team members' roles?"
2. Click first chunk citation
3. Verify first chunk highlights
4. Go back to query page
5. Click second chunk citation
6. Verify second chunk highlights (first highlight removed)

**Pass Criteria**:

- [ ] First click highlights correct lines
- [ ] Second click removes previous highlight
- [ ] Second click highlights new lines
- [ ] No overlap or interference between highlights
- [ ] URL updates correctly for each chunk

---

### Test 7: Edge Cases

**Test 7a: Single Line Chunk**

```json
{
  "start_line": 10,
  "end_line": 10,
  "chunk_index": 0
}
```

Expected: Single line highlighted, no errors

**Test 7b: Large Line Range**

```json
{
  "start_line": 1,
  "end_line": 50,
  "chunk_index": 0
}
```

Expected: All 50 lines highlighted, scrolls to start

**Test 7c: Missing Line Numbers**

```json
{
  "start_line": null,
  "end_line": null,
  "chunk_index": null
}
```

Expected: Full document shown, no highlight, no errors

**Test 7d: Invalid Line Numbers**

```json
{
  "start_line": 999,
  "end_line": 1001,
  "chunk_index": 0
}
```

Expected: Graceful fallback, error message or full document shown

---

### Test 8: Performance & Responsiveness

**Objective**: Verify highlighting performs well with large documents

**Steps**:

1. Upload document with 1000+ lines
2. Submit query that returns chunk from line 500-510
3. Click source citation
4. Measure time to highlight

**Pass Criteria**:

- [ ] Page loads within 2 seconds
- [ ] Highlight appears within 500ms
- [ ] Scroll completes within 300ms
- [ ] No UI freezing or jank

---

## Regression Tests

### R1: Previous Frontend Fix (commit 33a36e5)

- [ ] Source citations overflow fixed persists
- [ ] Line navigation infrastructure works

### R2: Query Functionality

- [ ] All query modes work (naive, local, global, hybrid)
- [ ] Entity extraction still functions
- [ ] Relationship detection unchanged

### R3: Document Upload

- [ ] Synchronous upload works
- [ ] Asynchronous upload works
- [ ] Large document handling unchanged

---

## Automated Testing

### Playwright Test Script

```typescript
import { test, expect } from "@playwright/test";

test.describe("Source Citation Line Highlighting", () => {
  test("should highlight correct lines when clicking source citation", async ({
    page,
  }) => {
    // Navigate to query page
    await page.goto("http://localhost:3000/query");

    // Submit query
    await page
      .getByRole("textbox", { name: "Ask a question..." })
      .fill("Who is Sarah Chen?");
    await page
      .getByRole("textbox", { name: "Ask a question..." })
      .press("Enter");

    // Wait for response
    await page.waitForSelector('[data-testid="source-citation-chunk"]', {
      timeout: 15000,
    });

    // Get citation link
    const citation = page.getByTestId("source-citation-chunk").first();
    const href = await citation.getAttribute("href");

    // Verify URL has line parameters
    expect(href).toMatch(/start_line=\d+/);
    expect(href).toMatch(/end_line=\d+/);
    expect(href).toMatch(/chunk_index=\d+/);

    // Click citation
    await citation.click();

    // Wait for document page
    await page.waitForURL(/\/documents\/.*\?.*start_line=/, { timeout: 5000 });

    // Check for highlight element
    const highlight = page.locator(".highlight-citation");
    await expect(highlight).toBeVisible({ timeout: 3000 });

    // Verify highlight color
    const bgColor = await highlight.evaluate(
      (el) => window.getComputedStyle(el).backgroundColor
    );
    expect(bgColor).toContain("255, 255, 0"); // Yellow
  });

  test("should handle missing line numbers gracefully", async ({ page }) => {
    // Navigate directly with no line params
    await page.goto("http://localhost:3000/documents/test-doc-id");

    // Should show full document without errors
    await expect(
      page.locator('[data-component="ContentRenderer"]')
    ).toBeVisible();

    // No highlights should exist
    const highlights = page.locator(".highlight-citation");
    await expect(highlights).toHaveCount(0);
  });
});
```

---

## Success Criteria

**Backend**:

- [x] Line numbers stored in vector metadata
- [x] SOTA engine extracts line numbers
- [x] API response includes line numbers
- [x] All compilation errors fixed
- [x] Service builds and runs

**Frontend**:

- [ ] URL parameters generated correctly
- [ ] Highlighting renders visually
- [ ] Scroll-to-line works
- [ ] Container overflow fixed
- [ ] Graph explorer navigation works

**End-to-End**:

- [ ] Complete flow works: query → citation click → document highlight
- [ ] No regressions in existing functionality
- [ ] Performance acceptable (<2s page load, <500ms highlight)

---

## Troubleshooting Guide

### Issue: No Line Numbers in API Response

**Symptoms**: API returns null for start_line, end_line  
**Checks**:

1. Verify vector metadata includes line numbers:
   ```bash
   # Check backend logs for "VECTOR STORAGE: Chunk embedding stored"
   tail -f /tmp/backend.log | grep "VECTOR"
   ```
2. Verify document was processed with line tracking:
   ```bash
   curl http://localhost:8080/api/v1/workspaces/default/documents/{doc_id}
   ```
3. Check pipeline configuration:
   ```rust
   // In pipeline.rs, verify enable_lineage_tracking = true
   ```

**Fix**: Ensure `start_line`, `end_line`, `chunk_index` are in metadata when upserting to vector storage

---

### Issue: Highlighting Not Appearing

**Symptoms**: Navigate to document but no yellow highlight  
**Checks**:

1. Inspect URL parameters
2. Check ContentRenderer component
3. Verify CSS classes
4. Check browser console for errors

**Debug**:

```javascript
// In browser console on document page
const params = new URLSearchParams(window.location.search);
console.log("start_line:", params.get("start_line"));
console.log("end_line:", params.get("end_line"));

// Check if applyLineHighlight was called
console.log(
  "ContentRenderer highlights:",
  document.querySelectorAll(".highlight-citation").length
);
```

**Fix**: Check ContentRenderer.applyLineHighlight() implementation

---

### Issue: Graph Explorer Not Filtering

**Symptoms**: Graph shows all entities, not filtered set  
**Checks**:

1. Verify URL parameter format
2. Check entity name normalization
3. Verify GraphView component reads params

**Debug**:

```javascript
// On graph page
const params = new URLSearchParams(window.location.search);
const entities = params.get("entities");
console.log("Entity filter:", entities);
console.log("Parsed entities:", entities?.split(","));
```

---

## Appendix: File References

**Backend**:

- Pipeline: `edgequake/crates/edgequake-pipeline/src/pipeline.rs`
- Lineage: `edgequake/crates/edgequake-pipeline/src/lineage.rs`
- Query Handler: `edgequake/crates/edgequake-api/src/handlers/query.rs`
- Document Handler: `edgequake/crates/edgequake-api/src/handlers/documents.rs`
- SOTA Engine: `edgequake/crates/edgequake-query/src/sota_engine.rs`

**Frontend**:

- ContentRenderer: `edgequake_webui/components/ContentRenderer.tsx`
- SourceCitations: `edgequake_webui/components/SourceCitations.tsx`
- QueryPage: `edgequake_webui/app/query/page.tsx`
- DocumentPage: `edgequake_webui/app/documents/[id]/page.tsx`
- GraphPage: `edgequake_webui/app/graph/page.tsx`

---

**Test Plan Version**: 1.0  
**Last Updated**: 2025-12-31  
**Status**: Ready for Execution
