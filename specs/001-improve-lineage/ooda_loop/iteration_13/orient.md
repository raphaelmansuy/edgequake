# Analysis - Iteration 13

## Design Decisions
1. New component (DocumentHierarchyTree) separate from existing LineageTree to maintain SRP
2. Uses useDocumentLineage hook (graph endpoint) which returns chunks + entities
3. Entity-to-chunk reverse mapping built via useMemo for O(1) lookup per chunk
4. Collapsible tree nodes with depth-based indentation via paddingLeft style
5. Shows chunk positions (start_line–end_line) and token counts
6. Entity nodes show type badges and extraction counts for deduplication visibility
