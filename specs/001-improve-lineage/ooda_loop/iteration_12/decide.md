# Decision - Iteration 12

## Changes to Make
1. Create `enhanced-metadata.tsx` component in `edgequake_webui/src/components/document/`
2. Add `EnhancedMetadata` section to `MetadataSidebar` as "Extended Metadata"
3. Use `useDocumentMetadata` hook (from OODA-11)
4. Filter displayed fields to avoid duplication with existing sidebar sections

## Priority
1. Create EnhancedMetadata component — HIGH impact, LOW effort
2. Integrate into MetadataSidebar — HIGH impact, LOW effort

## Expected Outcome
Users see all KV-stored metadata in the document detail sidebar, including fields from pipeline ingestion (OODA-04/05/06) that weren't previously visible in the UI.
