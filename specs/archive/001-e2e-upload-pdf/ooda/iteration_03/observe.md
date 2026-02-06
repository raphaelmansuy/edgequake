# OODA Iteration 03: Cancel Document Functionality Fix

## Observation Date

2026-02-06

## Mission Extension

User requested: "Ensure we can cancel a document with a processing state"

## Problem Discovered

**Bug**: The "Cancel Extraction" button was NOT appearing in the document action dropdown menu for documents with status `pending` or `processing`.

### Evidence Gathering

1. **Frontend Code Review** ([document-manager.tsx](../../../../../../edgequake_webui/src/components/documents/document-manager.tsx#L1598-L1606)):

   ```tsx
   {
     (doc.status === "pending" || doc.status === "processing") &&
       doc.track_id && (
         <DropdownMenuItem onClick={() => cancelMutation.mutate(doc.track_id!)}>
           <StopCircle className="h-4 w-4 mr-2" />
           {t("documents.actions.cancel", "Cancel Extraction")}
         </DropdownMenuItem>
       );
   }
   ```

   The condition requires `doc.track_id` to be truthy.

2. **API Response Investigation**:
   - Documents returned by `/api/v1/documents` did NOT include `track_id` field
   - The `DocumentSummary` struct has `track_id: Option<String>` with `#[serde(skip_serializing_if = "Option::is_none")]`
   - Track_id was always `None` because it wasn't being stored in document metadata

3. **Backend Code Review** ([processor.rs](../../../../../../edgequake/crates/edgequake-api/src/processor.rs)):
   - Document metadata is created in `process_text_insert` function
   - For PDF documents, metadata JSON was created WITHOUT `track_id`
   - The `ensure_document_source_type` function was NOT accepting or storing `track_id`

4. **E2E Test with Playwright**:
   - Navigated to documents page
   - Found document with status "Converting PDF" at 94%
   - Clicked action menu dropdown
   - Menu showed: Copy ID, View PDF, Reprocess, Delete
   - **MISSING**: Cancel Extraction option

## Root Cause

The `track_id` field was NOT being stored in the document metadata when PDF processing tasks were created. This caused:

1. Document metadata has no track_id
2. API returns documents with `track_id: null` (omitted due to skip_serializing_if)
3. Frontend condition `doc.track_id && ...` evaluates to false
4. Cancel button not rendered

## Files Affected

- `/edgequake/crates/edgequake-api/src/processor.rs` - Missing track_id in metadata
