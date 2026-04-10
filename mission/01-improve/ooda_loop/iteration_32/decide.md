# OODA-32 — Decide

Add WHY comment + 10 tests to `edgequake-auth/src/types.rs`:
- Role: parse unknown default, Display roundtrip, Default
- User::new: is_active, last_login_at, metadata empty
- UserInfo::from: maps correctly
- ApiKey::is_expired: None, past, future
- ApiKey::has_scope: empty scopes, wildcard, match, no-match (expand existing)
