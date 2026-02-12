# Iteration 18: Go + Rust SDK Fix

## Observe
- Go SDK conversations list mock used raw array `[]ConversationInfo{}` instead of `{"items":[...]}` wrapper
- Rust SDK had 54 unit + 17 E2E tests already passing after previous fixes

## Orient
- Go coverage test needed mock response format fix for conversations list
- Both SDKs already had correct chat types from previous iteration

## Decide
- Fix Go `edgequake_coverage_test.go` conversations list mock
- Verify both SDKs pass all E2E tests with 0 skips

## Act
- Fixed `sdks/go/edgequake_coverage_test.go`: Changed `TestConversations_List` mock from raw array to `struct{Items []ConversationInfo}{Items: ...}`
- Go: all E2E passed, 0 skipped ✅
- Rust: 54 unit + 17 E2E passed, 0 skipped ✅
