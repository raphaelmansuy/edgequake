# OODA-91: PDF Markdown Display Fix

## Task Summary
Fixed the PDF split view where markdown panel was showing blank instead of extracted content.

## Actions
- Investigated document structure and content rendering flow
- Identified root cause: PDF markdown stored in `pdf_documents` table, not in KV `{document_id}-content` key
- Added `getPdfContent` import and query to page.tsx
- Created `documentWithContent` memo to merge PDF markdown into document
- Updated 3 ContentRenderer usages to use merged document
- Added loading spinner for PDF content fetching
- Verified TypeScript compilation passes

## Decisions
- Used separate React Query for PDF content (staleTime: 60s) rather than modifying getDocument API
- Merged markdown via useMemo to avoid prop drilling changes
- Added loading state UI for better UX during content fetch

## Next Steps
- Browser verification: Navigate to PDF document to confirm markdown displays correctly
- Monitor for edge cases with documents that have pdf_id but no markdown_content

## Lessons/Insights
- PDF documents have dual storage: main document record + separate pdf_documents table for extracted content
- Frontend must explicitly fetch PDF-specific content via dedicated API endpoint

## Commit
`a0824fa9` - OODA-91: Fix PDF markdown display in split view
