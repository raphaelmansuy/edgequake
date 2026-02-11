# Document Upload Quick Reference

> **Choose the Right Endpoint for Your Upload Method**

EdgeQuake provides multiple ways to ingest documents. This guide helps you choose the correct endpoint and format.

---

## Quick Decision Tree

```
Do you have...
├─ Raw text/JSON content?
│  └─ Use: POST /api/v1/documents (JSON)
│
└─ Files (PDF, TXT, MD, etc.)?
   ├─ Single file?
   │  └─ Use: POST /api/v1/documents/upload
   │
   └─ Multiple files?
      └─ Use: POST /api/v1/documents/upload/batch
```

---

## Method 1: Text/JSON Upload

**Endpoint**: `POST /api/v1/documents`  
**Content-Type**: `application/json`  
**Use When**: You have text content to ingest (programmatic upload, API integration)

### Example: Basic Text Upload

```bash
curl -X POST http://localhost:8080/api/v1/documents \
  -H "Content-Type: application/json" \
  -d '{
    "content": "Marie Curie was a pioneering physicist...",
    "title": "Marie Curie Biography"
  }'
```

### Example: With Metadata

```bash
curl -X POST http://localhost:8080/api/v1/documents \
  -H "Content-Type: application/json" \
  -H "X-Workspace-ID: workspace-uuid" \
  -d '{
    "content": "Your document text here...",
    "title": "Document Title",
    "metadata": {
      "source": "wikipedia",
      "author": "John Doe",
      "category": "science"
    },
    "enable_gleaning": true,
    "max_gleaning": 2
  }'
```

### Request Body Schema

```typescript
{
  content: string;              // Required: Document text
  title?: string;               // Optional: Document title
  metadata?: object;            // Optional: Custom metadata
  async_processing?: boolean;   // Optional: Process async (default: false)
  track_id?: string;            // Optional: Custom tracking ID
  enable_gleaning?: boolean;    // Optional: Multi-pass extraction (default: true)
  max_gleaning?: number;        // Optional: Max gleaning passes (default: 1)
  use_llm_summarization?: boolean; // Optional: LLM-powered descriptions (default: true)
}
```

---

## Method 2: Single File Upload

**Endpoint**: `POST /api/v1/documents/upload`  
**Content-Type**: `multipart/form-data`  
**Use When**: Uploading files from disk (PDF, TXT, MD, JSON)

### Example: PDF Upload

```bash
curl -X POST http://localhost:8080/api/v1/documents/upload \
  -F "file=@research_paper.pdf" \
  -F "title=My Research Paper"
```

### Example: With Configuration

```bash
curl -X POST http://localhost:8080/api/v1/documents/upload \
  -F "file=@document.pdf" \
  -F "title=Financial Report" \
  -F 'metadata={"category": "finance", "year": 2024}' \
  -F 'config={"enhance_tables": true, "mode": "Hybrid"}'
```

### Supported File Types

| Extension | MIME Type          | Max Size | Notes                     |
| --------- | ------------------ | -------- | ------------------------- |
| `.pdf`    | `application/pdf`  | 50 MB    | Supports vision/hybrid mode |
| `.txt`    | `text/plain`       | 10 MB    | Plain text               |
| `.md`     | `text/markdown`    | 10 MB    | Markdown formatting      |
| `.json`   | `application/json` | 10 MB    | Structured data          |

### Form Fields

| Field      | Type   | Required | Description                              |
| ---------- | ------ | -------- | ---------------------------------------- |
| `file`     | File   | Yes      | The file to upload                       |
| `title`    | String | No       | Custom title (defaults to filename)      |
| `metadata` | JSON   | No       | Custom metadata object                   |
| `config`   | JSON   | No       | PDF processing configuration             |

---

## Method 3: Batch File Upload

**Endpoint**: `POST /api/v1/documents/upload/batch`  
**Content-Type**: `multipart/form-data`  
**Use When**: Uploading multiple files at once

### Example: Multiple Files

```bash
curl -X POST http://localhost:8080/api/v1/documents/upload/batch \
  -F "files=@doc1.pdf" \
  -F "files=@doc2.txt" \
  -F "files=@doc3.md"
```

### Response Format

```json
{
  "results": [
    {
      "filename": "doc1.pdf",
      "document_id": "doc-uuid-1",
      "status": "success",
      "chunk_count": 15
    },
    {
      "filename": "doc2.txt",
      "status": "duplicate",
      "duplicate_of": "doc-uuid-2"
    },
    {
      "filename": "doc3.md",
      "status": "failed",
      "error": "File too large"
    }
  ],
  "processed": 2,
  "duplicates": 1,
  "failed": 0
}
```

---

## Method 4: Directory Scan

**Endpoint**: `POST /api/v1/documents/scan`  
**Content-Type**: `application/json`  
**Use When**: Bulk uploading from a server directory

### Example: Recursive Scan

```bash
curl -X POST http://localhost:8080/api/v1/documents/scan \
  -H "Content-Type: application/json" \
  -d '{
    "path": "/data/documents",
    "recursive": true,
    "extensions": [".pdf", ".txt", ".md"],
    "max_files": 1000
  }'
```

### Request Schema

```typescript
{
  path: string;              // Required: Directory path
  recursive?: boolean;       // Optional: Scan subdirectories (default: true)
  extensions?: string[];     // Optional: File extensions to include
  max_files?: number;        // Optional: Max files to process (default: 1000)
}
```

---

## Common Errors and Fixes

### Error: "Expected request with `Content-Type: application/json`"

**Cause**: Using `-F` (multipart) with `/api/v1/documents`

**Fix**: Use `/api/v1/documents/upload` for file uploads

```bash
# ❌ WRONG
curl -X POST http://localhost:8080/api/v1/documents \
  -F "file=@doc.pdf"

# ✅ CORRECT
curl -X POST http://localhost:8080/api/v1/documents/upload \
  -F "file=@doc.pdf"
```

---

### Error: "Failed to parse the request body as JSON"

**Cause**: Using `-F` with a JSON endpoint or missing quotes

**Fix**: Use `-d` with properly formatted JSON

```bash
# ❌ WRONG
curl -X POST http://localhost:8080/api/v1/documents \
  -F "content=text here"

# ✅ CORRECT
curl -X POST http://localhost:8080/api/v1/documents \
  -H "Content-Type: application/json" \
  -d '{"content": "text here"}'
```

---

### Error: "missing field `content`"

**Cause**: JSON upload missing required `content` field

**Fix**: Include `content` in request body

```bash
# ❌ WRONG
curl -X POST http://localhost:8080/api/v1/documents \
  -H "Content-Type: application/json" \
  -d '{"title": "My Doc"}'

# ✅ CORRECT
curl -X POST http://localhost:8080/api/v1/documents \
  -H "Content-Type: application/json" \
  -d '{"content": "Document text...", "title": "My Doc"}'
```

---

## API Endpoint Summary

| Endpoint                          | Method | Content-Type               | Purpose                    |
| --------------------------------- | ------ | -------------------------- | -------------------------- |
| `/api/v1/documents`               | POST   | `application/json`         | Upload text/JSON content   |
| `/api/v1/documents/upload`        | POST   | `multipart/form-data`      | Upload single file         |
| `/api/v1/documents/upload/batch`  | POST   | `multipart/form-data`      | Upload multiple files      |
| `/api/v1/documents/scan`          | POST   | `application/json`         | Scan directory for files   |
| `/api/v1/documents`               | GET    | N/A                        | List all documents         |
| `/api/v1/documents/{id}`          | GET    | N/A                        | Get document details       |
| `/api/v1/documents/{id}`          | DELETE | N/A                        | Delete document            |

---

## Best Practices

1. **Use the JSON endpoint** (`/documents`) for programmatic text ingestion
2. **Use the upload endpoint** (`/documents/upload`) for file uploads from disk
3. **Use batch upload** for multiple files to reduce overhead
4. **Use directory scan** for server-side bulk ingestion
5. **Always specify Content-Type** header explicitly to avoid confusion
6. **Include metadata** for better document organization and filtering

---

## Next Steps

- **Full API Reference**: [REST API Documentation](rest-api.md)
- **Tutorial**: [Document Ingestion Deep-Dive](../tutorials/document-ingestion.md)
- **Troubleshooting**: [Common Issues](../troubleshooting/common-issues.md#1-document-upload-errors)
- **OpenAPI Spec**: Available at `/swagger-ui` when server is running
