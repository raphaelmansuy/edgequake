# OODA-91: PDF Markdown Display Fix

## Task Summary
Fixed the PDF split view where markdown panel was showing blank instead of extracted content, plus fixed two React errors that appeared during testing.

## Actions
- Investigated document structure and content rendering flow
- Identified root cause: PDF markdown stored in `pdf_documents` table, not in KV `{document_id}-content` key
- Added `getPdfContent` import and query to page.tsx
- Created `documentWithContent` memo to merge PDF markdown into document
- Updated 3 ContentRenderer usages to use merged document
- Added loading spinner for PDF content fetching
- Fixed React hooks order violation by moving hooks before early returns
- Added react-pdf CSS imports for TextLayer and AnnotationLayer
- Verified TypeScript compilation passes
- Started dev stack and confirmed all services running

## Decisions
- Used separate React Query for PDF content (staleTime: 60s) rather than modifying getDocument API
- Merged markdown via useMemo to avoid prop drilling changes
- Added loading state UI for better UX during content fetch
- Moved all hooks before conditional returns to comply with React Rules of Hooks
- Imported both TextLayer and AnnotationLayer CSS for full PDF feature support

## Next Steps
- Browser verification complete: services running on localhost:3000 and localhost:8080
- PDF markdown now displays correctly in split view
- All React warnings resolved

## Lessons/Insights
- PDF documents have dual storage: main document record + separate pdf_documents table for extracted content
- Frontend must explicitly fetch PDF-specific content via dedicated API endpoint
- React hooks must be called in the same order every render - place before any conditional returns
- react-pdf requires explicit CSS imports for TextLayer and AnnotationLayer features

## Commits
- `a0824fa9` - Initial PDF markdown display fix
- `39cefe22` - React hooks order violation fix
- `99e02761` - react-pdf CSS imports for TextLayer/AnnotationLayer
