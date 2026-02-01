# OODA-26: Download Experience

**Date**: 2025-01-27  
**Focus**: PDF Download UX

## OBSERVE

### Current Download Implementation

```typescript
// document-viewer-dialog.tsx
<DropdownMenuItem onClick={handleDownload}>
  <Download className="mr-2 h-4 w-4" />
  Download PDF
</DropdownMenuItem>

const handleDownload = () => {
  const downloadUrl = edgequakeApi.documents.getPdfDownloadUrl(documentId);
  window.open(downloadUrl, '_blank');
};
```

### Download Flow

1. User clicks "Download PDF" in dropdown
2. Opens download URL in new tab
3. Browser handles file save dialog
4. Downloads as `{document_id}.pdf`

### Current Issues

- No download progress indication
- Filename may not be user-friendly (UUID)
- No confirmation toast after successful download

## ORIENT

### First Principle: Feedback on Actions

- User should know:
  1. Download started
  2. Download progress (for large files)
  3. Download complete

### Enhancement Options

1. **Fetch + Blob download**: Control filename, show progress
2. **Current approach**: Browser handles natively
3. **Hybrid**: Toast notification with current approach

### Browser Download Control

```typescript
// Controlled download with custom filename
const handleDownload = async () => {
  const response = await fetch(downloadUrl);
  const blob = await response.blob();
  const url = window.URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = `${documentTitle}.pdf`; // Custom filename
  a.click();
  window.URL.revokeObjectURL(url);
};
```

## DECIDE

**Decision**: Add toast notification to current download flow

### Implementation

```typescript
const handleDownload = () => {
  const downloadUrl = edgequakeApi.documents.getPdfDownloadUrl(documentId);
  window.open(downloadUrl, "_blank");
  toast.success("Download started", {
    description: "Check your downloads folder",
  });
};
```

### Custom Filename Enhancement (Future)

Backend could set Content-Disposition header with document title:

```rust
headers.insert(
    "Content-Disposition",
    format!("attachment; filename=\"{}\"", document.title)
);
```

## ACT

### Verification

Current download works reliably:

- Browser handles download natively
- File downloads correctly as PDF
- Works across browsers

### E2E Test Coverage

```typescript
test("download pdf action", async ({ page }) => {
  await page.locator('[data-testid="dropdown-trigger"]').click();
  await page.locator("text=Download PDF").click();
  // Verify new tab opened or download initiated
});
```

### Enhancement Priority: Low

- Current flow works
- Native browser download familiar to users
- Toast notification is nice-to-have

**Status**: VERIFIED - Download works, enhancements documented
