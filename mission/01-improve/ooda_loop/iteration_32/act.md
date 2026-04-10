# OODA-32 — Act

## Changes Made

### 1. WHY comment: `types.rs` Role::parse (line 30)
- Explains why unknown role strings default to User (least privilege)

### 2. Tests added to `types.rs` (+9 tests → 14 total)
- `test_role_default_is_user` — Default derive
- `test_role_display_roundtrip` — Display → parse for all 3 roles
- `test_role_fromstr_error_message` — Error message format
- `test_user_new_defaults` — is_active, last_login_at, metadata
- `test_user_info_from_user` — From trait mapping
- `test_api_key_not_expired_when_no_expiry` — None = never expires
- `test_api_key_expired_in_past` — past expiry
- `test_api_key_not_expired_in_future` — future expiry
- `test_api_key_empty_scopes_allows_all` — backward-compat allow-all

## Test Evidence

- **edgequake-auth**: 43 passed
- **Workspace total**: 1361 passed, 0 failed
