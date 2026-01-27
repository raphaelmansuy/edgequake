# Iteration 34 - Observe

**Date:** 2026-01-08  
**Focus:** documents.rs analysis for modularization

## Current State

### File Statistics

| Metric            | Value                           |
| ----------------- | ------------------------------- |
| Total lines       | 2,903                           |
| Handler functions | 12                              |
| Helper functions  | 1                               |
| DTOs              | Extracted to documents_types.rs |

### Function Analysis

```
Line    Function                    Lines   Category
-----------------------------------------------------
30      upload_document             374     UPLOAD
404     list_documents              331     LIST
735     get_document                329     GET
1064    delete_document             157     DELETE
1221    analyze_deletion_impact     100     DELETE
1321    upload_file                 413     UPLOAD
1734    upload_files_batch          81      UPLOAD
1815    process_single_file         93      UPLOAD (helper)
1908    get_track_status            182     TRACK
2090    scan_directory              196     DIRECTORY
2286    collect_files               61      DIRECTORY (helper)
2347    reprocess_failed            143     RECOVERY
2490    recover_stuck               413     RECOVERY
-----------------------------------------------------
Total                               2,903
```

### Logical Groupings

| Group         | Functions                                                             | Lines | Purpose                  |
| ------------- | --------------------------------------------------------------------- | ----- | ------------------------ |
| **Upload**    | upload_document, upload_file, upload_files_batch, process_single_file | ~961  | Document upload handling |
| **CRUD**      | list_documents, get_document                                          | ~660  | Basic read operations    |
| **Delete**    | delete_document, analyze_deletion_impact                              | ~257  | Deletion with cascade    |
| **Track**     | get_track_status                                                      | ~182  | Batch tracking           |
| **Directory** | scan_directory, collect_files                                         | ~257  | Directory scanning       |
| **Recovery**  | reprocess_failed, recover_stuck                                       | ~556  | Error recovery           |

## Modularization Candidates

### Option A: By Operation Type (Recommended)

```
documents/
├── mod.rs           # Re-exports
├── upload.rs        # upload_document, upload_file, upload_files_batch, process_single_file
├── crud.rs          # list_documents, get_document
├── delete.rs        # delete_document, analyze_deletion_impact
├── track.rs         # get_track_status
├── directory.rs     # scan_directory, collect_files
└── recovery.rs      # reprocess_failed, recover_stuck
```

### Option B: Keep Single File

- Lower complexity but 2,903 lines still violates SRP

## Risk Assessment

**Medium Risk:**

- Multiple internal dependencies
- Shared state patterns
- Need to maintain backward compatibility

**Considerations:**

- Handler functions need AppState access
- Some helper functions are private
- All share TenantContext pattern
