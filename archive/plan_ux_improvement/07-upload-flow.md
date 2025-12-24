# UX/UI Improvement: Upload Flow

## Current State Analysis

### Upload Flow Steps

1. User clicks/drops files on upload zone
2. Files are uploaded to backend
3. Batch progress card appears
4. Backend processes documents (entity extraction)
5. Progress card updates with status
6. Card auto-dismisses on completion
7. Document table updates with new entries

### Positive Observations

- Drag & drop is intuitive
- Progress tracking with batch card
- File list shows pending uploads
- Toast confirms upload success

---

## UX Issues Identified

### Critical

1. **Upload Zone Click Target**

   - **Issue**: SVG icon intercepted clicks (discovered during testing)
   - **Status**: May still occur under certain conditions
   - **Recommendation**:
     - Use `pointer-events: none` on decorative elements
     - Ensure entire zone is clickable

2. **Large File Handling**
   - **Issue**: No indication of max file size
   - **Impact**: Users may try to upload large files and fail
   - **Recommendation**:
     - Show max file size in UI
     - Client-side validation before upload
     - Clear error message if too large

### High Priority

3. **Upload Progress Granularity**

   - **Issue**: Progress shows 0% → 100%, no intermediate states
   - **Impact**: Long uploads appear stuck
   - **Recommendation**:
     - Show upload progress (bytes transferred)
     - Show processing progress separately
     - Indicate current step (uploading → processing → extracting → done)

4. **Multiple File Upload**

   - **Issue**: Batch upload works but file count in card unclear
   - **Impact**: Users unsure how many files are queued
   - **Recommendation**:
     - Show "3 of 5 files complete"
     - Individual file progress
     - Allow canceling individual files

5. **Error Recovery**

   - **Issue**: If upload fails, unclear how to retry
   - **Impact**: Users must start over
   - **Recommendation**:
     - "Retry" button per failed file
     - "Retry All Failed" bulk action
     - Show specific error per file

6. **Processing Time Estimation**
   - **Issue**: No ETA for processing completion
   - **Impact**: Users don't know how long to wait
   - **Recommendation**:
     - Estimate based on file size/content
     - Show "~2 min remaining"
     - Or at least "This may take a few minutes"

### Medium Priority

7. **File Validation Feedback**

   - **Issue**: Invalid file types silently ignored or delayed error
   - **Impact**: Confusing rejection experience
   - **Recommendation**:
     - Immediate feedback on drag
     - Clear error with supported types
     - Visual indicator for valid vs invalid

8. **Duplicate Detection**

   - **Issue**: Same file can be uploaded twice
   - **Impact**: Duplicate processing, wasted resources
   - **Recommendation**:
     - Hash-based duplicate check
     - "Already uploaded" warning
     - Option to reprocess anyway

9. **Cancel Upload**

   - **Issue**: No way to cancel in-progress upload
   - **Impact**: Stuck if wrong file selected
   - **Recommendation**:
     - Cancel button during upload
     - Cancel processing (harder but useful)
     - Confirm before closing page with pending uploads

10. **Batch Card Auto-Dismiss**
    - **Issue**: Card disappears after ~1 second of completion
    - **Impact**: Users may miss completion notification
    - **Recommendation**:
      - Longer display (3-5 seconds)
      - Or manual dismiss option
      - Show completion summary in toast

### Low Priority

11. **Drag Overlay**

    - **Issue**: No full-screen overlay when dragging
    - **Impact**: Users may not see upload zone
    - **Recommendation**:
      - Full-screen drop overlay
      - "Drop files anywhere" message
      - Visual feedback during drag

12. **Upload History**
    - **Issue**: No log of recent uploads
    - **Impact**: Can't verify what was uploaded when
    - **Recommendation**:
      - Show upload timestamp in document table
      - Filter by "recently uploaded"

---

## Recommendations

### Short Term (Sprint 1)

- [ ] Add max file size indication
- [ ] Show per-file upload progress
- [ ] Improve error messages for failed uploads
- [ ] Fix click target issues

### Medium Term (Sprint 2)

- [ ] Add cancel upload functionality
- [ ] Implement duplicate detection
- [ ] Add processing time estimation
- [ ] Improve batch progress detail

### Long Term

- [ ] Full-screen drag overlay
- [ ] Resumable uploads for large files
- [ ] Queue management for many files
- [ ] Upload from URL

---

## Wireframe: Improved Upload Zone

```
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                                                     │   │
│  │                    ⬆                                │   │
│  │                                                     │   │
│  │         Drag & drop files or click to upload        │   │
│  │                                                     │   │
│  │         Supports: .txt, .md, .json                  │   │
│  │         Maximum size: 10 MB per file                │   │
│  │                                                     │   │
│  │   ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─   │   │
│  │                                                     │   │
│  │         [  Browse Files  ]   [  From URL  ]         │   │
│  │                                                     │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## Wireframe: Detailed Progress Card

```
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│  📤 Uploading 3 Files                              [×]     │
│  ─────────────────────────────────────────────────────────  │
│                                                             │
│  ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓░░░░░░░░░░  67%   ~1 min left        │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ ✓ report.md              Completed    12 entities   │  │
│  │ ⟳ analysis.txt           Processing...              │  │
│  │ ⏳ data.json              Queued                     │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
│  Current step: Extracting entities from analysis.txt       │
│                                                             │
│  [Cancel All]                              [Hide Progress]  │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## Wireframe: Error State

```
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│  ⚠️ Upload Errors                                   [×]    │
│  ─────────────────────────────────────────────────────────  │
│                                                             │
│  2 of 3 files uploaded successfully                         │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ ✓ report.md              Completed    12 entities   │  │
│  │ ✓ analysis.txt           Completed    8 entities    │  │
│  │ ✗ large_file.md          Failed: File too large     │  │
│  │                           (15 MB > 10 MB limit)     │  │
│  │                          [Retry with smaller file]  │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
│  [Dismiss]                            [Retry All Failed]    │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## Upload State Machine

```
States:
  IDLE → DRAGGING → UPLOADING → PROCESSING → COMPLETE
                       ↓            ↓
                     ERROR        ERROR

Transitions:
  IDLE + drag enter → DRAGGING
  DRAGGING + drag leave → IDLE
  DRAGGING + drop → UPLOADING
  IDLE + click → file dialog → UPLOADING
  UPLOADING + upload complete → PROCESSING
  UPLOADING + upload error → ERROR
  PROCESSING + processing complete → COMPLETE
  PROCESSING + processing error → ERROR
  ERROR + retry → UPLOADING
  COMPLETE + auto-dismiss → IDLE
```

---

## Acceptance Criteria

- [ ] Max file size shown in upload zone
- [ ] Per-file progress visible
- [ ] Cancel upload button works
- [ ] Error messages are specific and actionable
- [ ] Duplicate files are detected
- [ ] Processing time estimate shown
- [ ] Batch card shows per-file status
- [ ] Click target works reliably
