# SPEC-014 — Edge Cases & Mitigations

## Edge cases handled

1. **No files in multipart**
   - Mitigation: return `400 BadRequest` ("Missing 'file' or 'files' field …").

2. **Mixed valid/invalid files in same batch**
   - Mitigation: per-file result entries include `status=failed` + `error`; batch succeeds with aggregate counters.

3. **Duplicate PDFs inside same batch**
   - Mitigation: first file may be accepted; subsequent identical file returns duplicate status using existing checksum-based logic.

4. **Concurrent duplicate writes**
   - Mitigation: preserved DB-constraint fallback path; re-query checksum and return duplicate response instead of hard 500.

5. **Workspace scoping**
   - Mitigation: endpoint requires and uses `TenantContext` and `workspace_id` for duplicate checks and storage scope.

6. **Vision config defaults**
   - Mitigation: shared single-file logic still resolves workspace vision config and provider/model fallbacks consistently.

## Known limitations (explicit)

- Batch requests currently apply one common option set to all files.
- Batch response can include mixed states (`processing`, `duplicate`, `reindexing`, `failed`), so callers must inspect each item.
