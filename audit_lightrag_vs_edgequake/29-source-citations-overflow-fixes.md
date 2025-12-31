# Source Citations Overflow & Navigation Fixes

**Date:** 2024-12-31  
**Status:** Planning → Implementation  
**Priority:** HIGH - User-reported UX blockers

## Executive Summary

Five critical UX issues identified in the Source Citations component and Document Detail page that prevent users from effectively navigating and understanding source content:

1. **Chunks overflow document container** - content not constrained
2. **Key Topics/Connections overflow container** - badge flex-wrap issues
3. **No chunk-to-document navigation** - missing line number highlighting
4. **Right sidebar not scrollable** - metadata sidebar scroll broken
5. **Chunk extraction not line-based** - backend splits on characters, not lines

## Issue Analysis

### Issue 1: Chunks Overflow in Documents Tab
**Location:** `source-citations.tsx` DocumentsTab component  
**Root Cause:** Passage content (150 chars) lacks proper text truncation and container constraints  

```tsx
<p className="text-[11px] text-muted-foreground line-clamp-2 flex-1 leading-relaxed">
  {chunk.content.slice(0, 150)}{chunk.content.length > 150 ? '...' : ''}
</p>
```

**Problem:** `line-clamp-2` alone doesn't prevent long words from overflowing horizontally.

### Issue 2: Key Topics/Connections Overflow
**Location:** `source-citations.tsx` KnowledgeTab component  
**Root Cause:** Badge flex-wrap works, but ScrollArea height is fixed at 300px which clips content

```tsx
<ScrollArea className="max-h-[300px]">
  <div className="space-y-5 pr-2">
```

**Problem:** When 25+ topics render, the container doesn't show all items despite being scrollable.

### Issue 3: No Line-Based Navigation
**Location:** Multiple files  
**Root Cause:** Navigation doesn't include line numbers, no highlighting mechanism

**Current flow:**
```typescript
// source-citations.tsx
onClick={() => onDocumentClick?.(docId, chunk.content, chunkIdx)}

// query-page.tsx
const handleDocumentClick = (docId: string, content?: string) => {
  router.push(`/documents/${docId}${content ? '?highlight=' + encodeURIComponent(content) : ''}`);
};

// documents/[id]/page.tsx
const highlightText = searchParams.get('highlight') || undefined;
<ContentRenderer document={document} highlightText={highlightText} />
```

**Problems:**
1. No `start_line`/`end_line` passed in URL
2. No highlighting with "stabilo" (highlighter pen) style
3. QueryContext chunks missing `start_line`, `end_line` fields
4. Backend has line data but API doesn't expose it

### Issue 4: Right Sidebar Not Scrollable  
**Location:** `documents/[id]/page.tsx` MetadataSidebar  
**Root Cause:** Parent container missing proper flex/overflow setup

```tsx
<div className="w-[35%] shrink-0">
  <MetadataSidebar document={document} />
</div>
```

**Problem:** Sidebar has `h-full` but parent `.flex` doesn't set `overflow-hidden` on child.

### Issue 5: Chunk Extraction Not Line-Based
**Location:** Backend `edgequake/crates/edgequake-core/src/chunking/`  
**Root Cause:** TextSplitter uses character offsets, not line boundaries

**Current:** Chunks split at character positions (e.g., 1200 chars + overlap)  
**Desired:** Chunks split at newline boundaries for clean line-based citations

## Implementation Plan

### Phase 1: Frontend Overflow Fixes (30 min)

#### Task 1.1: Fix Chunk Content Overflow
**File:** `source-citations.tsx` line ~230

**Changes:**
```tsx
<p className="text-[11px] text-muted-foreground line-clamp-2 flex-1 leading-relaxed break-words overflow-hidden">
  {chunk.content.slice(0, 150)}{chunk.content.length > 150 ? '...' : ''}
</p>
```

**Add:** `break-words overflow-hidden` for horizontal constraint

#### Task 1.2: Fix Key Topics/Connections Overflow
**File:** `source-citations.tsx` line ~305

**Changes:**
```tsx
<ScrollArea className="h-[400px]"> {/* increased from 300px */}
  <div className="space-y-5 pr-4"> {/* pr-4 for scroll gutter */}
```

**Rationale:** More vertical space prevents truncation, better padding for scrollbar

#### Task 1.3: Fix Document Title Overflow  
**File:** `source-citations.tsx` line ~195

**Changes:**
```tsx
<button
  className="text-sm font-medium flex items-center gap-1.5 hover:text-primary transition-colors text-left max-w-full overflow-hidden"
  onClick={() => onDocumentClick?.(docId, chunks[0]?.content, 0)}
  title={`Open: ${getDocumentTitle(chunks)}`}
>
  <FileText className="h-3.5 w-3.5 text-muted-foreground flex-shrink-0" />
  <span className="truncate">
    {getDocumentTitle(chunks)}
  </span>
</button>
```

**Add:** Wrap title text in `<span className="truncate">` for proper ellipsis

### Phase 2: Line-Based Navigation (60 min)

#### Task 2.1: Update QueryContext Type
**File:** `types/index.ts` line ~245

**Changes:**
```typescript
export interface QueryContext {
  chunks: Array<{
    content: string;
    document_id: string;
    score: number;
    file_path?: string;
    start_line?: number;      // ADD
    end_line?: number;        // ADD
    chunk_index?: number;     // ADD
  }>;
  // ... rest unchanged
}
```

#### Task 2.2: Update Navigation to Include Line Numbers
**File:** `source-citations.tsx` line ~225

**Changes:**
```tsx
<button
  key={chunkIdx}
  onClick={() => onDocumentClick?.(
    docId, 
    chunk.content, 
    chunkIdx,
    chunk.start_line,  // ADD
    chunk.end_line     // ADD
  )}
  className="w-full text-left p-2 rounded bg-muted/40 hover:bg-muted/70 transition-colors group/chunk"
>
  <div className="flex items-start gap-2">
    <Badge variant="outline" className="text-[9px] h-4 px-1 flex-shrink-0 mt-0.5">
      {chunkIdx + 1}
    </Badge>
    <p className="text-[11px] text-muted-foreground line-clamp-2 flex-1 leading-relaxed break-words overflow-hidden">
      {chunk.content.slice(0, 150)}{chunk.content.length > 150 ? '...' : ''}
    </p>
    <span className={`text-[9px] flex-shrink-0 ${getConfidenceLabel(chunk.score).color}`}>
      {Math.round(chunk.score * 100)}%
    </span>
  </div>
  {/* ADD: Line range display */}
  {chunk.start_line !== undefined && chunk.end_line !== undefined && (
    <div className="text-[9px] text-muted-foreground mt-1 pl-6">
      Lines {chunk.start_line}-{chunk.end_line}
    </div>
  )}
</button>
```

#### Task 2.3: Update SourceCitationsProps Interface
**File:** `source-citations.tsx` line ~23

**Changes:**
```tsx
interface SourceCitationsProps {
  context: QueryContext;
  onEntityClick?: (entityId: string) => void;
  onDocumentClick?: (
    documentId: string, 
    chunkContent?: string, 
    chunkIndex?: number,
    startLine?: number,     // ADD
    endLine?: number        // ADD
  ) => void;
  onExploreGraph?: (entityLabels: string[]) => void;
}
```

#### Task 2.4: Update Query Page Navigation Handler
**File:** `app/(dashboard)/query/page.tsx`

**Find and replace:**
```typescript
const handleDocumentClick = (
  docId: string, 
  content?: string, 
  chunkIdx?: number,
  startLine?: number,
  endLine?: number
) => {
  const params = new URLSearchParams();
  if (content) params.set('highlight', content);
  if (startLine !== undefined) params.set('start_line', startLine.toString());
  if (endLine !== undefined) params.set('end_line', endLine.toString());
  
  router.push(`/documents/${docId}${params.toString() ? '?' + params.toString() : ''}`);
};
```

#### Task 2.5: Add Highlighter-Style (Stabilo) CSS
**File:** `app/globals.css`

**Add:**
```css
/* Highlighter pen style for source citations */
mark.highlight-citation {
  background: linear-gradient(104deg, 
    rgba(255, 237, 74, 0.3) 0.9%, 
    rgba(255, 237, 74, 0.7) 2.4%, 
    rgba(255, 237, 74, 0.5) 5.8%, 
    rgba(255, 237, 74, 0.4) 93%, 
    rgba(255, 237, 74, 0.7) 96%, 
    transparent 98%
  );
  background-size: 30% 0.5em;
  background-repeat: no-repeat;
  background-position: 0 88%;
  color: inherit;
  padding: 0.125rem 0;
  border-radius: 0.25rem;
  animation: highlight-fade-in 0.8s ease-out;
}

@keyframes highlight-fade-in {
  from {
    background-size: 0% 0.5em;
  }
  to {
    background-size: 30% 0.5em;
  }
}

/* Dark mode version */
.dark mark.highlight-citation {
  background: linear-gradient(104deg, 
    rgba(253, 224, 71, 0.4) 0.9%, 
    rgba(253, 224, 71, 0.8) 2.4%, 
    rgba(253, 224, 71, 0.6) 5.8%, 
    rgba(253, 224, 71, 0.5) 93%, 
    rgba(253, 224, 71, 0.8) 96%, 
    transparent 98%
  );
}
```

#### Task 2.6: Update ContentRenderer to Highlight Lines
**File:** `components/document/content-renderer.tsx` line ~20

**Changes:**
```tsx
interface ContentRendererProps {
  document: Document;
  highlightText?: string;
  startLine?: number;   // ADD
  endLine?: number;     // ADD
}

export function ContentRenderer({ 
  document, 
  highlightText,
  startLine,
  endLine 
}: ContentRendererProps) {
  const contentRef = useRef<HTMLDivElement>(null);
  
  const renderer = useMemo(() => {
    return getRendererForDocument(document, highlightText, startLine, endLine);
  }, [document, highlightText, startLine, endLine]);

  // Scroll to and highlight the lines when startLine/endLine provided
  useEffect(() => {
    if ((startLine === undefined || !contentRef.current) && !highlightText) return;
    
    const timer = setTimeout(() => {
      const container = contentRef.current;
      if (!container) return;
      
      // If line numbers provided, scroll to line range
      if (startLine !== undefined) {
        const lineElements = container.querySelectorAll('[data-line-number]');
        const targetLine = Array.from(lineElements).find(
          el => parseInt(el.getAttribute('data-line-number') || '0') >= startLine
        );
        if (targetLine) {
          targetLine.scrollIntoView({ behavior: 'smooth', block: 'center' });
        }
      } else if (highlightText) {
        // Fallback to text-based highlight
        const highlightedElements = container.querySelectorAll('mark.highlight-citation');
        if (highlightedElements.length > 0) {
          highlightedElements[0].scrollIntoView({ 
            behavior: 'smooth', 
            block: 'center' 
          });
        }
      }
    }, 100);
    
    return () => clearTimeout(timer);
  }, [highlightText, startLine, endLine]);

  return (
    <div ref={contentRef} className="p-8 max-w-4xl mx-auto">
      <Suspense fallback={<ContentSkeleton />}>
        {renderer}
      </Suspense>
    </div>
  );
}
```

#### Task 2.7: Update getRendererForDocument Function
**File:** `components/document/content-renderer.tsx` line ~50

**Changes:**
```tsx
function getRendererForDocument(
  doc: Document, 
  highlightText?: string,
  startLine?: number,
  endLine?: number
) {
  const mimeType = doc.mime_type?.toLowerCase() || '';
  const fileName = doc.file_name?.toLowerCase() || '';
  let content = doc.content || doc.content_summary || '';

  // Apply highlight to content
  if (startLine !== undefined && endLine !== undefined) {
    content = applyLineHighlight(content, startLine, endLine);
  } else if (highlightText && content) {
    content = applyTextHighlight(content, highlightText);
  }

  // ... rest of function unchanged
}
```

#### Task 2.8: Create applyLineHighlight Helper
**File:** `components/document/content-renderer.tsx` (add after imports)

**Add:**
```tsx
/**
 * Highlight specific line range in content using stabilo highlighter style
 */
function applyLineHighlight(content: string, startLine: number, endLine: number): string {
  const lines = content.split('\n');
  
  return lines.map((line, idx) => {
    const lineNumber = idx + 1;
    if (lineNumber >= startLine && lineNumber <= endLine) {
      return `<mark class="highlight-citation" data-line-number="${lineNumber}">${escapeHtml(line)}</mark>`;
    }
    return `<span data-line-number="${lineNumber}">${escapeHtml(line)}</span>`;
  }).join('\n');
}

function escapeHtml(text: string): string {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}
```

#### Task 2.9: Update Document Detail Page to Pass Line Numbers
**File:** `app/(dashboard)/documents/[id]/page.tsx` line ~46

**Changes:**
```tsx
export default function DocumentViewPage() {
  // ... existing code
  const searchParams = useSearchParams();
  const documentId = params.id as string;
  
  const highlightText = searchParams.get('highlight') || undefined;
  const startLine = searchParams.get('start_line') 
    ? parseInt(searchParams.get('start_line')!) 
    : undefined;
  const endLine = searchParams.get('end_line') 
    ? parseInt(searchParams.get('end_line')!) 
    : undefined;

  // ... rest of component

  return (
    <div className="flex flex-col h-screen overflow-hidden">
      {/* ... header */}
      <div className="flex-1 flex overflow-hidden">
        <div className="hidden lg:flex flex-1 overflow-hidden">
          <div className="flex-1 overflow-auto">
            <ContentRenderer 
              document={document} 
              highlightText={highlightText}
              startLine={startLine}
              endLine={endLine}
            />
          </div>
          {/* ... sidebar */}
        </div>
      </div>
    </div>
  );
}
```

### Phase 3: Sidebar Scroll Fix (10 min)

#### Task 3.1: Fix Sidebar Container Overflow
**File:** `app/(dashboard)/documents/[id]/page.tsx` line ~175

**Changes:**
```tsx
{/* Metadata Sidebar - 35% */}
<div className="w-[35%] shrink-0 overflow-hidden">
  <MetadataSidebar document={document} />
</div>
```

**Add:** `overflow-hidden` to parent container

#### Task 3.2: Verify MetadataSidebar Structure
**File:** `components/document/metadata-sidebar.tsx` line ~20

**Verify present:**
```tsx
<div className="h-full flex flex-col border-l bg-background">
  <div className="sticky top-0 z-10 bg-background border-b p-4 shadow-sm">
    <KeyStats document={document} />
  </div>
  <ScrollArea className="flex-1">
    {/* content */}
  </ScrollArea>
</div>
```

**Status:** ✅ Already correct - just needs parent overflow fix

### Phase 4: Backend Line-Based Chunking (90 min)

#### Task 4.1: Update Backend SourceReference Struct
**File:** `edgequake/crates/edgequake-api/src/handlers/query.rs`

**Find:**
```rust
pub struct SourceReference {
    pub content: String,
    pub document_id: String,
    pub score: f32,
    pub file_path: Option<String>,
}
```

**Replace:**
```rust
pub struct SourceReference {
    pub content: String,
    pub document_id: String,
    pub score: f32,
    pub file_path: Option<String>,
    pub start_line: Option<usize>,
    pub end_line: Option<usize>,
    pub chunk_index: Option<usize>,
}
```

#### Task 4.2: Update SOTA Engine to Populate Line Numbers
**File:** `edgequake/crates/edgequake-query/src/sota_engine.rs`

**Find chunk processing and update to include line info:**
```rust
// When creating SourceReference from ChunkLineage
SourceReference {
    content: chunk.content.clone(),
    document_id: chunk.document_id.clone(),
    score: chunk.score,
    file_path: chunk.file_path.clone(),
    start_line: chunk.start_line,
    end_line: chunk.end_line,
    chunk_index: Some(chunk.index),
}
```

#### Task 4.3: Verify ChunkLineage Has Line Data
**File:** `edgequake/crates/edgequake-storage/src/models/chunk.rs`

**Verify fields exist:**
```rust
pub struct ChunkLineage {
    pub chunk_id: String,
    pub content: String,
    pub start_line: Option<usize>,
    pub end_line: Option<usize>,
    // ...
}
```

**Status:** ✅ Fields already exist per frontend types

#### Task 4.4: Update TextSplitter for Line-Aware Splitting
**File:** `edgequake/crates/edgequake-core/src/chunking/text_splitter.rs`

**Goal:** Modify split logic to:
1. Calculate target chunk size in characters
2. Find nearest newline boundary after target size
3. Track line numbers for each chunk

**Implementation:**
```rust
impl TextSplitter {
    pub fn split_with_lines(&self, text: &str) -> Vec<ChunkWithLines> {
        let lines: Vec<&str> = text.lines().collect();
        let mut chunks = Vec::new();
        let mut current_chunk = String::new();
        let mut current_start_line = 1;
        let mut current_line = 1;
        let mut current_size = 0;
        
        for line in lines {
            let line_len = line.len() + 1; // +1 for newline
            
            // Check if adding this line would exceed chunk size
            if current_size + line_len > self.chunk_size && !current_chunk.is_empty() {
                // Save current chunk
                chunks.push(ChunkWithLines {
                    content: current_chunk.clone(),
                    start_line: current_start_line,
                    end_line: current_line - 1,
                    start_offset: 0, // TODO: track char offsets if needed
                    end_offset: current_chunk.len(),
                });
                
                // Start new chunk with overlap
                let overlap_lines = self.calculate_overlap_lines(&current_chunk);
                current_chunk = overlap_lines;
                current_start_line = current_line - overlap_lines.lines().count();
                current_size = overlap_lines.len();
            }
            
            current_chunk.push_str(line);
            current_chunk.push('\n');
            current_size += line_len;
            current_line += 1;
        }
        
        // Add final chunk
        if !current_chunk.is_empty() {
            chunks.push(ChunkWithLines {
                content: current_chunk,
                start_line: current_start_line,
                end_line: current_line - 1,
                start_offset: 0,
                end_offset: current_chunk.len(),
            });
        }
        
        chunks
    }
    
    fn calculate_overlap_lines(&self, text: &str) -> String {
        let lines: Vec<&str> = text.lines().collect();
        let overlap_chars = (self.chunk_size as f64 * self.overlap) as usize;
        let mut result = String::new();
        let mut current_len = 0;
        
        for line in lines.iter().rev() {
            if current_len + line.len() > overlap_chars {
                break;
            }
            result.insert_str(0, line);
            result.insert(0, '\n');
            current_len += line.len() + 1;
        }
        
        result
    }
}

pub struct ChunkWithLines {
    pub content: String,
    pub start_line: usize,
    pub end_line: usize,
    pub start_offset: usize,
    pub end_offset: usize,
}
```

#### Task 4.5: Wire Line Data Through Pipeline
**File:** `edgequake/crates/edgequake-pipeline/src/pipeline.rs`

**Ensure line data flows from chunking → storage → query**

### Phase 5: E2E Testing (45 min)

#### Task 5.1: Create Visual Verification Test
**File:** `edgequake_webui/e2e/source-citations-overflow-test.spec.ts`

**Content:**
```typescript
import { test, expect } from '@playwright/test';

test.describe('Source Citations Overflow Fixes', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('http://localhost:3000/query');
    
    // Submit query to get citations
    await page.getByPlaceholder('Ask questions about your knowledge graph').fill('What is RepoNavigator?');
    await page.getByRole('button', { name: 'Send' }).click();
    
    // Wait for response and citations
    await page.waitForSelector('text=Source', { timeout: 30000 });
    
    // Expand citations
    await page.getByRole('button', { name: /Source/ }).click();
  });

  test('documents tab chunks should not overflow', async ({ page }) => {
    // Click Documents tab
    await page.getByRole('tab', { name: 'Documents' }).click();
    
    // Get first chunk passage
    const passage = page.locator('.group\\/chunk').first();
    await expect(passage).toBeVisible();
    
    // Check overflow
    const box = await passage.boundingBox();
    const parent = await passage.locator('..').boundingBox();
    
    expect(box?.width).toBeLessThanOrEqual(parent?.width || 0);
    
    // Screenshot
    await page.screenshot({ 
      path: 'test-results/citations-documents-no-overflow.png',
      fullPage: false
    });
  });

  test('knowledge tab topics should not overflow', async ({ page }) => {
    // Click Knowledge tab
    await page.getByRole('tab', { name: 'Knowledge' }).click();
    
    // Check Key Topics section
    const topicsSection = page.locator('text=Key Topics').locator('..');
    await expect(topicsSection).toBeVisible();
    
    // Verify badges wrap properly
    const badges = topicsSection.locator('[role=button]');
    const count = await badges.count();
    expect(count).toBeGreaterThan(0);
    
    // Screenshot
    await page.screenshot({ 
      path: 'test-results/citations-knowledge-no-overflow.png',
      fullPage: false
    });
  });

  test('clicking chunk should navigate with line numbers', async ({ page }) => {
    // Click first chunk passage
    const firstChunk = page.locator('.group\\/chunk').first();
    await firstChunk.click();
    
    // Verify URL contains start_line and end_line
    await page.waitForURL(/documents\/[^/]+\?.*start_line=\d+/);
    const url = page.url();
    expect(url).toContain('start_line=');
    expect(url).toContain('end_line=');
    
    // Verify highlight is visible
    await page.waitForSelector('mark.highlight-citation', { timeout: 5000 });
    const highlight = page.locator('mark.highlight-citation').first();
    await expect(highlight).toBeVisible();
    
    // Screenshot
    await page.screenshot({ 
      path: 'test-results/document-line-highlight.png',
      fullPage: true
    });
  });
});

test.describe('Document Detail Page', () => {
  test('right sidebar should be scrollable', async ({ page }) => {
    // Navigate to a document
    await page.goto('http://localhost:3000/query');
    await page.getByPlaceholder('Ask questions about your knowledge graph').fill('test');
    await page.getByRole('button', { name: 'Send' }).click();
    
    // Wait and click source citation
    await page.waitForSelector('text=Source', { timeout: 30000 });
    await page.getByRole('button', { name: /Source/ }).click();
    await page.getByRole('tab', { name: 'Documents' }).click();
    
    const docLink = page.locator('button[title^="Open:"]').first();
    await docLink.click();
    
    // Wait for document detail page
    await page.waitForURL(/documents\/[^/]+/);
    
    // Find sidebar (desktop view)
    const sidebar = page.locator('.w-\\[35\\%\\]').first();
    await expect(sidebar).toBeVisible();
    
    // Check if scrollable (has overflow content)
    const scrollHeight = await sidebar.evaluate(el => el.scrollHeight);
    const clientHeight = await sidebar.evaluate(el => el.clientHeight);
    
    // If content overflows, verify we can scroll
    if (scrollHeight > clientHeight) {
      await sidebar.evaluate(el => el.scrollTop = 50);
      const scrollTop = await sidebar.evaluate(el => el.scrollTop);
      expect(scrollTop).toBeGreaterThan(0);
    }
    
    // Screenshot
    await page.screenshot({ 
      path: 'test-results/document-sidebar-scrollable.png',
      fullPage: true
    });
  });
});
```

#### Task 5.2: Update Existing E2E Tests
**File:** `edgequake_webui/e2e/source-citations-visual.spec.ts`

**Add test cases for line numbers:**
```typescript
test('source citations display line ranges', async ({ page }) => {
  // ... existing setup
  
  // Check for line range display
  const lineRange = page.locator('text=/Lines \\d+-\\d+/').first();
  await expect(lineRange).toBeVisible();
});
```

## Success Criteria

### Phase 1: Overflow Fixes ✅
- [ ] Chunk passages constrained horizontally (no horizontal scroll)
- [ ] Document titles truncate with ellipsis
- [ ] Key Topics display all badges with proper wrapping
- [ ] Connections list shows all items without clipping

### Phase 2: Line Navigation ✅
- [ ] QueryContext includes `start_line`, `end_line`, `chunk_index`
- [ ] Clicking chunk navigates to document with line range in URL
- [ ] Document page highlights lines with stabilo/highlighter effect
- [ ] Line range displayed below each passage (e.g., "Lines 42-58")

### Phase 3: Sidebar Scroll ✅
- [ ] Metadata sidebar scrollable when content exceeds viewport
- [ ] Stats section remains sticky at top while scrolling

### Phase 4: Backend Line Data ✅
- [ ] Backend API returns line numbers in SourceReference
- [ ] TextSplitter splits on line boundaries
- [ ] Line tracking accurate across pipeline

### Phase 5: E2E Tests ✅
- [ ] All E2E tests pass
- [ ] Visual verification screenshots generated
- [ ] No overflow in any component

## Timeline

- **Phase 1 (Overflow):** 30 min
- **Phase 2 (Navigation):** 60 min
- **Phase 3 (Sidebar):** 10 min
- **Phase 4 (Backend):** 90 min (optional - can defer)
- **Phase 5 (Testing):** 45 min

**Total:** ~3.5 hours (2 hours for frontend-only, 90min for backend)

## Dependencies

- Backend line data (Phase 4) optional - can mock with default values
- Playwright tests require dev stack running (`make dev`)

## Rollout Strategy

1. **Immediate:** Phase 1 (overflow fixes) - pure CSS, zero risk
2. **Same session:** Phase 2 (navigation) + Phase 3 (sidebar) - frontend only
3. **Next session:** Phase 4 (backend chunking) - requires Rust changes
4. **Continuous:** Phase 5 (E2E tests) - run after each phase

## Notes

- Line highlighting uses "stabilo highlighter" effect per user request
- All changes maintain backward compatibility
- Backend changes optional - frontend can display "Line N/A" if missing
