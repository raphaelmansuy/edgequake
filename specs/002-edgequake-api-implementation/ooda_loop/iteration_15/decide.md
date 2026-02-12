# OODA Iteration 15 — Decide

## Action Plan for Iterations 15-24

### Iteration 15 (this): Foundation
- [x] Audit full API surface from routes.rs
- [x] Document chat API mismatch
- [x] Amend mission file
- [x] Create .gitignore for Go, Java, Kotlin (already done)
- [ ] Create .gitignore for Python, Ruby, Rust, Swift
- [ ] Create OODA files

### Iteration 16: Fix Python SDK
- Fix chat types: `complete(message: str)` instead of `complete(messages: list)`
- Fix chat response type to match actual API
- Add conversation/folder E2E tests with default tenant/user
- Remove skip in TestChat

### Iteration 17: Fix Go SDK
- Fix chat types: Message string field instead of Messages array
- Fix chat response type
- Ensure conversation/folder tests pass with default tenant/user

### Iteration 18: Fix Rust SDK
- Add user_id to TenantContext and ClientBuilder
- Fix chat types to match actual API
- Add conversation/folder E2E tests

### Iteration 19: Fix TypeScript SDK
- Fix chat request/response types
- Add default tenant/user to E2E helpers
- Remove all 14 skips

### Iteration 20: Fix Java + Kotlin SDKs
- Fix chat request/response models
- Add tenant/user defaults for E2E
- Remove conversation/folder skips

### Iteration 21: Fix PHP + Ruby SDKs
- Add conversation/folder E2E tests (remove hardcoded skips)
- Verify chat format is correct

### Iteration 22: Fix Swift + C# SDKs
- Add conversation/folder E2E tests
- Verify chat and remove skips

### Iteration 23: E2E run all SDKs
- Run every SDK E2E against live backend
- Fix any failures
- Verify 0 skips

### Iteration 24: Final verification + commit
- Run 3x verification  
- Commit all changes
