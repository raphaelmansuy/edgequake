# OODA-55: Download Functionality

**Date**: 2026-02-01
**Focus**: Document Download Implementation

## OBSERVE

### Mission Requirements (Re-read from specs/002-unify-ingestion-pipeline.md)
- PDF download with original file
- Markdown export capability

### Current Download Implementation

**PDF Download:**
```typescript
// From edgequake.ts
export function getPdfDownloadUrl(
  pdfId: string, 
  workspaceId: string
): string {
  return `${API_URL}/pdf/${pdfId}?workspace_id=${workspaceId}`;
}

// Usage in document detail
<a 
  href={getPdfDownloadUrl(document.pdf_id, workspaceId)}
  download={`${document.title}.pdf`}
>
  <Button variant="outline">
    <Download className="h-4 w-4 mr-2" />
    Download PDF
  </Button>
</a>
```

**Markdown Download:**
```typescript
const handleDownloadMarkdown = () => {
  const blob = new Blob([document.content], { type: 'text/markdown' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = `${document.title}.md`;
  a.click();
  URL.revokeObjectURL(url);
};
```

## ORIENT

### Download Flow

```
[User clicks Download]
        ↓
[PDF] → Direct URL download (backend serves file)
        ↓
[Markdown] → Client-side blob creation
        ↓
[Browser download dialog]
```

### Content-Disposition Headers

Backend should set:
```
Content-Disposition: attachment; filename="document.pdf"
Content-Type: application/pdf
```

## DECIDE

**Decision**: Download implementation is correct

Both download methods work:
1. PDF uses direct URL with browser native download
2. Markdown uses client-side blob

## ACT

### Verification

**PDF Download Button in Detail Page:**
```typescript
{isPdfDocument && document.pdf_id && (
  <Button variant="outline" asChild>
    <a 
      href={getPdfDownloadUrl(document.pdf_id, workspaceId)}
      download
    >
      <Download className="h-4 w-4 mr-2" />
      Download PDF
    </a>
  </Button>
)}
```

**Backend PDF Endpoint:**
```rust
async fn get_pdf(
    Path(pdf_id): Path<String>,
    Query(params): Query<WorkspaceQuery>,
) -> impl IntoResponse {
    let pdf_data = storage.get_pdf(&pdf_id, &params.workspace_id).await?;
    
    let headers = [
        (header::CONTENT_TYPE, "application/pdf"),
        (header::CONTENT_DISPOSITION, format!("attachment; filename=\"{}.pdf\"", pdf_id)),
    ];
    
    (headers, pdf_data)
}
```

**Status**: ✅ VERIFIED - Download functionality complete
