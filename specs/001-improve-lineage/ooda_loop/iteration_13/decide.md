# Decision - Iteration 13

## Changes to Make
1. Create `document-hierarchy-tree.tsx` with Document root → Chunk nodes → Entity leaves
2. Integrate into MetadataSidebar as "Data Hierarchy" section with GitBranch icon
3. Tree supports collapsible nodes with chevron toggles and depth-based indentation

## Expected Outcome
Users see the actual data structure: which chunks were created and which entities were extracted from each chunk. This enables source traceability (F8).
