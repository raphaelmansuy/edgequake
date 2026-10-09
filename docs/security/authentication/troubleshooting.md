---
title: Troubleshooting SSO
description: Symptom, cause and fix for the SSO error codes and the usual setup mistakes.
---

Match the symptom, then apply the fix. Denial codes appear in the redirect as `?error=<code>`; the full list is in [Tenants, organizations and roles](tenant-and-roles.md#denial-codes).

| Symptom | Cause | Fix |
|---------|-------|-----|
| `/auth/oidc/callback` returns 503 | SSO not active | `EDGEQUAKE_OIDC_ENABLED=true` + `AUTH_ENABLED=true` |
| `?error=org_unknown` | Tenant for the org alias does not exist | Create a tenant whose slug equals the org alias |
| `?error=org_missing` | Token has no org claim (user not a member / scope missing) | Add the user to the Organization; keep the `organization` scope |
| `?error=org_ambiguous:a,b` | Several orgs, no hint | Pick one (UI picker) or pass `?org=` |
| `?error=account_exists_unlinked` | Same email already local | Sign in with password, or enable verified-email linking deliberately |
| `?error=jit_disabled` | `EDGEQUAKE_OIDC_JIT=false` | Pre-create the user/membership or enable JIT |
| Web UI shows `invalid_handoff` (API: 401, reason `code_invalid`) | The one-time code expired (90 s), was already used, or the tab was restored | Start sign-in again |
| Login loops or `state_expired` | The login host differs from the redirect URI host, so the state cookie is lost | Use the same origin for login start and `REDIRECT_URI` |
| Issuer mismatch at startup or callback | Issuer string differs from discovery `issuer` | Set Keycloak `KC_HOSTNAME`; use identical string |
| Keycloak form shows again after a correct password | Keycloak brute-force protection is on in the shipped realm and locked the user after quick failures | Wait, or unlock the user in the Admin console |
| Realm edits ignored | The import skips realms that exist | Edit in the Admin console, or drop the database volume (dev only) |
| Refresh returns 401 `session_revoked` | Back-channel logout / admin revoke | Sign in again |
| Refresh returns 403 `tenant_suspended` or `membership_revoked` | Tenant/membership changed | Restore access, sign in again |
| Server exits at boot with a pending migration | The schema gate (SPEC-150) refuses to serve; exit code 78 | Run `edgequake migrate`, or set `EDGEQUAKE_SCHEMA_GATE=wait` |

Check the Keycloak side on its own: `python3 scripts/keycloak_smoke.py` (add `--api URL`
to include the EdgeQuake handoff).
