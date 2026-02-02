-- Fix missing document for PDF "001-BEYONG-TRANFORMER-OUTLINE-V1_1.docx.pdf"
-- This PDF was processed but document was never created in documents table

BEGIN;

-- 1. Generate a new document UUID
-- 2. Create the document record
-- 3. Link PDF to document
-- 4. Update metadata

DO $$
DECLARE
    new_doc_id UUID := gen_random_uuid();
    pdf_uuid UUID := '30f2f58d-0892-4f1c-8da1-0f9abde1bbf6';
    ws_id UUID := 'dfb92a34-bff4-4b60-adb1-b5cf2b15acc5';
    tenant_uuid UUID := '425ac304-3871-472f-978d-582674a1822f';
    pdf_content TEXT;
    pdf_filename TEXT;
BEGIN
    -- Get markdown content and filename from PDF
    SELECT markdown_content, filename 
    INTO pdf_content, pdf_filename
    FROM pdf_documents 
    WHERE pdf_id = pdf_uuid;

    -- Create document record with content from PDF
    INSERT INTO documents (
        id,
        tenant_id,
        workspace_id,
        title,
        content,
        status,
        chunk_count,
        entity_count,
        relationship_count,
        created_at,
        updated_at
    ) VALUES (
        new_doc_id,
        tenant_uuid,
        ws_id,
        pdf_filename,
        pdf_content,
        'indexed',  -- Valid status: indexed (not completed)
        3,
        15,
        9,
        '2026-02-01 23:55:22.277658+00',
        '2026-02-01 23:55:45.328270+00'
    );

    -- Link PDF to document
    UPDATE pdf_documents
    SET document_id = new_doc_id
    WHERE pdf_id = pdf_uuid;

    -- Log the fix
    RAISE NOTICE 'Created document % for PDF %', new_doc_id, pdf_uuid;
    RAISE NOTICE 'Document should now appear in UI at: http://localhost:3000/documents?workspace=default-workspace';
END $$;

COMMIT;

-- Verify the fix
SELECT 
    'PDF Document' as source,
    pdf_id,
    filename,
    document_id,
    processing_status
FROM pdf_documents 
WHERE pdf_id = '30f2f58d-0892-4f1c-8da1-0f9abde1bbf6';

SELECT 
    'Documents Table' as source,
    d.id as document_id,
    d.workspace_id,
    d.status,
    w.name as workspace_name,
    t.name as tenant_name
FROM documents d
JOIN pdf_documents pdf ON pdf.document_id = d.id
JOIN workspaces w ON d.workspace_id = w.workspace_id
JOIN tenants t ON w.tenant_id = t.tenant_id
WHERE pdf.pdf_id = '30f2f58d-0892-4f1c-8da1-0f9abde1bbf6';
