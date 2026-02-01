# OODA-18: Orient - Analysis & First Principles

## Date: 2026-02-01

## Mission Re-Read ✓

Objectives confirmed from `specs/002-unify-ingestion-pipeline.md`:

- PDF/Markdown viewing with best-in-class library
- Side-by-side view
- Excellent UX (scrolling, borders, margins)
- Multi-tenancy compliance
- OpenAPI documentation
- Comprehensive testing

---

## First Principles Analysis

### 1. What problem are we solving?

**User Need**: View original PDF alongside extracted Markdown to:

- Verify extraction quality
- Cross-reference content
- Download original document
- Copy extracted text

### 2. What is the simplest solution?

The current implementation already provides:

- PDF rendering (react-pdf)
- Markdown display (StreamingMarkdownRenderer)
- Side-by-side layout (SideBySideViewer)
- View mode toggle

**Principle: Don't fix what isn't broken.**

### 3. What can we improve?

| Area            | Current     | Ideal               | Gap            |
| --------------- | ----------- | ------------------- | -------------- |
| Scroll behavior | Independent | Smooth, contained   | Minor CSS      |
| Borders/margins | Basic       | Consistent, minimal | Refinement     |
| Loading states  | Skeleton    | Progressive render  | React Suspense |
| Error handling  | Basic       | Retry + fallback    | Complete       |
| Testing         | Unknown     | E2E + Unit          | Create tests   |

---

## Risk Assessment

### Low Risk (Proceed)

- CSS refinements for scroll/borders
- Test creation for existing components
- Documentation updates

### Medium Risk (Careful)

- Scroll synchronization (complex, may not add value)
- Adding more PDF features (scope creep)

### High Risk (Avoid)

- Replacing react-pdf with another library
- Major refactoring of working components
- Adding features not in mission scope

---

## Gap Analysis vs Mission

| Mission Objective               | Current Status                       | Action Needed |
| ------------------------------- | ------------------------------------ | ------------- |
| PDF viewer with best JS library | ✅ react-pdf (2.8M weekly downloads) | None          |
| Markdown viewer                 | ✅ StreamingMarkdownRenderer         | None          |
| Side-by-side view               | ✅ SideBySideViewer                  | None          |
| Scrolling UX                    | ⚠️ Works but can be refined          | Minor CSS     |
| Border/margin UX                | ⚠️ Basic styling                     | Minor CSS     |
| Multi-tenancy                   | ✅ Backend enforces isolation        | None          |
| OpenAPI docs                    | ✅ utoipa annotations                | None          |
| Testing                         | ❌ No E2E tests for viewer           | Create tests  |

---

## Component Quality Assessment

### PDFViewer

**Strengths:**

- Dynamic import for SSR compatibility
- Configurable zoom/pagination
- Error boundary with retry
- Responsive width handling

**Improvements Possible:**

- Add keyboard navigation
- Add scroll-to-page feature
- Improve mobile touch handling

### MarkdownViewer

**Strengths:**

- Reuses StreamingMarkdownRenderer (DRY)
- Copy to clipboard
- Typography optimized

**Improvements Possible:**

- Add heading-based navigation
- Add search within content

### SideBySideViewer

**Strengths:**

- Resizable divider
- Three view modes
- Smooth resize animation

**Improvements Possible:**

- Keyboard resize support
- Remember panel size preference
- Better touch support

---

## Technology Stack Validation

```
Frontend:
├── React 19 ✓ (latest)
├── Next.js 15 ✓ (App Router)
├── react-pdf 10.x ✓ (actively maintained)
├── pdfjs-dist 4.x ✓ (Mozilla's library)
├── Tailwind CSS ✓ (styling)
└── Radix UI ✓ (accessible components)

Backend:
├── Rust axum ✓
├── utoipa ✓ (OpenAPI)
└── PostgreSQL ✓ (PDF storage)
```

**Verdict: Stack is modern and appropriate. No changes needed.**

---

## Decision Matrix

| Option                 | Effort | Value  | Priority |
| ---------------------- | ------ | ------ | -------- |
| Create E2E tests       | Medium | High   | 1        |
| Refine scroll UX       | Low    | Medium | 2        |
| Refine borders/margins | Low    | Medium | 3        |
| Add keyboard shortcuts | Medium | Low    | 4        |
| Scroll sync            | High   | Low    | 5        |

---

## Recommendation

Focus on:

1. **Testing**: Create comprehensive E2E tests for document viewer
2. **UX Polish**: Minor CSS refinements for scroll/borders
3. **Documentation**: Verify OpenAPI is complete

Avoid:

- Major feature additions
- Library replacements
- Scope creep beyond mission
