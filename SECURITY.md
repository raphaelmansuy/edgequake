# Security Policy

## Reporting a Vulnerability

Report privately via GitHub Security Advisories or email the maintainers listed in the repository. Do not disclose until a patch is available.

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.32.x (latest pin) | yes |
| older | security fixes only on request |

## Hardening (SPEC-163 / SPEC-154 / SPEC-158)

Production servers must set:

- `JWT_SECRET` (≥32 random bytes; never the documented default)
- `EDGEQUAKE_DEV_MODE=false`
- `EDGEQUAKE_AUTH_ENABLED=true` (this wins over a leftover `DEV_MODE=true`)
- `EDGEQUAKE_CORS_ORIGINS` explicit allow-list
- `ALLOW_REGISTRATION=false`
- `EDGEQUAKE_STRICT_STARTUP=1`
- `EDGEQUAKE_SECRETS_KEY` before storing provider connection keys
- bind `EDGEQUAKE_HOST=127.0.0.1` unless a reverse proxy terminates TLS

Quickstart compose publishes **127.0.0.1** only. Do not use it as a public deployment.

`edgequake doctor` prints posture. `/health` includes `security_posture`.

Details: `docs/providers/security.md`, `docs/operations/runtime-auth-hardening.md`, `docs/security/`.
