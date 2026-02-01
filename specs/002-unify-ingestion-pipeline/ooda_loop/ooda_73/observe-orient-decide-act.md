# OODA-73: Component Decomposition

**Date**: 2026-02-01
**Focus**: UI Component Structure

## OBSERVE

### Mission Requirements (Re-read from specs/002-unify-ingestion-pipeline.md)
- Maintainable component architecture
- Reusable UI elements

### Document Components Structure

```
components/
└── documents/
    ├── document-manager.tsx       # Main orchestrator
    ├── document-list.tsx          # Table display
    ├── document-preview-panel.tsx # Slide-out panel
    ├── document-upload.tsx        # Upload dropzone
    ├── pdf-viewer.tsx             # PDF rendering
    ├── markdown-viewer.tsx        # Markdown display
    └── side-by-side-viewer.tsx    # Split view
```

## ORIENT

### Component Responsibilities

| Component | Single Responsibility |
|-----------|----------------------|
| DocumentManager | State & data orchestration |
| DocumentList | Table rendering & interactions |
| DocumentPreviewPanel | Side panel with preview |
| DocumentUpload | File handling & upload |
| PDFViewer | PDF page rendering |
| MarkdownViewer | Markdown to HTML |
| SideBySideViewer | Layout for PDF + Markdown |

### Props vs State

| Component | Props | Local State | Query State |
|-----------|-------|-------------|-------------|
| DocumentManager | minimal | selection, filters | documents |
| DocumentList | documents, handlers | none | none |
| PDFViewer | file | scale, page | none |

## DECIDE

**Decision**: Component structure follows best practices

Principles applied:
1. Single responsibility
2. Props down, events up
3. Co-location of related components

## ACT

### DocumentManager as Container

```typescript
// Orchestrates all document state and passes down
const DocumentManager = () => {
  const { data: documents } = useDocuments();
  const [selectedDoc, setSelectedDoc] = useState<Document | null>(null);
  
  return (
    <div className="flex">
      <div className="flex-1">
        <DocumentUpload onUpload={handleUpload} />
        <DocumentList 
          documents={documents}
          onSelect={setSelectedDoc}
          onDoubleClick={handleNavigate}
        />
      </div>
      <DocumentPreviewPanel
        document={selectedDoc}
        onClose={() => setSelectedDoc(null)}
        onViewDetails={handleViewDetails}
      />
    </div>
  );
};
```

### Pure Presentational Component

```typescript
// DocumentList: No hooks, just props
interface DocumentListProps {
  documents: Document[];
  selectedId?: string;
  onSelect: (doc: Document) => void;
  onDoubleClick: (doc: Document) => void;
}

const DocumentList = ({ 
  documents, 
  selectedId, 
  onSelect, 
  onDoubleClick 
}: DocumentListProps) => (
  <Table>
    {/* Pure rendering based on props */}
  </Table>
);
```

**Status**: ✅ VERIFIED - Component architecture correct
