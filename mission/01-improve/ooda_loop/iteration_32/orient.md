# OODA-32 — Orient

## First Principles
- `Role::parse` silently defaulting is a security-sensitive behavior — MUST be tested
- `ApiKey::is_expired` with no expiry returning false is a critical invariant
- `has_scope` with empty scopes = allow all is a deliberate choice for backward compat

## Plan
1. Add WHY comment on `Role::parse` default behavior
2. Add tests for Role::parse, Display, Default
3. Add tests for User::new defaults, UserInfo::from
4. Add tests for ApiKey::is_expired (none, past, future)
5. Commit as OODA-32

**Expected: ~10 new tests**
