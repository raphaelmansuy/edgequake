# OODA-32 — Observe

## Target: Auth Types Pure Functions

### Untested Pure Functions in `types.rs`

1. `Role::parse("unknown")` → defaults to User (not tested)
2. `Role::Display` → not tested
3. `Role::default()` → User (derive, untested)
4. `User::new()` — 2 defaults: is_active=true, last_login_at=None
5. `UserInfo::from(&User)` — maps user_id, username, role.to_string()
6. `ApiKey::is_expired()` — None case (never expires) + past/future expiry
7. `ApiKey::has_scope()` — empty scopes (allow all), wildcard "*", specific match, no match

### Untested in `rbac.rs`
8. `Permission::as_str()` — 12 variants, only string conversion tested

### Files needing WHY comments
- `types.rs` — no WHY on Role::parse defaulting behavior
- `rbac.rs` — has WHY comments already
